use core::{default, fmt};
use std::cmp::{max, min};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::Instant;

use geo::algorithm::geodesic_distance::GeodesicDistance;
use geo::prelude::EuclideanDistance;
use itertools::Itertools;
use log::{debug, info, trace};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;

use crate::gtfs_data::{GtfsData, GtfsTime, LatLng, Route, Stop, Trip};

#[derive(Debug)]
pub struct RaptorNavigator<'a> {
    start_stop: Stop,
    end_stop: Stop,

    // v[stop_id] -> is within 50 m to destination?
    stops_near_destination_map: HashMap<usize, bool>,
    stops_near_destination_list: Vec<usize>,

    // routes that have trips active today. This speeds up things by skipping rotues.
    routes_active_this_day: HashSet<usize>,

    navigation_params: NavigationParams,

    /// best arrival time at each `stop_id` with `changes`.
    /// [stop_id][changes] -> distance from source in seconds
    t: HashMap<(usize, u8), GtfsTime>,

    /// best absolute time for each `stop`
    /// [stop_id] -> time
    tbest: HashMap<usize, GtfsTime>,

    /// How do I arrive at `stop_id` with `changes` changes?
    /// This is used to reconstruct the solution
    /// [stop_id][changes] -> trip that arrives there
    p: HashMap<(usize, u8), BacktrackingInfo>,

    /// Stops to consider in the next iteration
    marked_stops: Vec<usize>,

    /// Optimizations if we only need the best solution, and not many of them
    only_best: bool,
    best_destination_time: GtfsTime,

    dataset: &'a GtfsData,

}

/// This is used to walk between stops when we change bus
const NEAR_STOP_NUMBER: usize = 10;

#[derive(Debug, Clone, Default)]
pub struct NavigationParams {
    pub from: LatLng,
    pub to: LatLng,
    pub max_changes: u8,
    pub start_time: GtfsTime,
    //pub sol_callback: Box<dyn Fn(Solution)>,
}

#[derive(Debug, Default)]
struct BacktrackingInfo {
    trip_id: Option<usize>,
    route_id: Option<usize>,
    stop_id: usize,
    // start-end index inside trip stops of this piece of solution
    start_stop_inx: usize,
    end_index: usize,
}

impl BacktrackingInfo {
    fn is_walking_path(&self) -> bool {
        self.trip_id.is_none()
    }
}

#[derive(Debug, Default)]
struct Solution {
    start_time: GtfsTime,
    duration_seconds: usize,
    components: Vec<SolutionComponent>,
}

impl fmt::Display for Solution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\n####### Solution components: {}", self.components.len());
        for c in &self.components {
            write!(f, "{}", c);
        }
        write!(f,"####### ")
    }
}


impl Solution {
    fn set_last_component_start(&mut self, stop_id: usize) {
        if let Some(last) = self.components.last_mut() {
            if let SolutionComponent::Walk(w) = last {
                w.stop_id = stop_id
            }
        }
    }
    fn add_walking_path(&mut self, stop_id: usize) {
        let component = WalkSolutionComponent { stop_id };
        self.set_last_component_start(stop_id);
        self.components.push(SolutionComponent::Walk(component));
    }

    fn add_bus_path(&mut self, stop_id: usize, route_id: usize, trip_id: usize, from_inx: usize,
                    to_inx: usize) {
        let component = BusSolutionComponent {
            route_id,
            trip_id,
            from_inx: Some(from_inx),
            to_inx: Some(to_inx),
        };
        self.set_last_component_start(stop_id);
        self.components.push(SolutionComponent::Bus(component));
    }

    fn complete(&mut self) {
        self.components.reverse();
    }
}


#[derive(Debug)]
enum SolutionComponent {
    Walk(WalkSolutionComponent),
    Bus(BusSolutionComponent),
}

impl fmt::Display for SolutionComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolutionComponent::Walk(w) => {
                writeln!(f, "Walk path")
            }
            SolutionComponent::Bus(b) => {
                writeln!(f, "Route {} from {} to {}", b.route_id, b.from_inx.unwrap(), b.to_inx.unwrap())
            }
        }
    }
}

