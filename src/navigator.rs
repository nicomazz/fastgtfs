use std::cmp::{max, min};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::time::Instant;

use geo::algorithm::geodesic_distance::GeodesicDistance;
use itertools::Itertools;
use log::{debug, error, trace};

use crate::gtfs_data::{GtfsData, GtfsTime, LatLng, Stop, StopDistance, Trip};
use crate::navigator_models::{NavigationParams, Solution};
use std::convert::TryInto;
use crate::navigator_models::SolutionComponent::Bus;

#[derive(Debug)]
pub struct RaptorNavigator<'a> {
    start_stop: Stop,
    end_stop: Stop,

    // v[stop_id] -> is within 50 m to destination?
    stops_near_destination_map: HashSet<usize>,
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

    /// destination reached during the navigation. It might be the real destination stop, or one
    /// nearby
    best_stop: Option<usize>,
    /// how many hops do we need to reach the best_stop?
    best_kth: Option<u8>,
    best_destination_time: GtfsTime,
    /// List of trip ids of previous solutions to avoid now (In this way, we can find new solutions
    banned_trip_ids: HashSet<usize>,

    dataset: &'a GtfsData,

    on_solution_found: Sender<Solution>,

}

/// This is used to walk between stops when we change bus
const NEAR_STOP_NUMBER: usize = 30;


#[derive(Debug, Default)]
pub struct BacktrackingInfo {
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


impl<'a> RaptorNavigator<'a> {
    pub fn new(dataset: &GtfsData, on_solution_found: Sender<Solution>) -> RaptorNavigator {
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
            best_stop: None,
            best_kth: None,
            best_destination_time: GtfsTime::new_infinite(),
            on_solution_found,
            banned_trip_ids: Default::default(),
        }
    }


    fn init_navigation(&mut self, params: &NavigationParams) {
        self.navigation_params = params.clone();

        self.start_stop = self.dataset.find_nearest_stop(&params.from).clone();
        self.end_stop = self.dataset.find_nearest_stop(&params.to).clone();
        self.compute_routes_active_today(&params.start_time);
        self.find_near_destination_stops(self.end_stop.stop_pos.clone());
    }

    fn clear_partial(&mut self) {
        self.t.clear();
        self.tbest.clear();
        self.p.clear();
        self.marked_stops.clear();
        self.routes_active_this_day.clear();
        self.stops_near_destination_list.clear(); // TODO: should those be cleaned?
        self.stops_near_destination_map.clear();
        self.best_stop = None;
        self.best_kth = None;
        self.best_destination_time = GtfsTime::new_infinite();
    }

    fn clear_all(&mut self) {
        self.clear_partial();
    }
    pub fn find_near_destination_stops(&mut self, pos: LatLng) {
        self.stops_near_destination_list = self.dataset.get_stops_in_range(pos, 50.0);
        self.stops_near_destination_map = self.stops_near_destination_list.iter().copied().collect();
    }

    /// does 3 searches, each time removing the trips of the precedent solutions
    pub fn find_path_multiple(&mut self, params: NavigationParams) {
        debug!("Navigation with param {:?} started", params);
        let now = Instant::now();
        self.only_best = true;

        for ith_navigation in 0..4 {
            trace!("##### Starting {}-th navigation", ith_navigation);
            self.clear_all();
            self.init_navigation(&params);
            self.navigate();
            match self.best_stop {
                Some(best_stop) => {
                    let sol = self.reconstruct_solution(best_stop, self.best_kth.unwrap());
                    self.add_trips_to_banned(&sol);
                }
                None => {
                    error!("No solution found at {}-th navigagtion", ith_navigation)
                }
            }
        }
        trace!("Navigation finished in: {} ms", now.elapsed().as_millis());
    }

