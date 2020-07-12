use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::time::Instant;

use itertools::Itertools;
use rayon::iter::ParallelIterator;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator};

use crate::gtfs_data::{
    to_coordinates, GtfsData, LatLng, Route, Shape, Stop, StopTime, StopTimes, Trip,
};
use crate::raw_models::{parse_gtfs, RawRoute, RawShape, RawStop, RawStopTime, RawTrip};

#[derive(Debug, Default)]
pub struct RawParser {
    paths: Vec<String>,
    pub dataset: GtfsData,

    pub routes_name_to_inx: HashMap<String, usize>,
    pub trip_name_to_inx: HashMap<String, usize>,
    pub shape_name_to_inx: HashMap<String, usize>,
    pub stop_name_to_inx: HashMap<String, usize>,

    pub stop_times_inserted: HashMap<StopTimes, usize>,
}

#[derive(Debug, Default)]
struct ShapeInConstruction<'a> {
    id: String,
    raw_shapes: Vec<&'a RawShape>,
    points: Vec<LatLng>,
}

struct StopTimeInConstruction {
    trip_id: String,
    stop_times: StopTimes,
    start_time: i64,
}

// 05:00:00
fn str_time_to_seconds(s: &str) -> i64 {
    let sp: Vec<i64> = s.split(':').map(|s| s.parse::<i64>().unwrap()).collect();
    let (h, m, s) = (sp[0], sp[1], sp[2]);
    s + m * 60 + h * 60 * 60
}

struct StopTimesInConstruction {
    trip_id: String,
    stop_times: Vec<RawStopTime>,
}

const DEFAULT_OUT_PATH: &str = "gtfs_serialized";

mod gtfs_serializer {
    use std::fs::File;
    use std::io::Write;
    use std::thread;
    use std::thread::JoinHandle;

    use serde::Serialize;

    use crate::gtfs_data::GtfsData;

    fn serialize_vector<T: 'static + serde::Serialize + Sync + Send>(
        out_path: String,
        name: &'static str,
        v: Vec<T>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut buffer = flexbuffers::FlexbufferSerializer::new();
            v.serialize(&mut buffer).unwrap();
            let mut output_file = File::create(format!("{}/{}", out_path, name)).unwrap();
            output_file.write_all(buffer.view()).unwrap();
        })
    }

    pub fn generate_serialized_data(ds: GtfsData, folder: String) {
        let f = folder;
        vec![
            serialize_vector(f.clone(), "routes", ds.routes),
            serialize_vector(f.clone(), "trips", ds.trips),
            serialize_vector(f.clone(), "shapes", ds.shapes),
            serialize_vector(f.clone(), "stops", ds.stops),
            serialize_vector(f.clone(), "stop_times", ds.stop_times),
        ]
        .into_iter()
        .for_each(|v| {
            v.join().unwrap();
        });
    }
}

mod gtfs_deserializer {
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;
    use std::thread;
    use std::thread::JoinHandle;
    use std::time::Instant;

    use serde::de::DeserializeOwned;
    use serde::Deserialize;

    use crate::gtfs_data::GtfsData;

    fn deserialize_vector<T: 'static + DeserializeOwned + Sync + Send>(
        in_file: String,
    ) -> JoinHandle<Vec<T>> {
        thread::spawn(move || {
            let now = Instant::now();
            let content = read_file(Path::new(&in_file));
            let r = flexbuffers::Reader::get_root(&content).unwrap();
            let res = Vec::<T>::deserialize(r).unwrap();
            println!(
                "Reading serialized for {} in: {}",
                in_file,
                now.elapsed().as_millis()
            );
            res
        })
    }

    pub fn read_serialized_data(folder: String) -> GtfsData {
        let routes_t = deserialize_vector(folder.clone() + "/routes");
        let trips_t = deserialize_vector(folder.clone() + "/trips");
        let shapes_t = deserialize_vector(folder.clone() + "/shapes");
        let stops_t = deserialize_vector(folder.clone() + "/stops");
        let stop_times_t = deserialize_vector(folder.clone() + "/stop_times");

        GtfsData {
            dataset_id: 0,
            routes: routes_t.join().unwrap(),
            trips: trips_t.join().unwrap(),
            shapes: shapes_t.join().unwrap(),
            stops: stops_t.join().unwrap(),
            stop_times: stop_times_t.join().unwrap(),
        }
    }

    fn read_file(path: &Path) -> Vec<u8> {
        let mut content = vec![];
        File::open(path).unwrap().read_to_end(&mut content).unwrap();
        content
    }
}

impl RawParser {
    pub fn new(paths: Vec<String>) -> RawParser {
        RawParser {
            paths,
            ..Default::default()
        }
    }

    pub fn read_preprocessed_data_from_default() -> GtfsData {
        RawParser::read_preprocessed_data(DEFAULT_OUT_PATH.to_string())
    }

    pub fn read_preprocessed_data(folder: String) -> GtfsData {
        let now = Instant::now();
        let res = gtfs_deserializer::read_serialized_data(folder);
        println!("Reading serialized data in: {}", now.elapsed().as_millis());
        res
    }

    pub fn generate_serialized_data_into_default(&mut self) {
        self.generate_serialized_data(DEFAULT_OUT_PATH)
    }