#[derive(Debug, Default)]
struct BusSolutionComponent {
    route_id: usize,
    trip_id: usize,
    /// Within the trip path, `from` and `to` which index
    from_inx: Option<usize>,
    to_inx: Option<usize>,
}

#[derive(Debug, Default)]
struct WalkSolutionComponent {
    stop_id: usize,
}

struct RouteStop {
    route: usize,
    stop: usize,
}


impl<'a> RaptorNavigator<'a> {
    pub fn new(dataset: &GtfsData) -> RaptorNavigator {
        RaptorNavigator {
            dataset,
            start_stop: Default::default(),
            end_stop: Default::default(),
            stops_near_destination_list: Default::default(),
            stops_near_destination_map: Default::default(),
            routes_active_this_day: Default::default(),
            navigation_params: Default::default(),
            t: Default::default(),
            tbest: Default::default(),
            p: Default::default(),
            marked_stops: vec![],
            only_best: false,
            best_destination_time: GtfsTime::new_infinite(),
        }
    }

    pub fn find_path(&mut self, params: NavigationParams) {
        self.init_navigation(params);

        println!("Navigation from {} to {} started", self.start_stop.stop_name, self.end_stop.stop_name);
        self.navigate();
    }

    fn init_navigation(&mut self, params: NavigationParams) {
        self.navigation_params = params.clone();

        self.start_stop = self.dataset.find_nearest_stop(params.from).clone();
        self.end_stop = self.dataset.find_nearest_stop(params.to).clone();
        self.marked_stops.push(self.start_stop.stop_id);
        self.compute_routes_active_today(&params.start_time);
        self.find_near_destination_stops(self.end_stop.stop_pos.clone());
    }

    pub fn find_near_destination_stops(&mut self, pos: LatLng) {
        self.stops_near_destination_list = self.dataset.get_stops_in_range(pos, 50.0);
        self.stops_near_destination_map = self.stops_near_destination_list.iter().map(|&stop| (stop, true)).collect();
    }

    /// Does |max_hop_allowed| passes
    fn navigate(&mut self) {
        let now = Instant::now();
        for hop in 1..self.navigation_params.max_changes {
            println!("---- hop {}", hop);
            // Let's get all the routespassing trougth the stops marked
            let route_stops_to_consider = self.build_route_stop();
            println!("Considering {} route_stops", route_stops_to_consider.len());

            for (route_id, stop_inx) in route_stops_to_consider {
                self.handle_route_stop(route_id, stop_inx, hop);
            }

            self.add_walking_path(hop);
        }
        println!("Navigation finished in: {} ms", now.elapsed().as_millis());
    }

    /// For each marked stop, tries to reach the new possible ones
    fn build_route_stop(&mut self) -> BTreeMap<usize, usize> { // route_id -> stop_id
        println!("Building route stop");
        let to_consider = self.marked_stops.iter()
            .map(|stop_id| self.dataset.get_stop(*stop_id))
            .collect::<Vec<&Stop>>();
        let mut routes_to_consider = BTreeMap::<usize, usize>::new(); // route_id, inx of the stop
        for stop in to_consider {
            let routes = stop.routes.iter().map(|r_id| self.dataset.get_route(*r_id));
            for route in routes {
                let stop_inx = route.get_stop_inx(self.dataset, stop.stop_id);
                let prec_inx = routes_to_consider.entry(route.route_id).or_insert(stop_inx);
                *prec_inx = min(*prec_inx, stop_inx);
            }
        }
        self.marked_stops.clear();
        println!("route stops to consider now: {}", routes_to_consider.len());
        routes_to_consider
    }

    /// take all the routes that pass in this stop
    fn consider_stop(&self, stop: &Stop) {
        println!("considering stop {}", stop.stop_name);
        stop.routes
            .iter().map(|route_id| self.dataset.get_route(*route_id))
            .for_each(|route| {
                self.consider_route(&stop, route);
            });
    }
    /// traverse all the route's trip, and
    fn consider_route(&self, stop: &Stop, route: &Route) {}

