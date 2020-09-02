use std::cmp::{max, min};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::time::Instant;

use itertools::Itertools;
use log::{debug, error, info, trace};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::gtfs_data::{GtfsData, GtfsTime, LatLng, RouteId, Stop, StopDistance, StopId, StopIndex, Trip, TripId};
use crate::navigator_models::{NavigationParams, Solution, WalkingPathUpdate};
use crate::navigator_models::SolutionComponent::Bus;

#[derive(Debug)]
pub struct RaptorNavigator<'a> {
    start_stop: Stop,
    end_stop: Stop,

    // v[stop_id] -> is within 50 m to destination?
    stops_near_destination_map: HashSet<StopId>,
    stops_near_destination_list: Vec<StopId>,

    // trips active today. This speeds up things by skipping the many trips that are not useful.
    active_trips: HashMap<RouteId, Vec<TripId>>, // v[route_id] -> trips active in the searched date


    navigation_params: NavigationParams,

    /// best arrival time at each `stop_id` with `changes`.
    /// [stop_id][changes] -> distance from source in seconds
    t: HashMap<(StopId, Round), GtfsTime>,

    /// best absolute time for each `stop`
    /// [stop_id] -> time
    tbest: HashMap<StopId, GtfsTime>,

    /// How do I arrive at `stop_id` with `changes` changes?
    /// This is used to reconstruct the solution
    /// [stop_id][changes] -> trip that arrives there
    p: HashMap<(StopId, Round), BacktrackingInfo>,

    /// Stops to consider in the next iteration
    marked_stops: Vec<StopId>,

    /// Optimizations if we only need the best solution, and not many of them
    only_best: bool,

    /// destination reached during the navigation. It might be the real destination stop, or one
    /// nearby
    best_stop: Option<StopId>,
    /// how many hops do we need to reach the best_stop?
    best_kth: Option<Round>,
    best_destination_time: GtfsTime,
    /// List of trip ids of previous solutions to avoid now (In this way, we can find new solutions
    banned_trip_ids: HashSet<TripId>,

    dataset: &'a GtfsData,

    on_solution_found: Sender<Solution>,

}

