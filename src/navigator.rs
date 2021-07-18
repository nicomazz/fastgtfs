use std::cmp::{max, min};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::iter::FromIterator;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use itertools::Itertools;
use log::{debug, error, info, trace};
#[cfg(not(target_arch = "wasm32"))]
use rayon::iter::ParallelIterator;

use crate::gtfs_data::{
    GtfsData, GtfsTime, LatLng, RouteId, Stop, StopDistance, StopId, StopIndex, StopTimesId, Trip,
    TripId,
};
use crate::navigator_models::SolutionComponent::Bus;
use crate::navigator_models::{NavigationParams, Solution, SolutionComponent, TimeUpdate};
use crate::wasm_aware_rayon_iterators::{
    IntoParallelIteratorIfPossible, ParallelIteratorIfPossible,
};

type SolutionCallback = Arc<Mutex<Sender<Solution>>>;

/// This is an implementation of RAPTOR, from Microsoft research
/// https://www.microsoft.com/en-us/research/wp-content/uploads/2012/01/raptor_alenex.pdf
/// However, even if the idea is from the paper, there are al lot of small changes in the
/// data representation, and optimizations to make the final result more human friendly.
/// By human friendly, I mean that instead of changing 3 busses, with a few seconds for
/// making each change, this algorithm prefers routes with a safer (higher) change time.
///
/// # Example
///  
/// See `tests/navigator.rs` for some example of how to use this.
pub struct RaptorNavigator<'a> {
    on_solution_found: Option<SolutionCallback>, //Sender<Solution>,

    start_stop: Stop,
    end_stop: Stop,

    /// v[stop_id] -> is within X m to destination? (check X in code)
    stops_near_destination_map: HashSet<StopId>,
    stops_near_destination_list: Vec<StopId>,

    /// trips active on the searched date.
    /// This speeds up things by skipping the many trips inactive trips.
    active_trips: HashMap<RouteId, Vec<TripId>>, // v[route_id] -> trips active in the searched date

    /// From, to, departure time, max number of changes.
    navigation_params: NavigationParams,

    /// best arrival time at each `stop_id` with `changes`.
    /// [stop_id][changes] -> distance from start stop in seconds
    t: HashMap<(StopId, Round), GtfsTime>,

    /// best absolute time for each `stop`, in seconds since the start_time
    /// [stop_id] -> time
    tbest: HashMap<StopId, GtfsTime>,

    /// How do I arrive at `stop_id` with `changes` changes?
    /// This is used to reconstruct the solution
    /// [stop_id][changes] -> trip that arrives there, and the stop where I got into this trip.
    p: HashMap<(StopId, Round), BacktrackingInfo>,

    /// Stops to consider in the next iteration
    marked_stops: Vec<StopId>,

    /// Used to optimize things if we only need the best solution.
    only_best: bool,

    /// When this is not None, it is the destination stop (as in `navigation_params`), or a stop nearby.
    best_stop: Option<StopId>,
    /// how many hops do we need to reach `best_stop`?
    best_kth: Option<Round>,
    best_destination_time: GtfsTime,
    /// List of trip ids of previous solutions to avoid now
    /// (In this way, we can find several different solutions).
    banned_trip_ids: HashSet<TripId>,

    dataset: &'a GtfsData,
}

/// This algorithm is organized in several rounds, one for each change. We usually use a maximum of
/// 3 changes, that allow to reach pretty much everywhere (using at maximum 4 busses)
type Round = u8;

/// This is used to walk between stops when we change bus
const NEAR_STOP_NUMBER: usize = 30;

/// All the `Option` fields are `None` for a walk path
#[derive(Debug, Default)]
pub struct BacktrackingInfo {
    trip_id: Option<TripId>,
    route_id: Option<RouteId>,
    /// we departed by this stop id to reach this new stop
    from_stop_id: StopId,
    /// `from_stop_id` was at this index inside the trip. This start-end index inside trip stops of this piece of solution
    from_stop_inx: Option<StopIndex>,
    to_stop_index: Option<StopIndex>,
    distance: Option<u64>, // meters
}