    fn handle_route_stop(&mut self, route_id: usize, start_stop_inx: usize, hop_att: u8) { // returns new best solutions
        if !self.routes_active_this_day.contains(&route_id) {
            return;
        }
        let route = self.dataset.get_route(route_id);
        let stop_times = &self.dataset.get_route_stop_times(route_id).stop_times;
        //println!("considering route {} start_stop_inx {}/{}", route.route_long_name, start_stop_inx, stop_times.len());
        let start_stop_id = stop_times[start_stop_inx].stop_id;

        let mut trip: Option<usize> = None;
        let mut _att_stop_inx = start_stop_inx;
        while _att_stop_inx < stop_times.len() {
            let att_stop_inx = _att_stop_inx;
            _att_stop_inx += 1;
            let stop_time = &stop_times[att_stop_inx];
            let stop_id = stop_time.stop_id;
            let curr_stop = self.dataset.get_stop(stop_id);

            let mut time_at_stop = GtfsTime::new_infinite();

            if let Some(trip_id) = trip {
                let prec_best = self.tbest.entry(stop_id).or_insert(GtfsTime::new_infinite());
                time_at_stop = GtfsTime::new_from_midnight(stop_time.time);
                time_at_stop.set_day_from(&self.navigation_params.start_time);

                if self.only_best && self.best_destination_time < time_at_stop {
                    // this solution can't improve the best one
                } else if time_at_stop < *prec_best {
                    // We improve this stop solution with this trip
                    self.update_best(curr_stop.stop_id, hop_att, time_at_stop.clone());

                    // let's save the parent for backtracking
                    self.p.insert((stop_id, hop_att), BacktrackingInfo {
                        trip_id: Some(trip_id),
                        route_id: Some(route_id),
                        stop_id: start_stop_id,
                        start_stop_inx,
                        end_index: att_stop_inx,
                    });

                    self.marked_stops.push(curr_stop.stop_id);
                }
            }

            let prec_time_in_stop = self.t.entry((curr_stop.stop_id, hop_att - 1)).or_insert(GtfsTime::new_infinite());

          //  println!("prec time in stop {:?}, att_time_at_stop {:?}", prec_time_in_stop, time_at_stop);
            // we either don't have a trip already, or with other solutions we arrive there earlier
            if trip.is_none() || *prec_time_in_stop <= time_at_stop {
                let new_trip: Option<(&Trip, usize)> = self.dataset.trip_after_time(
                    route_id, stop_id, prec_time_in_stop, att_stop_inx, HashSet::new()); // todo use excluded trips
                if new_trip.is_none() {
                    continue;
                }

                let (new_trip, trip_stop_inx) = new_trip.unwrap();
                let trip_stop_inx = max(att_stop_inx, trip_stop_inx);
                let arriving_time_with_new_trip = new_trip.start_time + stop_times[trip_stop_inx].time;
                let arriving_time_with_old_trip = if trip.is_none() { i64::MAX } else {
                    self.dataset.get_trip(trip.unwrap()).start_time + stop_times[att_stop_inx].time // this is a bit wrong, we should consider the index of the old trip
                };
                // pruning if we are only looking for the best
                if self.only_best && self.best_destination_time.since_midnight() < (new_trip.start_time + stop_times[trip_stop_inx].time) as u32 {
                    continue;
                }

                if trip.is_none() || arriving_time_with_new_trip < arriving_time_with_old_trip {
                    trip = Some(new_trip.trip_id);
                    _att_stop_inx = trip_stop_inx;
                } else if new_trip.trip_id == trip.unwrap() {
                    // todo use deley before takin gin optimization
                }
            }
        }

        if !self.only_best { self.check_for_solution(hop_att); }

        // todo
    }

    fn update_best(&mut self, stop_id: usize, kth: u8, new_time: GtfsTime) {
        self.t.insert((stop_id, kth), new_time.clone());
        let prec_absolute_best = self.tbest.entry(stop_id).or_insert(GtfsTime::new_from_timestamp(i64::MAX));
        if new_time < *prec_absolute_best {
            *prec_absolute_best = new_time;
        }
    }