    /// Does |navigation_params.max_changes| passes
    fn navigate(&mut self) {
        self.update_best(self.start_stop.stop_id, 0, self.navigation_params.start_time.clone());
        self.marked_stops.push(self.start_stop.stop_id);
        self.add_walking_path(0);

        for hop in 1..self.navigation_params.max_changes {
            trace!("---- hop {}", hop);
            // Let's get all the routespassing trougth the stops marked
            let route_stops_to_consider = self.build_route_stop();
            trace!("Considering {} route_stops", route_stops_to_consider.len());

            for (route_id, stop_inx) in route_stops_to_consider {
                self.handle_route_stop(route_id, stop_inx, hop);
            }

            self.add_walking_path(hop);
        }
    }

    /// For each marked stop, tries to reach the new possible ones
    fn build_route_stop(&mut self) -> BTreeMap<usize, usize> { // route_id -> stop_id
        trace!("Building route stop");
        let to_consider = self.marked_stops.iter()
            .map(|&stop_id| self.dataset.get_stop(stop_id))
            .collect::<Vec<&Stop>>();
        let mut routes_to_consider = BTreeMap::<usize, usize>::new(); // route_id, inx of the stop
        // TODO: make this concurrent
        for stop in to_consider {
            let routes = stop.routes.iter().map(|r_id| self.dataset.get_route(*r_id));
            for route in routes {
                let stop_inx = route.get_stop_inx(self.dataset, stop.stop_id);
                if stop_inx.is_none() {
                    error!("Route {} doesn't contain stop {}. You should handle multiple stop times per route!", route.route_short_name, stop.stop_name);
                    continue;
                }
                let prec_inx = routes_to_consider.entry(route.route_id).or_insert_with(|| stop_inx.unwrap());
                *prec_inx = min(*prec_inx, stop_inx.unwrap());
            }
        }
        self.marked_stops.clear();
        trace!("route stops to consider now: {}", routes_to_consider.len());
        routes_to_consider
    }
    /*
        /// take all the routes that pass in this stop
        fn consider_stop(&self, stop: &Stop) {
            trace!("considering stop {}", stop.stop_name);
            stop.routes
                .iter().map(|route_id| self.dataset.get_route(*route_id))
                .for_each(|route| {
                    self.consider_route(&stop, route);
                });
        }
        /// traverse all the route's trip, and
        fn consider_route(&self, _stop: &Stop, _route: &Route) {}*/

