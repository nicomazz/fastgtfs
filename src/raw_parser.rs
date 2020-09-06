use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::io::{Error, Read};
use std::path::Path;
use std::time::Instant;
use std::{env, fs};

use geo::algorithm::geodesic_distance::GeodesicDistance;
use itertools::Itertools;
use log::error;
use rayon::iter::ParallelIterator;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator};

use crate::gtfs_data::{
    to_coordinates, GtfsData, GtfsTime, LatLng, Route, Service, ServiceException, Shape, Stop,
    StopDistance, StopTime, StopTimes, StopWalkTime, Trip,
};
use crate::raw_models::{
    parse_gtfs, RawRoute, RawService, RawServiceException, RawShape, RawStop, RawStopTime, RawTrip,
};

#[derive(Debug, Default)]
pub struct RawParser {
    paths: Vec<String>,
    pub dataset: GtfsData,

    pub routes_name_to_inx: HashMap<String, usize>,
    pub trip_name_to_inx: HashMap<String, usize>,
    pub shape_name_to_inx: HashMap<String, usize>,
    pub stop_name_to_inx: HashMap<String, usize>,
    pub service_name_to_inx: HashMap<String, usize>,

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
pub fn str_time_to_seconds(s: &str) -> i64 {
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
            let out_file_name = &format!("{}/{}", out_path, name);
            let mut output_file = File::create(out_file_name)
                .unwrap_or_else(|_| panic!("Can't create {}", out_file_name));
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
            serialize_vector(f.clone(), "services", ds.services),
            serialize_vector(f.clone(), "walk_times", ds.walk_times),
        ]
        .into_iter()
        .for_each(|v| {
            v.join().unwrap();
        });
    }
}

mod gtfs_deserializer {
    use std::path::Path;
    use std::thread;
    use std::thread::JoinHandle;
    use std::time::Instant;

    use serde::de::DeserializeOwned;
    use serde::Deserialize;

    use crate::gtfs_data::GtfsData;
    use crate::raw_parser::read_file;

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
        let services_t = deserialize_vector(folder.clone() + "/services");
        let walk_times_t = deserialize_vector(folder.clone() + "/walk_times");

        GtfsData {
            dataset_id: 0,
            routes: routes_t.join().unwrap(),
            trips: trips_t.join().unwrap(),
            shapes: shapes_t.join().unwrap(),
            stops: stops_t.join().unwrap(),
            services: services_t.join().unwrap(),
            stop_times: stop_times_t.join().unwrap(),
            walk_times: walk_times_t.join().unwrap(),
        }
    }
}

pub fn read_file(path: &Path) -> Vec<u8> {
    let mut content = vec![];
    File::open(path).unwrap().read_to_end(&mut content).unwrap();
    content
}

impl RawParser {
    pub fn new(paths: Vec<String>) -> RawParser {
        RawParser {
            paths,
            ..Default::default()
        }
    }

    pub fn parse(&mut self) {
        self.paths
            .clone()
            .iter()
            .map(|p| Path::new(p))
            .for_each(|p| self.parse_path(p));
        self.try_parse_walk_paths();
    }