impl BacktrackingInfo {
    fn is_walking_path(&self) -> bool {
        self.trip_id.is_none()
    }

    fn new_walking_info(from_stop_id: StopId, distance: u64) -> BacktrackingInfo {
        BacktrackingInfo {
            trip_id: None,
            route_id: None,
            from_stop_id,
            from_stop_inx: None,
            to_stop_index: None,
            distance: Some(distance),
        }
    }
}

impl<'a> RaptorNavigator<'a> {
    pub fn new(dataset: &GtfsData, on_solution_found: Option<SolutionCallback>) -> RaptorNavigator {
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

    pub fn navigate_blocking(dataset: &GtfsData, params: NavigationParams) -> Vec<Solution> {
        RaptorNavigator::new(&dataset, Option::None).find_path_multiple(params)
    }

    fn init_navigation(&mut self, params: &NavigationParams) {
        let now = instant::Instant::now();
        self.navigation_params = params.clone();

        let start_end_stops = &[&params.from, &params.to]
            .to_vec()
            .into_par_iter_if_possible()
            .map(|pos| self.dataset.find_nearest_stop(pos))
            .collect::<Vec<&Stop>>();
        self.start_stop = start_end_stops[0].clone();
        self.end_stop = start_end_stops[1].clone();

        self.compute_trips_active_today(&params.start_time);
        self.compute_stops_near_destination(self.end_stop.stop_pos.clone());

        info!(
            "Init navigation finished in: {} ms",
            now.elapsed().as_millis()
        );
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

    fn compute_stops_near_destination(&mut self, pos: LatLng) {
        self.stops_near_destination_list = self.dataset.get_stops_in_range(pos, 100.0);
        self.stops_near_destination_map =
            self.stops_near_destination_list.iter().copied().collect();
    }

    fn on_solution_found(&mut self, solution: &Solution) {
        trace!("New solution found!");
        if let Some(callback) = &self.on_solution_found {
            let callback = callback.lock().unwrap();
            callback.send(solution.clone()).unwrap();
        }
    }

    /// does `params.max_changes` searches, each time adding the trips of precedent solutions in `banned_trip_ids`.
    pub fn find_path_multiple(&mut self, params: NavigationParams) -> Vec<Solution> {
        debug!("Navigation with param {:?} started", params);
        let now = instant::Instant::now();
        self.only_best = true;
        let mut solutions = vec![];

        self.init_navigation(&params);

        for ith_navigation in 0..params.num_solutions_to_find {
            trace!(
                "##### Starting {}-th navigation with {} trips banned",
                ith_navigation,
                self.banned_trip_ids.len()
            );
            self.clear_partial();
            self.navigate();
            match self.best_stop {
                Some(best_stop) => {
                    let sol = self.reconstruct_solution(best_stop, self.best_kth.unwrap());
                    self.add_trips_to_banned(&sol);
                    self.on_solution_found(&sol);
                    solutions.push(sol);
                }
                None => error!("No solution found at {}-th navigagtion", ith_navigation),
            }
        }
        info!("Navigation finished in: {} ms", now.elapsed().as_millis());
        solutions
    }

    /// Does `navigation_params.max_changes` passes
    fn navigate(&mut self) {
        self.update_best(
            self.start_stop.stop_id,
            0,
            self.navigation_params.start_time.clone(),
        );
        self.marked_stops.push(self.start_stop.stop_id);
        self.add_walking_path(0);

        for hop_att in 1..=self.navigation_params.max_changes + 1 {
            trace!("---- hop {}", hop_att);
            // Let's get all the routes passing through the stops marked
            let route_stops_to_consider = self.build_route_stop();
            self.marked_stops.clear();
            trace!("Considering {} route_stops", route_stops_to_consider.len());

            let updates = Vec::from_iter(route_stops_to_consider)
                .into_par_iter_if_possible()
                .flat_map(|((route_id, stop_times_id), stop_inx)| {
                    self.handle_routes_passing_in_stop(route_id, stop_times_id, stop_inx, hop_att)
                })
                .collect();
            self.perform_best_updates(updates, hop_att);
            self.add_walking_path(hop_att);
        }
    }

    fn perform_best_updates(&mut self, possible_updates: Vec<TimeUpdate>, hop_att: u8) {
        for update in possible_updates {
            let TimeUpdate {
                to_stop_id,
                destination_time,
                backtrack_info,
            } = update;

            // pruning: if this can't improve the best solution, just continue
            if self.only_best && self.best_destination_time <= destination_time {
                return;
            }

            let old_dest_stop_time = self.stop_time(to_stop_id, hop_att);
            // TODO: This `if` is only a workaround. Fix it? It might be fixed by applying walking path in another hop.
            // we apply walking paths on the same layer of normal paths, and a cycle can be created
            // More than a cycle, it's one walking path from A to B, and another from B to C.
            // C's parent will be B as walk path, but also B's parent is a walking path.
            /*if backtrack_info.is_walking_path() {
                if let Some(parent_backtrack) = self.p.get(&(backtrack_info.from_stop_id, hop_att))
                {
                    if parent_backtrack.is_walking_path() {
                        continue;
                    }
                }
            }*/
            if destination_time < old_dest_stop_time {
                self.update_best(to_stop_id, hop_att, destination_time);
                self.p.insert((to_stop_id, hop_att), backtrack_info);
                self.marked_stops.push(to_stop_id);
            }
        }
        self.marked_stops = self.marked_stops.clone().into_iter().unique().collect();
    }

    /// A route contains several trips. Each of them can have different paths.
    /// For this reason we need to return also the StopTimesId (that represents
    /// the path of one or more trips).
    /// Usually, a route has a low number of different StopTimes (<5), so the
    /// output of this function should have a size almost always < 5
    fn get_all_routes_stop(
        &self,
        route_id: RouteId,
        stop_id: StopId,
    ) -> Vec<(RouteId, StopTimesId, StopIndex)> {
        let stop_times = self.dataset.get_route_stop_times(route_id);

        let res = stop_times
            .iter()
            .filter_map(|stop_time| {
                let stop_inx = stop_time.get_stop_inx(stop_id);
                stop_inx.map(|stop_inx| (route_id, stop_time.stop_times_id, stop_inx))
            })
            .collect_vec();

        if res.is_empty() {
            panic!("There should have been a stop inside this route id that matches. Check where the stops are assigned to routes!");
        }
        res
    }

    fn get_stop_routes_to_process(&self, stop: &Stop) -> Vec<(RouteId, StopTimesId, StopIndex)> {
        stop.routes // We associate the stop index at each route that passes through this stop
            .iter()
            .filter(|r_id| self.active_trips.contains_key(r_id))
            .flat_map(|&r_id| self.get_all_routes_stop(r_id, stop.stop_id))
            .collect::<Vec<(RouteId, StopTimesId, StopIndex)>>()
    }
    /// This makes a list of routes that pass trough each marked stops.
    /// Those will be scanned later in handle_route_stop
    fn build_route_stop(&self) -> BTreeMap<(RouteId, StopTimesId), StopIndex> {
        trace!("Building route stop");
        let now = instant::Instant::now();
        let ds = self.dataset;

        // This means that we later have to consider RouteId, with the path StopTimesId, from StopIndex
        let all_route_stops: Vec<(RouteId, StopTimesId, StopIndex)> = self
            .marked_stops
            .clone()
            .into_par_iter_if_possible()
            .map(|stop_id| ds.get_stop(stop_id))
            .flat_map(|stop| self.get_stop_routes_to_process(stop))
            .collect();

        // Always keep the minimum stop index, for each route. Using something that is not the minimum can't be optimal.
        let mut routes_to_consider = BTreeMap::<(RouteId, StopTimesId), StopIndex>::new();
        for (route_id, stop_times_id, stop_inx) in all_route_stops {
            let prec_stop_inx = routes_to_consider
                .entry((route_id, stop_times_id))
                .or_insert_with(|| stop_inx);
            *prec_stop_inx = min(*prec_stop_inx, stop_inx);
        }

        info!(
            "route stops to consider now: {} in {} ms",
            routes_to_consider.len(),
            now.elapsed().as_millis()
        );
        routes_to_consider
    }

    fn handle_routes_passing_in_stop(
        &self,
        route_id: RouteId,
        stop_times_id: StopTimesId,
        _start_stop_inx: StopIndex,
        hop_att: Round,
    ) -> Vec<TimeUpdate> {
        // This has the path template for all trips. The first time in each StopTime is always zero.
        let stop_times = &self.dataset.get_stop_times(stop_times_id).stop_times;
        let mut start_stop_inx = _start_stop_inx;
        let mut start_stop_id = stop_times[start_stop_inx].stop_id;
        assert_ne!(self.best_stop_time(start_stop_id), GtfsTime::new_infinite());
        // There might be multiple stops in this trip where we can go up. We take the one that
        // maximizes the waiting between precedent and next bus change max(arrival_in_stop - prec_best_time)
        let mut time_delta_change = 0;
        //debug!("Considering route  {} {}", &_route.route_short_name, &_route.route_long_name);
        let mut trip: Option<usize> = None;
        let mut _curr_loop_stop_inx = start_stop_inx;

        let mut updates = vec![];

        // curr_* variables refer to the current trip.
        // prec_* variables to the precedent.
        while _curr_loop_stop_inx < stop_times.len() {
            let curr_stop_inx = _curr_loop_stop_inx;
            _curr_loop_stop_inx += 1;

            let curr_stop_time = &stop_times[curr_stop_inx];
            let curr_stop_id = curr_stop_time.stop_id;
            let curr_stop = self.dataset.get_stop(curr_stop_id);
            // debug!("Now at stop with inx: {}, name: {}", att_stop_inx, curr_stop.stop_name);

            let mut curr_time = GtfsTime::new_infinite();

            if let Some(trip_id) = trip {
                // We already have a trip set.
                let curr_trip = self.dataset.get_trip(trip_id);
                let prec_best = self.best_stop_time(curr_stop_id);

                curr_time = self.new_time(curr_stop_time.offset_with_trip(curr_trip.start_time));

                if self.only_best && self.best_destination_time < curr_time {
                    // this solution can't improve the best one.
                } else if curr_time < prec_best {
                    debug_assert_ne!(start_stop_id, curr_stop_id);
                    updates.push(TimeUpdate {
                        to_stop_id: curr_stop.stop_id,
                        destination_time: curr_time.clone(),
                        backtrack_info: BacktrackingInfo {
                            trip_id: Some(trip_id),
                            route_id: Some(route_id),
                            from_stop_id: start_stop_id,
                            from_stop_inx: Some(start_stop_inx),
                            to_stop_index: Some(curr_stop_inx),
                            distance: None,
                        },
                    });
                } else {
                    // debug!("This doesn't improve anything. Time at stop: {}, prec_best: {}", time_at_stop, *prec_best);
                }
            }

            // This can be infinite when we never reached the stop before.
            let prec_time = self.stop_time(curr_stop.stop_id, hop_att - 1);

            // we either don't have a trip already, or
            // with other solutions we arrive there earlier than with the trip we are now.
            if trip.is_none() || prec_time <= curr_time {
                // This route doesn't have active trips.
                if !self.active_trips.contains_key(&route_id) {
                    continue;
                }
                // Let's get the next trip passing here, respecting all conditions.
                let new_trip: Option<(&Trip, StopIndex)> = self.dataset.trip_after_time(
                    self.active_trips.get(&route_id).unwrap(),
                    curr_stop_id,
                    &prec_time,
                    curr_stop_inx,
                    stop_times_id,
                    &self.banned_trip_ids,
                );

                if new_trip.is_none() {
                    continue;
                }

                let (new_trip, new_trip_stop_inx) = new_trip.unwrap();
                debug_assert!(
                    !self.banned_trip_ids.contains(&new_trip.trip_id),
                    "Returned a trip in the banned list!"
                );

                // let's align to this new trip. If actually get new_trip_stop_inx as maximum,
                // this means that it was for us impossible to get into a bus in curr_stop_inx,
                // this because self.dataset.trip_after_time scans in increasing stop index.
                let trip_stop_inx = max(curr_stop_inx, new_trip_stop_inx);
                let arriving_time_new_trip =
                    self.new_time(new_trip.start_time + stop_times[trip_stop_inx].time);
                let arriving_time_old_trip = match trip {
                    None => GtfsTime::new_infinite(),
                    Some(trip) => self.new_time(
                        self.dataset.get_trip(trip).start_time + stop_times[curr_stop_inx].time,
                    ),
                };

                // pruning if we are only looking for the best
                if self.only_best && self.best_destination_time < arriving_time_new_trip {
                    continue;
                }
                if trip.is_none() || arriving_time_new_trip < arriving_time_old_trip {
                    debug_assert!(
                        prec_time <= arriving_time_new_trip,
                        "Trip starts earlier than arriving in this stop  \n\
                            ->start time {}\n->arriving_time_new_trip {}\n\n",
                        prec_time,
                        arriving_time_new_trip,
                    );

                    // let's get into this new trip
                    trip = Some(new_trip.trip_id);
                    _curr_loop_stop_inx = trip_stop_inx;

                    start_stop_inx = trip_stop_inx;
                    start_stop_id = curr_stop_id;
                    debug_assert_eq!(stop_times[curr_stop_inx].stop_id, start_stop_id);

                    time_delta_change = arriving_time_new_trip.distance(&prec_time);
                } else if new_trip.trip_id == trip.unwrap() {
                    // This is the same trip, but maybe it is better to take it at this stop?
                    // It all depends on the delta between the precedent arrival at this stop,
                    // and when this trip passes there
                    let att_delay = arriving_time_new_trip.distance(&prec_time);
                    if att_delay > time_delta_change {
                        time_delta_change = att_delay;
                        start_stop_inx = curr_stop_inx;
                        start_stop_id = curr_stop_id;
                    }
                }
            }
        }
        updates
    }

    fn new_time(&self, seconds_since_midnight: i64) -> GtfsTime {
        self.navigation_params
            .start_time
            .new_replacing_time(seconds_since_midnight)
    }

    fn update_best(&mut self, stop_id: usize, hop_att: Round, new_time: GtfsTime) {
        // We improve something.
        debug_assert!(self.stop_time(stop_id, hop_att) > new_time);
        self.t.insert((stop_id, hop_att), new_time.clone());

        let prec_absolute_best = self
            .tbest
            .entry(stop_id)
            .or_insert_with(GtfsTime::new_infinite);
        if new_time < *prec_absolute_best {
            *prec_absolute_best = new_time.clone();
        }
        debug_assert!(self.best_stop_time(stop_id) <= new_time);

        if self.stops_near_destination_map.contains(&stop_id)
            && (new_time < self.best_destination_time
                || (new_time == self.best_destination_time
                    && hop_att < self.best_kth.unwrap_or(u8::MAX)))
        {
            self.best_destination_time = new_time;
            self.best_kth = Some(hop_att);
            self.best_stop = Some(stop_id);
        }
    }

    pub fn seconds_by_walk(meters: usize) -> u64 {
        let walk_speed_kmh = 3;
        (meters * 36 / (walk_speed_kmh * 10)) as u64
    }

    fn stop_time(&self, stop_id: StopId, hop: u8) -> GtfsTime {
        self.t
            .get(&(stop_id, hop))
            .unwrap_or(&GtfsTime::new_infinite())
            .clone()
    }

    fn best_stop_time(&self, stop_id: StopId) -> GtfsTime {
        self.tbest
            .get(&stop_id)
            .unwrap_or(&GtfsTime::new_infinite())
            .clone()
    }

    /// Let's add the near stop to each one marked
    fn add_walking_path(&mut self, hop_att: Round) {
        trace!(
            "Adding walking paths. Initial number of marked stops: {}",
            self.marked_stops.len()
        );
        let now = instant::Instant::now();

        let updates_to_do = self.compute_walking_updates();

        self.perform_best_updates(updates_to_do, hop_att);

        trace!(
            "Time to process walking paths: {} ms. Final number of marked stops: {}",
            now.elapsed().as_millis(),
            self.marked_stops.len()
        );
    }

    fn compute_walking_updates(&self) -> Vec<TimeUpdate> {
        self.marked_stops
            .par_iter_if_possible()
            .flat_map(|&from_stop_id| self.compute_walking_updates_for_stop(from_stop_id))
            .collect::<Vec<TimeUpdate>>()
    }

    fn compute_walking_updates_for_stop(&self, from_stop_id: StopId) -> Vec<TimeUpdate> {
        let ds = self.dataset;
        let tbest = &self.tbest;
        let from_stop = ds.get_stop(from_stop_id);
        let from_stop_best_time = tbest.get(&from_stop_id).unwrap();
        // Those are precalculated using a real walking distance navigator.
        let precalculated_near_stops = ds.get_near_stops_by_walk(from_stop.stop_id);

        let optional_near_stops_by_air_distance: Vec<StopDistance> =
            if precalculated_near_stops.near_stops.is_empty() {
                // it should almost never enter here!
                error!("No near stops for {}", from_stop.stop_name);
                self.compute_nearest_stops_by_air_distance(from_stop)
            } else {
                vec![] // in this case, we use the precalculated stops
            };

        let near_stops_with_distance: &Vec<StopDistance> =
            if precalculated_near_stops.near_stops.is_empty() {
                &optional_near_stops_by_air_distance
            } else {
                &precalculated_near_stops.near_stops
            };

        near_stops_with_distance
            .iter()
            .filter(|sd| sd.distance_meters < 10000) //nobody wants to walk for 10 km
            .filter_map(|sd| {
                let to_stop_id = sd.stop_id;
                let cost = RaptorNavigator::seconds_by_walk(sd.distance_meters);
                let destination_time = from_stop_best_time.clone().add_seconds(cost).clone();
                let prec_best_destination_time = tbest
                    .get(&(to_stop_id))
                    .unwrap_or(&GtfsTime::new_infinite())
                    .clone();
                if destination_time < prec_best_destination_time {
                    Some(TimeUpdate {
                        to_stop_id,
                        destination_time,
                        backtrack_info: BacktrackingInfo::new_walking_info(
                            from_stop_id,
                            sd.distance_meters as u64,
                        ),
                    })
                } else {
                    None
                }
            })
            .collect_vec()
    }
    fn compute_nearest_stops_by_air_distance(&self, stop: &Stop) -> Vec<StopDistance> {
        let stop_pos = &stop.stop_pos;
        self.dataset
            .get_near_stops(stop_pos, NEAR_STOP_NUMBER)
            .iter()
            .map(|&s| self.dataset.get_stop(s))
            .map(|to| StopDistance {
                stop_id: to.stop_id,
                distance_meters: stop_pos.distance_meters(&to.stop_pos) as usize,
            })
            .collect()
    }

    fn reconstruct_solution(&self, stop_id: usize, hop_att: Round) -> Solution {
        let mut solution = Solution {
            navigation_start_time: (&self.navigation_params.start_time).clone(),
            ..Default::default()
        };

        let mut att_stop = stop_id;
        let mut att_kth = hop_att;
        let start_stop = self.start_stop.stop_id;

        let mut upper_time = self.t.get(&(stop_id, hop_att)).unwrap().clone();

        // we reconstruct the solution from the last component to the first
        while att_stop != start_stop {
            let _entry = (att_stop, att_kth);
            debug_assert!(self.p.contains_key(&_entry));
            let backtrack_info = self.p.get(&(att_stop, att_kth)).unwrap_or_else(|| {
                panic!(
                    "Can't find parent information for stop {}({}) (hop {})",
                    self.dataset.get_stop(att_stop).stop_name,
                    att_stop,
                    hop_att
                )
            });
            let prec_stop = backtrack_info.from_stop_id;

            if backtrack_info.is_walking_path() {
                solution.add_walking_path(
                    prec_stop,
                    att_stop,
                    backtrack_info.distance.unwrap() as usize,
                );
                att_stop = prec_stop;
                continue;
            }

            let prec_trip_id = backtrack_info.trip_id.unwrap();
            let prec_trip = self.dataset.get_trip(prec_trip_id);
            let prec_route_id = backtrack_info.route_id.unwrap();
            let prec_route = self.dataset.get_route(prec_route_id);
            let path = self.dataset.get_stop_times(prec_trip.stop_times_id);

            // Additional check
            let start_inx = backtrack_info.from_stop_inx.unwrap();
            let time_at_start_inx =
                self.new_time(path.stop_times[start_inx].time + prec_trip.start_time);

            solution.add_bus_path(
                att_stop,
                prec_route,
                prec_trip,
                path,
                backtrack_info.from_stop_inx.unwrap(),
                backtrack_info.to_stop_index.unwrap(),
            );

            debug_assert!(
                upper_time >= time_at_start_inx,
                "upper: {}, start {}, Wrong solution: {}",
                upper_time,
                time_at_start_inx,
                &solution
            );
            upper_time = time_at_start_inx;

            att_stop = prec_stop;
            att_kth -= 1;
        }
        solution.set_last_component_start(self.start_stop.stop_id);
        solution.complete(&self.dataset);

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
        let now = instant::Instant::now();
        let &ds = &self.dataset;
        self.active_trips = self
            .dataset
            .routes
            .par_iter_if_possible()
            .map(|r| {
                (
                    r.route_id,
                    ds.get_trips_active_on_date_within_hours(r.route_id, &time, 5)
                        .into_iter()
                        .sorted_by_key(|&t_id| ds.get_trip(t_id).start_time)
                        .collect::<Vec<TripId>>(),
                )
            })
            .filter(|(_, v)| !v.is_empty())
            .collect();

        debug!(
            "routes active today: {}/{}. calculated in {} ms",
            self.active_trips.len(),
            self.dataset.routes.len(),
            now.elapsed().as_millis()
        );
    }

    // scans all the stops in `t` , and tries to reconstruct the chain up until the start stop for each one
    fn _validate_all_stops(&self, hop_att: u8) {
        let ds = self.dataset;

        ds.stops
            .iter()
            .filter(|stop| self.t.contains_key(&(stop.stop_id, hop_att)))
            .for_each(|stop| {
                let sol = self.reconstruct_solution(stop.stop_id, hop_att);
                RaptorNavigator::validate_solution(&sol, &self.navigation_params.start_time);
            })
    }

    pub fn validate_solution(sol: &Solution, start_time: &GtfsTime) {
        let mut last_time = start_time.clone();
        for component in &sol.components {
            if let SolutionComponent::Bus(b) = component {
                assert!(last_time <= b.departure_time(), "Wrong solution: {}", &sol);
                last_time = b.arrival_time();
            }
        }
    }
}