    pub fn generate_serialized_data(&mut self, out_folder: &str) {
        if !Path::new(out_folder).exists() {
            println!("Creating output path!");
            fs::create_dir_all(out_folder).expect("Can't create output folder!");
        }
        self.parse();
        let ds = std::mem::take(&mut self.dataset);
        gtfs_serializer::generate_serialized_data(ds, out_folder.to_string());
    }

    pub fn parse(&mut self) {
        self.paths
            .clone()
            .iter()
            .map(|p| Path::new(p))
            .for_each(|p| self.parse_path(p));
    }

    fn parse_path(&mut self, path: &Path) {
        self.parse_stops(path);
        self.parse_shape(path);
        self.parse_routes(path);
        self.parse_trips(path);
        self.parse_stop_times(path);
    }

    fn parse_stops(&mut self, path: &Path) {
        let stop_path = Path::new(&path).join(Path::new("stops.txt"));
        let raw_stops: Vec<RawStop> = parse_gtfs(&stop_path).expect("Stop parsing");
        raw_stops.into_iter().for_each(|s| self.add_stop(s));
    }

    fn add_stop(&mut self, stop: RawStop) {
        let number_of_stops = self.dataset.stops.len();
        self.stop_name_to_inx.insert(stop.stop_id, number_of_stops);
        self.dataset.stops.push(Stop {
            stop_id: number_of_stops as i64,
            stop_name: stop.stop_name,
            stop_pos: LatLng {
                lat: stop.stop_lat.parse::<f64>().unwrap(),
                lng: stop.stop_lon.parse::<f64>().unwrap(),
            },
            stop_timezone: "".to_string(),
        })
    }

    fn parse_stop_times(&mut self, path: &Path) {
        let stop_times_path = Path::new(&path).join(Path::new("stop_times.txt"));
        let mut reader = csv::Reader::from_reader(File::open(&stop_times_path).unwrap());
        let grouped_trips: Vec<StopTimesInConstruction> = reader
            .deserialize()
            .filter_map(Result::ok)
            .group_by(|l: &RawStopTime| l.trip_id.clone())
            .into_iter()
            .map(|(trip_id, raw_stop_times)| StopTimesInConstruction {
                trip_id,
                stop_times: raw_stop_times.collect(),
            })
            .collect();

        let stop_times = grouped_trips
            .into_par_iter()
            .map(|stop_times_in_construction| self.create_stop_times(stop_times_in_construction))
            .collect::<Vec<StopTimeInConstruction>>();

        stop_times
            .into_iter()
            .for_each(|st: StopTimeInConstruction| {
                let stop_time_id = self.add_stop_times(st.stop_times);
                let trip_id = *self.trip_name_to_inx.get(&st.trip_id).unwrap();
                let trip: &mut Trip = self.dataset.trips.get_mut(trip_id).unwrap();
                trip.stop_times_id = stop_time_id as i64;
                trip.start_time = st.start_time;
            });
    }

    fn create_stop_times(
        &self,
        stop_times_in_construction: StopTimesInConstruction,
    ) -> StopTimeInConstruction {
        let raw_stop_times = stop_times_in_construction.stop_times;
        let trip_id = stop_times_in_construction.trip_id;
        assert!(!raw_stop_times.is_empty());
        let start_time = str_time_to_seconds(&raw_stop_times[0].arrival_time);
        let stop_times = raw_stop_times
            .par_iter()
            .map(|st| {
                let stop_id = *self.stop_name_to_inx.get(&st.stop_id).unwrap() as u64;
                StopTime {
                    stop_id,
                    time: str_time_to_seconds(&st.arrival_time) - start_time,
                }
            })
            .collect::<Vec<StopTime>>();

        StopTimeInConstruction {
            trip_id,
            stop_times: StopTimes { stop_times },
            start_time,
        }
    }

    fn add_stop_times(&mut self, stop_times: StopTimes) -> u64 {
        let number_of_stop_times = self.dataset.stop_times.len();
        let new_id = **self
            .stop_times_inserted
            .get(&stop_times)
            .get_or_insert(&number_of_stop_times) as u64;
        if new_id == number_of_stop_times as u64 {
            self.dataset.stop_times.push(stop_times);
        }
        new_id
    }

    fn parse_shape(&mut self, path: &Path) {
        let shape_path = Path::new(&path).join(Path::new("shapes.txt"));
        let raw_shapes: Vec<RawShape> = parse_gtfs(&shape_path).expect("Raw shape parsing");

        let grouped_shapes = raw_shapes.iter().group_by(|l| &l.shape_id[..]);

        let shapes_in_construction: Vec<ShapeInConstruction> = grouped_shapes
            .into_iter()
            .map(|(shape_id, vals)| ShapeInConstruction {
                id: shape_id.to_string(),
                raw_shapes: vals.into_iter().collect(),
                points: vec![],
            })
            .collect();

        // build the shapes in parallel
        let with_points = shapes_in_construction
            .into_par_iter()
            .map(|sh| ShapeInConstruction {
                id: sh.id,
                raw_shapes: vec![],
                points: sh
                    .raw_shapes
                    .into_iter()
                    .map(|v: &RawShape| to_coordinates(&v.shape_pt_lat, &v.shape_pt_lon))
                    .collect::<Vec<LatLng>>(),
            })
            .collect::<Vec<ShapeInConstruction>>();

        with_points.into_iter().for_each(|s| self.add_shape(s))
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
        self.routes_name_to_inx
            .insert(route.route_id, number_of_routes);
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
        for trip in raw_trips {
            self.add_trip(trip);
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