    fn handle_route_stop(&mut self, route_id: usize, start_stop_inx: usize, hop_att: u8) { // returns new best solutions
        if !self.routes_active_this_day.contains(&route_id) {
            return;
        }
        let _route = self.dataset.get_route(route_id);
        let stop_times = &self.dataset.get_route_stop_times(route_id).first().unwrap().stop_times;
        //trace!("considering route {} start_stop_inx {}/{}", route.route_long_name, start_stop_inx, stop_times.len());
        let start_stop_id = stop_times[start_stop_inx].stop_id;
        //debug!("Considering route  {} {}", &_route.route_short_name, &_route.route_long_name);
        let mut trip: Option<usize> = None;
        let mut _att_stop_inx = start_stop_inx;
        while _att_stop_inx < stop_times.len() {
            let att_stop_inx = _att_stop_inx;
            _att_stop_inx += 1;
            let stop_time = &stop_times[att_stop_inx];
            let stop_id = stop_time.stop_id;
            let curr_stop = self.dataset.get_stop(stop_id);
            //debug!("Now at stop with inx: {}, name: {}", att_stop_inx, curr_stop.stop_name);

            let mut time_at_stop = GtfsTime::new_infinite();

            if let Some(trip_id) = trip {
                let this_trip = self.dataset.get_trip(trip_id);
                let prec_best = self.tbest.entry(stop_id).or_insert_with(GtfsTime::new_infinite);
                time_at_stop = GtfsTime::base_day_add_from_midnight(&self.navigation_params.start_time, stop_time.real_time(this_trip.start_time));

                if self.only_best && self.best_destination_time < time_at_stop {
                  //  debug!("Can't improve current best");
                    // this solution can't improve the best one
                } else if time_at_stop < *prec_best {
                    // We improve this stop solution with this trip
                    self.update_best(curr_stop.stop_id, hop_att, time_at_stop.clone());
                    // let's save the parent for backtracking
                    self.p.insert((curr_stop.stop_id, hop_att), BacktrackingInfo {
                        trip_id: Some(trip_id),
                        route_id: Some(route_id),
                        stop_id: start_stop_id,
                        start_stop_inx,
                        end_index: att_stop_inx,
                    });
                   // debug!("Marking a new stop!");
                    self.marked_stops.push(curr_stop.stop_id);
                } else {
                    // debug!("This doesn't improve anything. Time at stop: {}, prec_best: {}", time_at_stop, *prec_best);
                }
            }

            let prec_time_in_stop = self.t.entry((curr_stop.stop_id, hop_att - 1)).or_insert_with(GtfsTime::new_infinite);

            //  trace!("prec time in stop {:?}, att_time_at_stop {:?}", prec_time_in_stop, time_at_stop);
            // we either don't have a trip already, or with other solutions we arrive there earlier
            if trip.is_none() || *prec_time_in_stop <= time_at_stop {
                let new_trip: Option<(&Trip, usize)> = self.dataset.trip_after_time(
                    route_id, stop_id, prec_time_in_stop, att_stop_inx, &self.banned_trip_ids); // todo use excluded trips

                if new_trip.is_none() {
                    //debug!("No trips available");
                    continue;
                }

                let (new_trip, trip_stop_inx) = new_trip.unwrap();
                let trip_stop_inx = max(att_stop_inx, trip_stop_inx);
                let arriving_time_with_new_trip = GtfsTime::new_from_midnight(new_trip.start_time + stop_times[trip_stop_inx].time);
                let arriving_time_with_old_trip = if trip.is_none() { GtfsTime::new_infinite() } else {
                    GtfsTime::new_from_midnight(self.dataset.get_trip(trip.unwrap()).start_time + stop_times[att_stop_inx].time) // this is a bit wrong, we should consider the index of the old trip
                };
                // pruning if we are only looking for the best
                if self.only_best && self.best_destination_time < arriving_time_with_new_trip {
                    continue;
                }

                if trip.is_none() || arriving_time_with_new_trip < arriving_time_with_old_trip {
                  //  debug!("Setting new trip!");
                    trip = Some(new_trip.trip_id);
                    _att_stop_inx = trip_stop_inx;
                } else if new_trip.trip_id == trip.unwrap() {
                    // todo use deley before taking in optimization
                }
            }
        }

        if !self.only_best { self.check_for_solution(hop_att); }
    }

    fn update_best(&mut self, stop_id: usize, kth: u8, new_time: GtfsTime) {
        self.t.insert((stop_id, kth), new_time.clone());
        let prec_absolute_best =
            self.tbest
                .entry(stop_id)
                .or_insert_with(GtfsTime::new_infinite);
        if new_time < *prec_absolute_best {
            *prec_absolute_best = new_time.clone();
        }

        if self.stops_near_destination_map.contains(&stop_id)
            && new_time < self.best_destination_time {
            self.best_destination_time = new_time;
            self.best_kth = Some(kth);
            self.best_stop = Some(stop_id);
        }
    }

