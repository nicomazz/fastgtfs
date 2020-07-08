use std::collections::hash_map::Iter;
use std::collections::HashMap;
use std::io::Error;
use std::path::Path;

use geo::Coordinate;
use itertools::Itertools;
use rayon::iter::IntoParallelRefIterator;

use crate::gtfs_data::{GtfsData, StopTime, StopTimes};
use crate::models::{Route, Shape, Trip};
use crate::raw_models::{parse_gtfs, parse_gtfs_iter, RawRoute, RawShape, RawStop, RawStopTime, RawTrip};

struct RawParser {
    paths: Vec<String>,
    dataset: GtfsData,

    pub routes_name_to_inx: HashMap<String, usize>,
    pub trip_name_to_inx: HashMap<String, usize>,
    pub shape_name_to_inx: HashMap<String, usize>,
    pub stop_name_to_inx: HashMap<String, usize>,

    pub stop_times_inserted: HashMap<StopTimes, usize>,
}

struct ShapeInConstruction<'a> {
    id: String,
    raw_shapes: Vec<&'a RawShape>,
    points: Vec<Coordinate<f64>>,
}

struct StopTimeInConstruction {
    trip_id: String,
    stop_times: StopTimes,
    start_time: i64,
}

fn str_time_to_seconds(s: &str) -> i64 {
    // 05:00:00 -> 5 * 60 * 60
    return 42;
}

impl RawParser {
    pub fn parse(&mut self) -> Result<GtfsData, Error> {
        for path in paths {
            self.parse_path(path);
        }
        dataset
    }

    fn parse_path(&mut self, path: &Path) {
        self.parse_shape(path);
        self.parse_routes(path);
        self.parse_trips(path);
        self.parse_stop_times(path);
    }

    fn parse_stop_times(&mut self, path: &Path) {
        let stop_times_path = Path::new(&path).join(Path::new("stop_times.txt"));
        // Vec<(Trip_id, Vec<RawStopTime>)>
        let grouped_trips: Vec<(&str, _)> = parse_gtfs_iter::<RawStopTime>(&shape_path)
            .into_iter()
            .filter_map(Result::ok)
            .group_by(|l: RawStopTime| l.trip_id)
            .collect::<Vec<(&str, _)>>();


        let stop_times = grouped_trips
            .into_iter()
            .par_iter()
            .map(|(trip_id, raw_stop_times)| self.create_stop_times(trip_id, raw_stop_times));
        
        stop_times.for_each(|st: StopTimeInConstruction| {
            let stop_time_id = self.add_stop_times(st.stop_times);
            let trip_id = self.trip_name_to_inx.get(&st.trip_id).unwrap();
            let trip : &mut Trip = self.dataset.trips.get_mut(trip_id).unwrap();
            trip.stop_times_id = stop_time_id as i64;
            trip.start_time = st.start_time;
        })
    }


    fn create_stop_times(&self, trip_id: String, raw_stop_times: Vec<RawStopTime>) -> StopTimeInConstruction {
        assert!(raw_stop_times.len() > 0);
        let start_time = str_time_to_seconds(&raw_stop_times[0].arrival_time);

        let stop_times = raw_stop_times
            .into_iter()
            .map(|st: RawStopTime|
                StopTime {
                    stop_id: *self.stop_name_to_inx.get(&st.stop_id).unwrap() as u64,
                    time: str_time_to_seconds(&st.arrival_time) - start_time,
                })
            .collect::<Vec<StopTime>>();

        StopTimeInConstruction {
            trip_id,
            stop_times: StopTimes {
                stop_times
            },
            start_time,
        }
    }

    fn add_stop_times(&mut self, stop_times: StopTimes) -> u64 {
        let number_of_stop_times = self.dataset.stop_times.len();
        let new_id = **self.stop_times_inserted.get(&stop_times).get_or_insert(&number_of_stop_times) as u64;
        if new_id == number_of_stop_times as u64 {
            self.dataset.stop_times.push(stop_times);
        }
        new_id
    }

