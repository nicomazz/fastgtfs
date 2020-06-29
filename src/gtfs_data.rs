use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::format;
use std::io::Error;
use std::rc::Rc;
use std::sync::RwLock;
use std::thread;

use geo::Coordinate;
use itertools::{enumerate, Itertools};
use log::{debug, error, info, trace, warn};
use rayon::prelude::*;

use crate::models::{Route, Shape, Stop, Trip};

#[derive(Debug, Default)]
pub struct GtfsData {
    pub dataset_id: u32,
    // pub calendar: HashMap<String, Calendar>,
    // pub calendar_dates: HashMap<String, Vec<CalendarDate>>,
    pub routes: Vec<Route>,
    pub trips: Vec<Trip>,
    pub shapes: Vec<Shape>,
    pub stops: Vec<Stop>,
    //pub agencies: Vec<Agency>,

    //temporary, while still using string ids
    pub routes_name_to_inx: HashMap<String, usize>,
    pub trip_name_to_inx: HashMap<String, usize>,
    pub shape_name_to_inx: HashMap<String, usize>,
    pub stop_name_to_inx: HashMap<String, usize>,
}

#[derive(Default, Clone, Copy)]
struct InitialInx {
    routes: usize,
    trips: usize,
    shapes: usize,
    stops: usize,
}

impl InitialInx {
    pub fn add_indexes(&mut self, ds: &GtfsData) {
        self.routes += ds.routes.len();
        self.stops += ds.stops.len();
        self.trips += ds.trips.len();
        self.shapes += ds.shapes.len();
    }
}

impl GtfsData {
    pub fn merge_datasets( datasets: &mut Vec<GtfsData>) -> GtfsData {

        GtfsData::zip_datasets_start_indexes(datasets)
            .par_iter_mut()
            .for_each(|(ds, start_inx)| ds.do_postprocessing(start_inx));

        let mut result: GtfsData = Default::default();

        for ds in datasets.iter_mut() {
            result.merge_dataset(ds);
        }

        result
    }

    fn zip_datasets_start_indexes(datasets: &mut Vec<GtfsData>) -> Vec<(&mut GtfsData, InitialInx)>{
        let mut start_indexes = Vec::<InitialInx>::new();
        let mut att_inx = InitialInx {
            routes: 1,
            ..Default::default()
        };
        for ds in datasets.iter() {
            start_indexes.push(att_inx);
            att_inx.add_indexes(&ds);
        }

        datasets
            .iter_mut()
            .zip(start_indexes.iter().copied())
            .collect::<Vec<(&mut GtfsData, InitialInx)>>()
    }
    pub(crate) fn merge_dataset(&mut self, new_ds: &mut GtfsData) -> &GtfsData {
        self.routes.append(new_ds.routes.as_mut());
        self.trips.append(new_ds.trips.as_mut());
        self.shapes.append(new_ds.shapes.as_mut());
        self.stops.append(new_ds.stops.as_mut());
        assert_eq!(new_ds.stops.len(), 0);
        self
    }
    pub fn get_routes(&self) -> &Vec<Route> {
        &self.routes
    }

    fn do_postprocessing(&mut self, initial_inx: &InitialInx) {
        self.normalize_stop_ids(initial_inx.stops);
        self.normalize_routes(initial_inx.routes);
        self.normalize_trips(initial_inx);
        self.assign_stops_to_routes();
        self.build_walk_paths();
        self.preprocess_stops_near_stops();
    }

    fn normalize_stop_ids(&mut self, initial_inx: usize) {
        &self.stops.sort_by(|a, b| a.stop_id.cmp(&b.stop_id));
        for (inx, stop) in enumerate(&mut self.stops) {
            stop.fast_id = (initial_inx + inx) as u64;
            self.stop_name_to_inx
                .insert(stop.stop_id.to_string(), stop.fast_id as usize);
        }
    }
    fn normalize_routes(&mut self, initial_inx: usize) -> HashMap<String, i32> {
        let mut res = HashMap::<String, i32>::new();
        for (inx, r) in enumerate(&mut self.routes) {
            r.fast_id = (initial_inx + inx) as i32;
            self.routes_name_to_inx
                .insert(r.route_id.clone(), r.fast_id as usize);
        }
        res
    }
    fn normalize_trips(&mut self, initial_inx: &InitialInx) {
        for (inx, t) in enumerate(&mut self.trips) {
            let route_id = *self.routes_name_to_inx.get(&t.route_id).unwrap() - initial_inx.routes;
            let route = self.routes.get_mut(route_id);
            match route {
                None => {println!("Route {} not found!",route_id); continue;},
                Some(_) => {},
            }
            let r: &mut Route = route.unwrap();
            t.fast_trip_id = (initial_inx.trips + inx + 1) as i64;
            r.trips.push(t.fast_trip_id);
        }
    }
    fn assign_stops_to_routes(&mut self) {
        &self
            .routes
            .par_iter_mut()
            .for_each(|r| GtfsData::preprocess_route(r));
    }

    fn build_walk_paths(&mut self) {}
    fn preprocess_stops_near_stops(&mut self) {}

    fn preprocess_route(route: &mut Route) {
        if route.trips.is_empty() {
            warn!("Route {} does not have trips", route.fast_id);
            return;
        }
        //let trip: &Trip = &*route.trips.get(route.trips.len() / 2).unwrap();
        //let stop_times = trip.get_stop_times();
    }

    pub fn get_trip(&self, id: usize) -> &Trip {
        &self.trips[id]
    }
}

impl Ord for GtfsData {
    fn cmp(&self, other: &Self) -> Ordering {
        self.trips.len().cmp(&other.trips.len())
    }
}

impl PartialOrd for GtfsData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for GtfsData {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for GtfsData {}