    /// Let's add the near stop to each one marked
    fn add_walking_path(&mut self, hop_att: u8) {
        // TODO do benchmark  this, it might be very, very expensive!
        println!("Adding walking paths. Initial number of marked stops: {}", self.marked_stops.len());
        let now = Instant::now();
        let original_best_times = self.tbest.clone();

        for from_stop_id in self.marked_stops.clone() {
            let from_stop = self.dataset.get_stop(from_stop_id);
            let near_stops = self.dataset.get_near_stops(&from_stop.stop_pos, NEAR_STOP_NUMBER);
            for to_stop_id in near_stops {
                let to_stop = self.dataset.get_stop(to_stop_id);
                let cost = self.seconds_by_walk(&from_stop.stop_pos, &to_stop.stop_pos);

                let mut after_walk_time = original_best_times.get(&from_stop.stop_id).unwrap().clone();
                after_walk_time.add_seconds(cost);

                if self.only_best && self.best_destination_time < after_walk_time {
                    continue;
                }
                let old_near_stop_time = self.t.entry((to_stop_id, hop_att)).or_insert_with(GtfsTime::new_infinite);
                if after_walk_time < *old_near_stop_time {
                    self.update_best(to_stop_id, hop_att, after_walk_time);
                    self.p.insert((to_stop_id, hop_att), BacktrackingInfo {
                        trip_id: None,
                        route_id: None,
                        stop_id: from_stop.stop_id,
                        start_stop_inx: 0,
                        end_index: 0,
                    });
                    self.marked_stops.push(to_stop_id);
                }
            }
        };
        self.marked_stops = self.marked_stops.clone().into_iter().unique().collect();

        println!("Time to process walking paths: {} ms. Final number of marked stops: {}", now.elapsed().as_millis(), self.marked_stops.len());
    }
    fn seconds_by_walk(&self, from: &LatLng, to: &LatLng) -> u64 {
        let walking_speed_kmH = 4;
        let meters_per_minute = walking_speed_kmH * 100 / 6;

        from.as_point().geodesic_distance(&to.as_point()) as u64 * 60 / meters_per_minute
    }
    fn check_for_solution(&mut self, hop_att: u8) {
        let mut dest_to_clear = vec![];
        for near_stop_dest in self.stops_near_destination_list.clone() {
            let entry = (near_stop_dest, hop_att);
            if self.p.contains_key(&entry) {
                println!("Can arrive at destination from {} with {} hops", near_stop_dest, hop_att);
                self.reconstruct_solution(near_stop_dest, hop_att);
                dest_to_clear.push(entry);
            }
        }
        for entry in dest_to_clear {
            self.p.remove_entry(&entry);
        }
    }

    fn reconstruct_solution(&self, stop_id: usize, hop_att: u8) {
        let mut solution: Solution = Default::default();
        solution.start_time = (&self.navigation_params.start_time).clone();

        let mut att_stop = stop_id;
        let mut att_kth = hop_att;
        let start_stop = self.start_stop.stop_id;

        // we reconstruct the solution from the last component to the first
        while att_stop != start_stop {
           // println!("Att stop: {}", att_stop);
            let entry = (att_stop, att_kth);
            //assert!(self.p.contains_key(&entry));
            let backtrack_info = self.p
                .get(&(att_stop, att_kth))
                .unwrap_or_else(||
                    panic!("Can't find parent information for stop {}({}) (hop {})",
                           self.dataset.get_stop(att_stop).stop_name, att_stop,
                           hop_att));
           // println!("backtraking: {:?}", backtrack_info);
            let prec_stop = backtrack_info.stop_id;

            if backtrack_info.is_walking_path() {
                solution.add_walking_path(att_stop);
                att_stop = prec_stop;
                continue;
            }

            let prec_trip = backtrack_info.trip_id.unwrap();
            let prec_route = backtrack_info.route_id.unwrap();

            solution.add_bus_path(att_stop,
                                  prec_route,
                                  prec_trip,
                                  backtrack_info.start_stop_inx,
                                  backtrack_info.end_index);
            att_stop = prec_stop;
            att_kth -= 1;
        }
        solution.set_last_component_start(self.start_stop.stop_id);
        solution.complete();
        println!("Solution found! {}",solution);
        // TODO send solution with the callback. Maybe use a channel?
    }

    fn compute_routes_active_today(&mut self, _time: &GtfsTime) {
        self.routes_active_this_day.clear();
        // todo: compute routes active today
        for r in &self.dataset.routes {
            self.routes_active_this_day.insert(r.route_id);
        }
    }
}


