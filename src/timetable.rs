use crate::gtfs_data::{GtfsData, GtfsTime, RouteId, StopId, TripId};
use itertools::Itertools;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Default, Debug, Hash, Ord, PartialOrd, Eq, PartialEq, Clone)]
struct Node {
    stop_id: StopId,
    /// This means that `stop_id` is the ith in the trip path
    ith: u8,
}

#[derive(Default)]
pub struct TimeTable {
    pub routes: Vec<RouteId>,
    trips: BTreeSet<TripId>,
    /// after topological sort
    pub stops: Vec<StopId>,
    pub direction: i8,
}

impl TimeTable {
    pub fn new(ds: &GtfsData, routes: Vec<RouteId>, direction: i8) -> Result<TimeTable, String> {
        let trips: BTreeSet<TripId> = routes
            .iter()
            .map(|&route_id| ds.get_route(route_id))
            .flat_map(|r| {
                r.trips
                    .iter()
                    .filter(|&&t| ds.get_trip(t).direction_id.parse::<i8>().unwrap() == direction)
                    .copied()
                    .collect_vec()
            })
            .collect();

        if trips.is_empty() {
            return Err("No trip matches the description. Try to change directions"
                .parse()
                .unwrap());
        }
        let mut result = TimeTable {
            routes,
            trips,
            direction,
            ..Default::default()
        };
        let trips = result
            .trips
            .iter()
            .map(|&t| t as usize)
            .collect::<Vec<usize>>();
        result.stops = TimeTableBuilder::new().sort_stops_topologically(ds, &trips[..]);

        Ok(result)
    }

    pub fn get_column(&self, ds: &GtfsData, trip_id: TripId) -> Vec<usize> {
        assert!(self.trips.contains(&trip_id));
        let trip = ds.get_trip(trip_id);
        let stop_times = &ds.get_stop_times(trip.stop_times_id).stop_times;

        let mut past_appearence = HashMap::new();
        let mut results = vec![];
        for target_stop_id in self.stops.clone() {
            let mut times = (*past_appearence.entry(target_stop_id).or_default()) + 1;
            let mut target_time = GtfsTime::new_infinite();
            for st in stop_times.iter() {
                if st.stop_id == target_stop_id {
                    times -= 1;
                    if times == 0 {
                        target_time = GtfsTime::new_from_midnight(st.time + trip.start_time);
                        break;
                    }
                }
            }
            results.push(target_time);
            *past_appearence.entry(target_stop_id).or_insert(0) += 1;
        }
        results
            .iter()
            .map(|time| time.since_midnight() as usize)
            .collect_vec()
    }

    pub fn get_trips_active_on_date(&self, dataset: &GtfsData, date: &GtfsTime) -> Vec<TripId> {
        self.trips
            .iter()
            .filter(|&&t| dataset.trip_id_active_on_time(t, date, Some(24 * 60 * 60)))
            .sorted_by_key(|&&t| dataset.get_trip(t).start_time)
            .copied()
            .collect_vec()
    }
}
#[derive(Default)]
struct TimeTableBuilder {
    /// This is to decide where the topological sort will start.
    start_point_len: HashMap<Node, usize>,
    first_stops: BTreeSet<Node>,
    last_stops: BTreeSet<Node>,
    visited: HashSet<Node>,
    graph: HashMap<Node, BTreeSet<Node>>,
    topo_sorted: Vec<Node>,
}

impl TimeTableBuilder {
    fn new() -> TimeTableBuilder {
        Default::default()
    }
    fn sort_stops_topologically(&mut self, ds: &GtfsData, trips: &[TripId]) -> Vec<StopId> {
        self.build_graph(ds, trips);
        debug_assert!(
            !self.graph.is_empty(),
            "Graph: {:?}, trips:{:?}",
            self.graph,
            trips
        );
        self.topological_sort()
    }
    fn build_graph(&mut self, ds: &GtfsData, trips: &[TripId]) {
        for &trip_id in trips.iter() {
            let trip = ds.get_trip(trip_id);
            let stops = &ds.get_stop_times(trip.stop_times_id).stop_times;
            let first_node = Node {
                stop_id: stops.first().unwrap().stop_id,
                ith: 0,
            };
            self.first_stops.insert(first_node.clone());
            self.update_start_point_len(first_node, stops.len());

            let mut cnt: HashMap<StopId, u8> = HashMap::new();

            for (ith, (prec, succ)) in stops.iter().zip(stops.iter().skip(1)).enumerate() {
                let prec_node = Node {
                    stop_id: prec.stop_id,
                    ith: *cnt.get(&prec.stop_id).unwrap_or_else(|| &0),
                };
                let succ_node = Node {
                    stop_id: succ.stop_id,
                    ith: *cnt.get(&succ.stop_id).unwrap_or_else(|| &0),
                };

                self.inc_count(&mut cnt, prec.stop_id);
                self.add_arc(prec_node.clone(), succ_node.clone());
                self.inc_count(&mut cnt, prec.stop_id);
                self.update_start_point_len(prec_node, stops.len() - ith);
            }
            let last_stop_id = stops.last().unwrap().stop_id;
            let last_node = Node {
                stop_id: last_stop_id,
                ith: *cnt.get(&last_stop_id).unwrap_or_else(|| &0),
            };
            self.last_stops.insert(last_node);
        }
    }

    fn topological_sort(&mut self) -> Vec<StopId> {
        let mut visit_order = self.first_stops.iter().cloned().collect_vec();
        visit_order.sort_by_key(|node| self.start_point_len.get(node).unwrap());
        debug_assert!(!visit_order.is_empty());
        for node in visit_order {
            self.topo_visit(&node);
        }
        self.topo_sorted.reverse();
        debug_assert_ne!(self.topo_sorted.len(), 0);
        self.topo_sorted
            .iter()
            .map(|node| node.stop_id)
            .collect_vec()
    }
    fn topo_visit(&mut self, att_node: &Node) {
        if self.visited.contains(&att_node) {
            return;
        }
        self.visited.insert(att_node.clone());
        let adjs = self.graph.entry(att_node.clone()).or_default().clone();

        for adj in adjs {
            self.topo_visit(&adj);
        }
        self.topo_sorted.push(att_node.clone());
    }

    /*
     * Utility functions
     */
    fn update_start_point_len(&mut self, start_node: Node, candidate_size: usize) {
        let prec = self
            .start_point_len
            .entry(start_node)
            .or_insert_with(|| candidate_size);
        if candidate_size > *prec {
            *prec = candidate_size;
        }
    }
    fn inc_count(&self, cnt: &mut HashMap<StopId, u8>, stop_id: StopId) {
        *cnt.entry(stop_id).or_insert_with(Default::default) += 1;
    }
    fn add_arc(&mut self, from: Node, to: Node) {
        let from = self.graph.entry(from).or_insert_with(Default::default);
        from.insert(to);
    }
}
