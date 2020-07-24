use core::default;
use std::cmp::{max, min};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::Instant;

use itertools::Itertools;
use log::{debug, info, trace};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;

use crate::gtfs_data::{GtfsData, LatLng, Route, Stop, Trip};

#[derive(Debug)]
pub struct RaptorNavigator<'a> {
    start_stop: Stop,
    end_stop: Stop,

    stops_near_destination_map: HashMap<usize, bool>,
    stops_near_destination_list: Vec<usize>,
    // v[stop_id] -> is within 50 m to destination?
    max_hop_allowed: u8,
    start_time: u64,
    routes_active_this_day: HashSet<usize>,

    // [stop_id][changes] -> distance in seconds
    t: HashMap<(usize, u8), i64>,

    // [stop_id][hops] -> time
    tbest: HashMap<usize, i64>,
    // [stop_id][changes] -> trip that arrives there
    p: HashMap<(usize, u8), BacktrackingInfo>,

    marked_stops: Vec<usize>, // stops reached so far

    only_best: bool,
    best_destination_time: i64,

    dataset: &'a GtfsData,

}

#[derive(Debug, Default)]
struct BacktrackingInfo {
    trip_id: usize,
    stop_id: usize,
    // start-end index inside trip stops of this piece of solution
    start_stop_inx: usize,
    end_index: usize,
}

struct Solution {}

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
            max_hop_allowed: 0,
            start_time: 0,
            routes_active_this_day: Default::default(),
            t: Default::default(),
            tbest: Default::default(),
            p: Default::default(),
            marked_stops: vec![],
            only_best: false,
            best_destination_time: 0,
        }
    }
    pub fn find_path_from_coordinates(&mut self, from: LatLng, to: LatLng, max_changes: u8, time: u64) {
        let from_stop = self.dataset.find_nearest_stop(from);
        let to_stop = self.dataset.find_nearest_stop(to);
        self.find_path(from_stop, to_stop, max_changes, time);
    }
    pub fn find_path(&mut self, from: &Stop, to: &Stop, max_changes: u8, time: u64) {
        self.start_stop = from.clone();
        self.end_stop = to.clone();
        self.max_hop_allowed = max_changes;
        self.start_time = time;
        self.compute_routes_active_today(time);
        self.marked_stops.push(from.stop_id);
        self.find_near_destination_stops(to.stop_pos.clone());
        println!("Navigation from {} to {} started", from.stop_name, to.stop_name);

        let now = Instant::now();
        self.navigate();
        println!("Navigation finished in: {} ms", now.elapsed().as_millis());
    }
    pub fn find_near_destination_stops(&mut self, pos: LatLng) {
        self.stops_near_destination_list = self.dataset.get_stops_in_range(pos, 50.0);
        self.stops_near_destination_map = self.stops_near_destination_list.iter().map(|&stop| (stop, true)).collect();
    }
    /// Does |max_hop_allowed| passes
    fn navigate(&mut self) {
        for hop in 1..self.max_hop_allowed {
            println!("---- hop {}", hop);
            // Let's get all the routespassing trougth the stops marked
            let route_stops_to_consider = self.build_route_stop();

            for (route_id, stop_inx) in route_stops_to_consider {
                self.handle_route_stop(route_id, stop_inx, hop);
            }

            self.add_walking_path();
        }
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
        let stop_times = &self.dataset.get_route_stop_times(route_id).stop_times;

        let mut trip: Option<usize> = None;
        let mut _att_stop_inx = start_stop_inx;
        while _att_stop_inx < stop_times.len() {
            let att_stop_inx = _att_stop_inx;
            _att_stop_inx += 1;
            let stop_time = &stop_times[att_stop_inx];
            let stop_id = stop_time.stop_id;
            let curr_stop = self.dataset.get_stop(stop_id);

            let mut time_at_stop = i64::MAX;

            if let Some(trip_id) = trip {
                let prec_best = self.tbest.get(&stop_id).unwrap_or_else(|| &i64::MAX);
                time_at_stop = stop_time.time;
                // todo time_at_stop.get_day_date_from(self.start_time);

                if self.only_best && self.best_destination_time < time_at_stop {
                    // this solution can't improve the best one
                } else if time_at_stop > 0 && time_at_stop < *prec_best {
                    // We improve this stop solution with this trip
                    self.t.insert((curr_stop.stop_id, hop_att), time_at_stop);
                    let prec_absolute_best = self.tbest.entry(stop_id).or_insert(i64::MAX);
                    *prec_absolute_best = min(*prec_absolute_best, time_at_stop);
                    // let's save the parent for backtracking
                    self.p.insert((stop_id, hop_att), BacktrackingInfo {
                        trip_id,
                        stop_id,
                        start_stop_inx,
                        end_index: att_stop_inx,
                    });
                    self.marked_stops.push(curr_stop.stop_id);
                    // todo add near destinatio noptimization
                }
            }

            let prec_time_in_stop = self.t.get(&(curr_stop.stop_id, hop_att - 1)).unwrap_or_else(|| &i64::MAX);

            // we either don't have a trip already, or with other solutions we arrive there earlier
            if trip.is_none() || *prec_time_in_stop <= time_at_stop {
                let new_trip: Option<(&Trip, usize)> = self.dataset.trip_after_time(
                    route_id, stop_id, *prec_time_in_stop, att_stop_inx, HashSet::new()); // todo use excluded trips
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
                if self.only_best && self.best_destination_time < new_trip.start_time + stop_times[trip_stop_inx].time {
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
    fn add_walking_path(&self) {
        println!("add_walking_path to implement!")
    }
    fn check_for_solution(&mut self, hop_att: u8) {
        let mut  dest_to_clear = vec![];
        for near_stop_dest in self.stops_near_destination_list.clone() {
            let entry = (near_stop_dest, hop_att);
            if self.p.contains_key(&entry) {
                println!("Can arrive at destination with {} hops", hop_att);
                self.reconstruct_solution(near_stop_dest, hop_att);
                dest_to_clear.push(entry);
            }

        }
        for entry in dest_to_clear {
            self.p.remove_entry(&entry);
        }
    }

    fn reconstruct_solution(&mut self, stop_id : usize, hop_att: u8){
        println!("--> TODO: reconstruct solution!");
    }
    fn compute_routes_active_today(&mut self, _time: u64) {
        self.routes_active_this_day.clear();
        // todo: compute routes active today
        for r in &self.dataset.routes {
            self.routes_active_this_day.insert(r.route_id);
        }
    }
}