    fn parse_path(&mut self, path: &Path) {
        self.parse_stops(path);
        self.parse_shape(path);
        self.parse_routes(path);
        self.parse_services(path);
        self.parse_trips(path);
        self.parse_stop_times(path);
        self.assign_stop_times_to_routes();
        self.assign_routes_to_stops();
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
    pub fn ensure_data_serialized_created(&mut self) {
        fs::create_dir_all(DEFAULT_OUT_PATH).unwrap();
        self.ensure_data_serialized_created_in_path(DEFAULT_OUT_PATH)
    }

    pub fn ensure_data_serialized_created_in_path(&mut self, path: &str) {
        println!("Ensuring data serialized");
        // todo, also check the version of the data
        let routes_file = format!("{}/routes", path);
        if !Path::new(&routes_file).exists() {
            self.generate_serialized_data(path);
            return;
        }

        let metadata = fs::metadata(routes_file).unwrap();
        let last_modified = metadata.modified().unwrap().elapsed().unwrap().as_secs();

        println!("Last modified: {}", last_modified);
        if last_modified > 60 * 60 * 24 {
            // rebuild the data every day
            println!("Generating serializable data!");
            self.generate_serialized_data(path);
        }
    }

    pub fn generate_serialized_data_into_default(&mut self) {
        self.generate_serialized_data(DEFAULT_OUT_PATH)
    }

    pub fn generate_serialized_data(&mut self, out_folder: &str) {
        println!(
            "Generating serialized data at path: {}",
            env::current_dir().unwrap().to_str().unwrap()
        );
        let path = self.paths.first().unwrap();
        let destination_folder = &format!("{}/{}", path, out_folder);
        if !Path::new(destination_folder).exists() {
            println!("Creating output path!");
            match fs::create_dir_all(destination_folder) {
                Ok(_) => {}
                Err(e) => {
                    panic!("Can't create output folder {}, {}", destination_folder, e);
                }
            }
        }
        println!("Creating serialized data!");
        self.parse();
        let ds = std::mem::take(&mut self.dataset);
        gtfs_serializer::generate_serialized_data(ds, out_folder.to_string());
    }

    fn assign_routes_to_stops(&mut self) {
        let ds = &mut self.dataset;
        let routes = &ds.routes;
        let mut routes_for_stop_id: HashMap<usize, Vec<usize>> = HashMap::new(); // stop_id -> Vec<route_id>

        routes.iter().for_each(|r| {
            r.trips
                .iter()
                .map(|trip_id| ds.get_trip(*trip_id).stop_times_id)
                .unique()
                .map(|stop_times_id| ds.get_stop_times(stop_times_id))
                .for_each(|stop_times| {
                    stop_times.stop_times.iter().for_each(|stop_time| {
                        routes_for_stop_id
                            .entry(stop_time.stop_id)
                            .or_default()
                            .push(r.route_id);
                    })
                });
        });

        for (stop_id, routes) in routes_for_stop_id {
            let stop = &mut self.dataset.stops[stop_id];
            routes.iter().for_each(|r_id| {
                stop.routes.insert(*r_id);
            });
        }
    }

    fn parse_stops(&mut self, path: &Path) {
        let stop_path = Path::new(&path).join(Path::new("stops.txt"));
        let raw_stops: Vec<RawStop> = parse_gtfs(&stop_path).expect("Stop parsing");
        raw_stops.into_iter().for_each(|s| self.add_stop(s));
    }

    pub fn create_stop(stop: RawStop, stop_id: usize) -> Stop {
        Stop {
            stop_id,
            stop_name: stop.stop_name,
            stop_pos: LatLng {
                lat: stop.stop_lat.parse::<f64>().unwrap(),
                lng: stop.stop_lon.parse::<f64>().unwrap(),
            },
            stop_timezone: "".to_string(),
            routes: Default::default(),
        }
    }
    fn add_stop(&mut self, stop: RawStop) {
        let number_of_stops = self.dataset.stops.len();
        self.stop_name_to_inx
            .insert(stop.stop_id.clone(), number_of_stops);
        self.dataset
            .stops
            .push(RawParser::create_stop(stop, number_of_stops))
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
                trip.stop_times_id = stop_time_id;
                trip.start_time = st.start_time;
            });
    }
    fn assign_stop_times_to_routes(&mut self) {
        let ds = &mut self.dataset;

        let routes_stop_times: Vec<BTreeSet<usize>> = ds
            .routes
            .par_iter()
            .map(|r| {
                r.trips
                    .iter()
                    .map(|trip_id| ds.get_trip(*trip_id).stop_times_id)
                    .collect()
            })
            .collect();

        ds.routes
            .iter_mut()
            .zip(routes_stop_times.into_iter())
            .for_each(|(route, stop_times)| {
                route.stop_times = stop_times;
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
                let stop_id = *self.stop_name_to_inx.get(&st.stop_id).unwrap();
                StopTime {
                    stop_id,
                    time: str_time_to_seconds(&st.arrival_time) - start_time,
                }
            })
            .collect::<Vec<StopTime>>();

        StopTimeInConstruction {
            trip_id,
            stop_times: StopTimes {
                stop_times_id: 0,
                stop_times,
            },
            start_time,
        }
    }

    fn add_stop_times(&mut self, mut stop_times: StopTimes) -> usize {
        let stop_times_inserted = &mut self.stop_times_inserted;
        let selfstop_times = &self.dataset.stop_times;
        let number_of_stop_times = selfstop_times.len();
        if !stop_times_inserted.contains_key(&stop_times) {
            stop_times_inserted.insert(stop_times.clone(), number_of_stop_times);
        }
        let new_id = *stop_times_inserted.get(&stop_times).unwrap();
        stop_times.stop_times_id = new_id;
        /*        let new_id = **stop_times_inserted
        .get(&stop_times)
        .get_or_insert(&number_of_stop_times);*/
        if new_id == number_of_stop_times {
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
            shape_id: number_of_shapes,
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
            route_id: number_of_routes,
            route_short_name: route.route_short_name,
            route_long_name: route.route_long_name,
            trips: vec![],
            stop_times: Default::default(),
            dataset_index: 0,
        })
    }

    fn parse_services(&mut self, path: &Path) {
        let services_path = Path::new(&path).join(Path::new("calendar.txt"));
        let services_exceptions_path = Path::new(&path).join(Path::new("calendar_dates.txt"));
        let raw_services: Vec<RawService> = parse_gtfs(&services_path).unwrap_or_else(|_| {
            println!("calendar.txt not found!");
            vec![]
        });
        let raw_services_exceptions: Vec<RawServiceException> =
            parse_gtfs(&services_exceptions_path).unwrap_or_else(|_| {
                println!("calendar_dates.txt not found!");
                vec![]
            });

        for service in &raw_services {
            let this_exceptions = raw_services_exceptions
                .iter()
                .filter(|e| e.service_id == service.service_id)
                .cloned()
                .collect();

            self.add_service(service, this_exceptions);
        }
        // Add remaining exceptions without a service in calendar.txt

        let service_ids = raw_services
            .iter()
            .map(|s| s.service_id.clone())
            .collect::<HashSet<String>>();
        for exception in raw_services_exceptions {
            if service_ids.contains(&exception.service_id) {
                continue;
            }
            self.add_service(
                &RawService {
                    service_id: exception.service_id.clone(),
                    monday: "0".to_string(),
                    tuesday: "0".to_string(),
                    wednesday: "0".to_string(),
                    thursday: "0".to_string(),
                    friday: "0".to_string(),
                    saturday: "0".to_string(),
                    sunday: "0".to_string(),
                    start_date: "19700101".to_string(),
                    end_date: "19700101".to_string(),
                },
                vec![exception],
            );
        }
    }

    fn add_service(&mut self, service: &RawService, exceptions: Vec<RawServiceException>) {
        let number_of_services = self.dataset.services.len();
        self.service_name_to_inx
            .insert(service.service_id.clone(), number_of_services);

        self.dataset.services.push(Service {
            service_id: number_of_services,
            days: self.generate_service_days(&service),
            start_date: GtfsTime::from_date(&service.start_date),
            end_date: GtfsTime::from_date(&service.end_date),
            exceptions: exceptions
                .into_iter()
                .map(|e| ServiceException {
                    date: GtfsTime::from_date(&e.date),
                    running: e.exception_type == "1",
                })
                .collect(),
        })
    }

    fn generate_service_days(&self, service: &RawService) -> Vec<bool> {
        let days = vec![
            &service.monday,
            &service.tuesday,
            &service.wednesday,
            &service.thursday,
            &service.friday,
            &service.saturday,
            &service.sunday,
        ];
        days.into_iter().map(|d| d == "1").collect::<Vec<bool>>()
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
        self.trip_name_to_inx
            .insert(trip.trip_id.clone(), number_of_trips);
        let trip_id = number_of_trips;
        let route_id = *self.routes_name_to_inx.get(&trip.route_id).unwrap();
        let shape_id = *self.shape_name_to_inx.get(&trip.shape_id).unwrap();
        let service_id_val = self.service_name_to_inx.get(&trip.service_id);
        let service_id: Option<usize> = if let Some(s_id) = service_id_val {
            Some(*s_id)
        } else {
            error!("Shape id not found: {}, trip: {:?}", trip.service_id, trip);
            None
        };

        self.dataset.trips.push(Trip {
            route_id,
            shape_id,
            trip_id,
            stop_times_id: 0,
            start_time: 0,
            service_id,
            trip_headsign: trip.trip_headsign,
            trip_short_name: trip.trip_short_name,
            direction_id: trip.direction_id,
            block_id: trip.block_id,
            wheelchair_accessible: trip.wheelchair_accessible,
        });

        let route_associated: &mut Route = self.dataset.routes.get_mut(route_id as usize).unwrap();
        route_associated.trips.push(trip_id);
    }

    fn try_parse_walk_paths(&mut self) {
        for path in self.paths.clone() {
            let path = Path::new(&path);
            if path.parent().is_some() {
                let walk_file_path = path.parent().unwrap().join("stop_distances_by_walk.txt");
                if walk_file_path.exists() {
                    self.parse_walk_paths(walk_file_path.as_os_str().to_str().unwrap());
                    return;
                }
            }
        }
    }
    // TODO: how to fix this; every stop real stop should be associated with the nearest point in the list, if it is within 30 meters.
    fn parse_walk_paths(&mut self, path: &str) {
        let raw_content = read_file(Path::new(&path));
        let content = std::str::from_utf8(&raw_content).unwrap();
        let mut lines = content.split('\n');

        /*
         *  Top: number of stops and number of near stops for each stop
         */
        let top: Vec<&str> = lines.next().unwrap().split(';').collect();
        let stops_number = top[0].parse::<usize>().unwrap();
        //let adj_stops_number = top[1].parse::<usize>().unwrap();

        /*
         *  List of points. :lat;lng"
         */
        
        let stop_positions = (0..stops_number)
            .map(|_| lines.next().unwrap())
            .map(|l| l.split(';').collect())
            .map(|v: Vec<&str>| LatLng {
                lat: v[0].parse::<f64>().unwrap(),
                lng: v[1].parse::<f64>().unwrap(),
            })
            .collect::<Vec<LatLng>>();

        // For each point, let's get the nearest stop id
        // Then, stop_mappings[i] -> stop_id associated to the i-th point.
        let stop_mappings: Vec<usize> = stop_positions
            .iter()
            .map(|p| *self.dataset.get_near_stops(p, 1).first().unwrap())
            .collect();

        let walking_result = (0..stops_number)
            .map(|_| lines.next().unwrap())
            .map(|l| {
                l.split(' ')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.parse::<usize>().unwrap())
                    .collect()
            })
            .map(|v: Vec<usize>| StopWalkTime {
                stop_id: stop_mappings[v[0] - 1],
                near_stops: v
                    .iter()
                    .skip(1)
                    .step_by(2)
                    .zip(v.iter().skip(2).step_by(2))
                    .map(|(&stop, &distance)| StopDistance {
                        stop_id: stop_mappings[stop - 1],
                        distance_meters: distance,
                    })
                    .collect(),
            })
            .collect::<Vec<StopWalkTime>>();

        println!("Size of walking results: {}", walking_result.len());
        self.dataset.walk_times = vec![];
        for stop in &self.dataset.stops {
            let (inx_nearest, dist_meters) = nearest_point(&stop.stop_pos, &stop_positions);
            if dist_meters > 30.0 {
                self.dataset.walk_times.push(StopWalkTime {
                    stop_id: stop.stop_id,
                    ..Default::default()
                });
                println!("Skipping one, dist: {}", dist_meters);
            } else {
                self.dataset.walk_times.push(StopWalkTime {
                    stop_id: stop.stop_id,
                    ..walking_result[inx_nearest].clone()
                });
            }
        }
    }
}

/// return the index in points of the nearest point to target
/// (index,distance)
fn nearest_point(target: &LatLng, points: &Vec<LatLng>) -> (usize, f64) {
    let coord = target.as_point();
    points
        .iter()
        .enumerate()
        // TODO geodesic distance is incredibly expensive, find a cheaper alternative
        .min_by_key(|(_, pos)| (pos.as_point().geodesic_distance(&coord) * 100000.0) as i64)
        .map(|(i, pos)| (i, pos.as_point().geodesic_distance(&coord)))
        .unwrap()
}