    fn seconds_by_walk(&self, meters: usize) -> u64 {
        let walk_speed_kmph = 4;
        let walk_speed_meters_per_minute = walk_speed_kmph * 100 / 6; // da km all'ora ci prendiamo metri per minuto (per far prima
        (meters * 60 / walk_speed_meters_per_minute) as u64
    }
    fn lat_lng_dist(&self, from: &LatLng, to: &LatLng) -> u64 {
        from.as_point().geodesic_distance(&to.as_point()) as u64
    }
    /// Let's add the near stop to each one marked
    fn add_walking_path(&mut self, hop_att: u8) {
        // TODO do benchmark  this, it might be very, very expensive!
        trace!("Adding walking paths. Initial number of marked stops: {}", self.marked_stops.len());
        let now = Instant::now();
        let original_best_times = self.tbest.clone();

        for from_stop_id in self.marked_stops.clone() {
            let from_stop = self.dataset.get_stop(from_stop_id);

            let near_walkable_stops = self.dataset.get_near_stops_by_walk(from_stop_id);
            let near_stops_with_distance: Vec<StopDistance> =
                if near_walkable_stops.near_stops.is_empty() {
                    self.dataset
                        .get_near_stops(&from_stop.stop_pos, NEAR_STOP_NUMBER)
                        .iter().map(|&s| self.dataset.get_stop(s))
                        .map(|to| StopDistance {
                            stop_id: to.stop_id,
                            distance_meters: self.lat_lng_dist(&from_stop.stop_pos, &to.stop_pos) as usize,
                        }).collect()
                } else {
                    near_walkable_stops.near_stops.clone()
                };

            for sd in near_stops_with_distance {
                let to_stop_id = sd.stop_id;
                let to_stop = self.dataset.get_stop(to_stop_id);
                let cost = self.seconds_by_walk(sd.distance_meters);
                let mut after_walk_time = original_best_times.get(&from_stop.stop_id).unwrap_or(&GtfsTime::new_infinite()).clone();
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
            if !self.only_best {
                self.check_for_solution(hop_att);
            }
        }
        self.marked_stops = self.marked_stops.clone().into_iter().unique().collect();

        trace!("Time to process walking paths: {} ms. Final number of marked stops: {}", now.elapsed().as_millis(), self.marked_stops.len());
    }

    fn check_for_solution(&mut self, hop_att: u8) {
        let mut dest_to_clear = vec![];
        for &near_stop_dest in &self.stops_near_destination_list {
            let entry = (near_stop_dest, hop_att);
            if self.p.contains_key(&entry) {
                trace!("Can arrive at destination from {} with {} hops", near_stop_dest, hop_att);
                self.reconstruct_solution(near_stop_dest, hop_att);
                dest_to_clear.push(entry);
            }
        }
        for entry in dest_to_clear {
            self.p.remove_entry(&entry);
        }
    }

    fn reconstruct_solution(&self, stop_id: usize, hop_att: u8)-> Solution {
        let mut solution: Solution = Default::default();
        solution.start_time = (&self.navigation_params.start_time).clone();

        let mut att_stop = stop_id;
        let mut att_kth = hop_att;
        let start_stop = self.start_stop.stop_id;

        // we reconstruct the solution from the last component to the first
        while att_stop != start_stop {
            let _entry = (att_stop, att_kth);
            assert!(self.p.contains_key(&_entry));
            let backtrack_info = self.p
                .get(&(att_stop, att_kth))
                .unwrap_or_else(||
                    panic!("Can't find parent information for stop {}({}) (hop {})",
                           self.dataset.get_stop(att_stop).stop_name, att_stop,
                           hop_att));
            let prec_stop = backtrack_info.stop_id;

            if backtrack_info.is_walking_path() {
                solution.add_walking_path(att_stop);
                att_stop = prec_stop;
                continue;
            }

            let prec_trip_id = backtrack_info.trip_id.unwrap();
            let prec_trip = self.dataset.get_trip(prec_trip_id);
            let prec_route_id = backtrack_info.route_id.unwrap();
            let prec_route = self.dataset.get_route(prec_route_id);

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
        self.on_solution_found.send(solution.clone()).unwrap();
        solution
    }

    fn add_trips_to_banned(&mut self, solution: &Solution) {
        for component in &solution.components {
            if let Bus(bus_component) = component {
                self.banned_trip_ids.insert(bus_component.trip.trip_id);
            }
        }
    }
    fn compute_routes_active_today(&mut self, time: &GtfsTime) {
        self.routes_active_this_day = self.dataset.routes
            .iter()
            .map(|r| r.route_id)
            .filter(|&r| self.dataset.route_active_on_day(r, time))
            .collect();
        debug!("routes active today: {}/{}", self.routes_active_this_day.len(), self.dataset.routes.len());
    }
}