    fn parse_shape(&mut self, path: &Path) {
        let shape_path = Path::new(&path).join(Path::new("shape.txt"));
        let raw_shapes: Vec<RawShape> = parse_gtfs(&shape_path).expect("Raw shape parsing");

        let mut curr_shape_id = -1;

        let grouped_shapes: Vec<(String, _)> = raw_shapes
            .into_iter()
            .group_by(|l| l.shape_id)
            .into_iter()
            .collect::<Vec<(String, _)>>();

        let shapes_in_constuction = grouped_shapes
            .into_iter()
            .map(|(shape_id, vals)| ShapeInConstruction {
                id: shape_id.to_string(),
                raw_shapes: vals.into_iter().collect(),
                points: vec![],
            })
            .collect::<Vec<ShapeInConstruction>>();

        // build the shapes in parallel
        let with_points = shapes_in_constuction
            .into_iter()
            .par_iter()
            .map(|sh| {
                ShapeInConstruction {
                    id: sh.id,
                    raw_shapes: vec![],
                    points: sh.raw_shapes
                        .into_iter()
                        .map(|v: RawShape| to_coordinates(v.shape_pt_lat, v.shape_pt_lon))
                        .collect::<Vec<Coordinate<f64>>>(),
                }
            })
            .collect::<Vec<ShapeInConstruction>>();

        with_points
            .into_iter()
            .for_each(|s| {
                self.add_shape(s)
            })
    }

    fn add_shape(&mut self, shape: ShapeInConstruction) {
        let number_of_shapes = self.dataset.shapes.len();
        self.shape_name_to_inx.insert(shape.id, number_of_shapes);
        self.dataset.shapes.push(Shape {
            shape_id: number_of_shapes as u64,
            points: shape.points,
        })
    }
    fn parse_routes(&mut self, path: &Path) {
        let routes_path = Path::new(&path).join(Path::new("routes.txt"));
        let raw_routes: Vec<RawRoute> = parse_gtfs(&routes_path).expect("Raw routes parsing");
        for route in raw_routes {
            self.add_route(route);
        }
    }
    fn add_route(&mut self, route: RawRoute) {
        let number_of_routes = self.dataset.routes.len();
        self.routes_name_to_inx.insert(route.route_id, number_of_routes);
        self.dataset.routes.push(Route {
            route_id: number_of_routes as i64,
            route_short_name: route.route_short_name,
            route_long_name: route.route_long_name,
            trips: vec![],
            stops: vec![],
            dataset_index: 0,
        })
    }
    fn parse_trips(&mut self, path: &Path) {
        let trips_path = Path::new(&path).join(Path::new("trips.txt"));
        let raw_trips: Vec<RawTrip> = parse_gtfs(&trips_path).expect("Raw trips parsing");
        for route in raw_routes {
            self.add_route(route);
        }
    }

    fn add_trip(&mut self, trip: RawTrip) {
        let number_of_trips = self.dataset.trips.len();
        self.trip_name_to_inx.insert(trip.trip_id, number_of_trips);
        let trip_id = number_of_trips as i64;
        let route_id = *self.routes_name_to_inx.get(&trip.route_id).unwrap() as i64;
        let shape_id = *self.shape_name_to_inx.get(&trip.shape_id).unwrap() as i64;

        self.dataset.trips.push(Trip {
            route_id,
            shape_id,
            trip_id,
            stop_times_id: 0,
            start_time: 0,
            service_id: trip.service_id,
            trip_headsign: trip.trip_headsign,
            trip_short_name: trip.trip_short_name,
            direction_id: trip.direction_id,
            block_id: trip.block_id,
            wheelchair_accessible: trip.wheelchair_accessible,
        });

        let route_associated: &mut Route = self.dataset.routes.get_mut(route_id as usize).unwrap();
        route_associated.trips.push(trip_id);
    }
}