/// This algorithm is organized in several rounds, one for each change. We usually use a maximum of
/// 3 changes, that allow to reach pretty much everywhere (using at maximum 4 busses)
type Round = u8;

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
            active_trips: Default::default(),
        }
    }


    fn init_navigation(&mut self, params: &NavigationParams) {
        self.navigation_params = params.clone();

        self.start_stop = self.dataset.find_nearest_stop(&params.from).clone();
        self.end_stop = self.dataset.find_nearest_stop(&params.to).clone();
        self.compute_trips_active_today(&params.start_time);
        self.compute_stops_near_destination(self.end_stop.stop_pos.clone());
    }

    fn clear_partial(&mut self) {
        self.t.clear();
        self.tbest.clear();
        self.p.clear();
        self.marked_stops.clear();
        self.best_stop = None;
        self.best_kth = None;
        self.best_destination_time = GtfsTime::new_infinite();
    }


    pub fn compute_stops_near_destination(&mut self, pos: LatLng) {
        self.stops_near_destination_list = self.dataset.get_stops_in_range(pos, 50.0);
        self.stops_near_destination_map = self.stops_near_destination_list.iter().copied().collect();
    }

    /// does 3 searches, each time removing the trips of the precedent solutions
    pub fn find_path_multiple(&mut self, params: NavigationParams) {
        debug!("Navigation with param {:?} started", params);
        let now = Instant::now();
        self.only_best = true;

        self.init_navigation(&params);

        for ith_navigation in 0..params.num_solutions_to_find {
            trace!("##### Starting {}-th navigation with {} trips banned", ith_navigation, self.banned_trip_ids.len());
            self.clear_partial();
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

    /// This makes a list of routes that pass trough each marked stops.
    /// Those will be scanned later in handle_route_stop
    fn build_route_stop(&mut self) -> BTreeMap<usize, usize> { // route_id -> stop_inx
        trace!("Building route stop");
        let now = Instant::now();
        let ds = &self.dataset;
        let active_trips = &self.active_trips;

        let all_route_stops: Vec<(RouteId, StopIndex)> = self.marked_stops.clone()
            .into_par_iter()
            .map(|stop_id| ds.get_stop(stop_id))
            .flat_map(|stop| stop.routes // We associate the stop index at each route that passes through this stop
                .iter()
                .filter(|r_id| active_trips.contains_key(r_id))
                .map(|r_id| ds.get_route(*r_id))
                .filter_map(|route| {
                    let stop_inx = route.get_stop_inx(ds, stop.stop_id);
                    if stop_inx.is_none() {
                        error!("Route {} doesn't contain stop {}. You should handle multiple stop times per route!", route.route_short_name, stop.stop_name);
                        return None;
                    }
                    Some((route.route_id, stop_inx.unwrap()))
                }).collect::<Vec<(RouteId, StopIndex)>>() // vec<(route_id, stop_inx)>
            ).collect();

        let mut routes_to_consider = BTreeMap::<RouteId, StopIndex>::new();
        for (route_id, stop_inx) in all_route_stops {
            let prec_stop_inx = routes_to_consider.entry(route_id).or_insert_with(|| stop_inx);
            *prec_stop_inx = min(*prec_stop_inx, stop_inx);
        }

        self.marked_stops.clear();
        info!("route stops to consider now: {} in {}", routes_to_consider.len(), now.elapsed().as_millis());
        routes_to_consider
    }


    fn handle_route_stop(&mut self, route_id: usize, start_stop_inx: usize, hop_att: Round) { // returns new best solutions
        let _route = self.dataset.get_route(route_id);
        let stop_times = &self.dataset.get_route_stop_times(route_id).first().unwrap().stop_times;
        //trace!("considering route {} start_stop_inx {}/{}", route.route_long_name, start_stop_inx, stop_times.len());
        let mut start_stop_id = stop_times[start_stop_inx].stop_id;

        // There might be multiple stops in this trip where we can go up. We take the onw that 
        // maximizes the waiting between precedent and next bus change.
        let mut time_delta_change = -1;
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
                let prec_best = self.tbest.entry(stop_id).or_insert_with(GtfsTime::new_infinite).clone();
                time_at_stop = self.new_time(stop_time.real_time(this_trip.start_time));

                if self.only_best && self.best_destination_time < time_at_stop {
                    //  debug!("Can't improve current best");
                    // this solution can't improve the best one
                } else if time_at_stop < prec_best {
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

            let prec_time_in_stop = self.t.entry((curr_stop.stop_id, hop_att - 1)).or_insert_with(GtfsTime::new_infinite).clone();

            //  trace!("prec time in stop {:?}, att_time_at_stop {:?}", prec_time_in_stop, time_at_stop);
            // we either don't have a trip already, or with other solutions we arrive there earlier
            if trip.is_none() || prec_time_in_stop <= time_at_stop {
                // This route doesn't have active trips
                if !self.active_trips.contains_key(&route_id) { continue; }
                let new_trip: Option<(&Trip, usize)> = self.dataset.trip_after_time(
                    self.active_trips.get(&route_id).unwrap(),
                    stop_id,
                    &prec_time_in_stop,
                    att_stop_inx, &self.banned_trip_ids); // todo use excluded trips

                if new_trip.is_none() {
                    //debug!("No trips available");
                    continue;
                }

                let (new_trip, trip_stop_inx) = new_trip.unwrap();
                if self.banned_trip_ids.contains(&new_trip.trip_id) {
                    panic!("Returned a trip in the banned list!!!");
                }
                let trip_stop_inx = max(att_stop_inx, trip_stop_inx);
                let arriving_time_with_new_trip =
                    self.new_time(new_trip.start_time + stop_times[trip_stop_inx].time);
                let arriving_time_with_old_trip = if trip.is_none() { GtfsTime::new_infinite() } else {
                    self.new_time(self.dataset.get_trip(trip.unwrap()).start_time + stop_times[att_stop_inx].time) // this is a bit wrong, we should consider the index of the old trip
                };
                // pruning if we are only looking for the best
                if self.only_best && self.best_destination_time < arriving_time_with_new_trip {
                    continue;
                }
                if trip.is_none() || arriving_time_with_new_trip < arriving_time_with_old_trip {
                    //  debug!("Setting new trip!");
                    trip = Some(new_trip.trip_id);
                    _att_stop_inx = trip_stop_inx;

                    time_delta_change = arriving_time_with_new_trip.distance(&prec_time_in_stop);
                } else if new_trip.trip_id == trip.unwrap() {
                    // This is the same trip, but maybe it is better to take it at this stop?
                    // It all depends on the delta between the precedent arrival at this stop, 
                    // and when this trip passes there
                    let att_delay = arriving_time_with_new_trip.distance(&prec_time_in_stop);
                    if att_delay > time_delta_change {
                        time_delta_change = att_delay;
                        start_stop_id = att_stop_inx;
                    }
                }
            }
        }

        if !self.only_best { self.check_for_solution(hop_att); }
    }

    fn new_time(&self, seconds_since_midnight: i64) -> GtfsTime {
        self.navigation_params.start_time.new_replacing_time(seconds_since_midnight)
    }
    fn update_best(&mut self, stop_id: usize, kth: Round, new_time: GtfsTime) {
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

    fn seconds_by_walk(meters: usize) -> u64 {
        let walk_speed_kmph = 4;
        let walk_speed_meters_per_minute = walk_speed_kmph * 100 / 6; // da km all'ora ci prendiamo metri per minuto (per far prima
        (meters * 60 / walk_speed_meters_per_minute) as u64
    }
    /// Let's add the near stop to each one marked
    fn add_walking_path(&mut self, hop_att: Round) {
        trace!("Adding walking paths. Initial number of marked stops: {}", self.marked_stops.len());
        let now = Instant::now();
        let original_best_times = self.tbest.clone();
        let ds = &self.dataset;

        let updates_to_do = self.marked_stops
            .clone()
            .into_par_iter()
            .flat_map(|from_stop_id| {
                let from_stop = ds.get_stop(from_stop_id);
                let near_walkable_stops = ds.get_near_stops_by_walk(from_stop.stop_id);
                let near_stops_with_distance: Vec<StopDistance> =
                    if near_walkable_stops.near_stops.is_empty() {
                        // it should almost never enter here!
                        error!("No near stops for {}", from_stop.stop_name);
                        ds.get_near_stops(&from_stop.stop_pos, NEAR_STOP_NUMBER)
                            .iter().map(|&s| ds.get_stop(s))
                            .map(|to| StopDistance {
                                stop_id: to.stop_id,
                                distance_meters: from_stop.stop_pos.distance_meters(&to.stop_pos) as usize,
                            }).collect()
                    } else {
                        near_walkable_stops.near_stops.clone()
                    };

                near_stops_with_distance.into_iter().map(|sd| {
                    let to_stop_id = sd.stop_id;
                    let cost = RaptorNavigator::seconds_by_walk(sd.distance_meters);

                    WalkingPathUpdate {
                        from_stop_id: from_stop.stop_id,
                        to_stop_id,
                        cost,
                    }
                }).collect_vec()
            }).collect::<Vec<WalkingPathUpdate>>();

        for update in updates_to_do {
            let WalkingPathUpdate { from_stop_id, to_stop_id, cost } = update;

            let mut time_after_walking = original_best_times.get(&from_stop_id).unwrap_or(&GtfsTime::new_infinite()).clone();
            time_after_walking.add_seconds(cost);

            // pruning: if this can't improve the best solution, just continue
            if self.only_best && self.best_destination_time < time_after_walking { continue; }

            let old_dest_stop_time = self.t.entry((to_stop_id, hop_att)).or_insert_with(GtfsTime::new_infinite);
            if time_after_walking < *old_dest_stop_time {
                self.update_best(to_stop_id, hop_att, time_after_walking);
                self.p.insert((to_stop_id, hop_att), BacktrackingInfo {
                    trip_id: None,
                    route_id: None,
                    stop_id: from_stop_id,
                    start_stop_inx: 0,
                    end_index: 0,
                });
                self.marked_stops.push(to_stop_id);
            }
        }
        self.marked_stops = self.marked_stops.clone().into_iter().unique().collect();

        trace!("Time to process walking paths: {} ms. Final number of marked stops: {}", now.elapsed().as_millis(), self.marked_stops.len());
    }

    fn check_for_solution(&mut self, hop_att: Round) {
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

    fn reconstruct_solution(&self, stop_id: usize, hop_att: Round) -> Solution {
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
            let path = self.dataset.get_stop_times(prec_trip.stop_times_id);

            solution.add_bus_path(att_stop,
                                  prec_route,
                                  prec_trip,
                                  path,
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
                debug!("Banning trip id: {}", bus_component.trip.trip_id);
                self.banned_trip_ids.insert(bus_component.trip.trip_id);
            }
        }
    }
    fn compute_trips_active_today(&mut self, time: &GtfsTime) {
        let now = Instant::now();
        let &ds = &self.dataset;
        self.active_trips = self.dataset.routes
            .par_iter()
            .map(|r| (r.route_id, ds.trips_active_on_date_within_hours(r.route_id, &time, 4)))
            .filter(|(_, v)| !v.is_empty())
            .collect();

        debug!("routes active today: {}/{}. calculated in: {}", self.active_trips.len(), self.dataset.routes.len(), now.elapsed().as_millis());
    }
}


