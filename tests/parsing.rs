use std::collections::HashMap;
use std::path::Path;

use itertools::Itertools;
use log::{debug, error, info, trace, warn};
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator};
use rayon::iter::ParallelIterator;

use fastgtfs::gtfs_data::{StopTime, StopTimes};
use fastgtfs::raw_models::{parse_gtfs, RawCalendar, RawRoute};
use fastgtfs::raw_parser::RawParser;
use fastgtfs::test_utils::{assert_dataset_filled, generate_serialized_data, get_test_paths, make_dataset};

#[test]
fn routes_parsing() {
    let test_paths = get_test_paths();
    for path in test_paths {
        let routes_path = Path::new(&path).join(Path::new("routes.txt"));
        let routes: Vec<RawRoute> = parse_gtfs(&routes_path).unwrap();
        assert!(!routes.is_empty());
        for route in routes {
            assert!(!route.route_id.is_empty());
            assert!(!route.agency_id.is_empty());
            assert!(!route.route_short_name.is_empty());
            assert!(!route.route_long_name.is_empty());
            assert!(!route.route_type.is_empty());
            assert!(!route.route_color.is_empty());
            assert!(!route.route_text_color.is_empty());
        }
    }
}

#[test]
fn calendar_parsing() {
    let test_paths = get_test_paths();
    for path in test_paths {
        let calendar_path = Path::new(&path).join(Path::new("calendar.txt"));
        let calendars: Vec<RawCalendar> = parse_gtfs(&calendar_path).unwrap();
        assert!(!calendars.is_empty());
        for calendar in calendars {
            assert!(!calendar.service_id.is_empty());
            assert!(!calendar.monday.is_empty());
            assert!(!calendar.tuesday.is_empty());
            assert!(!calendar.wednesday.is_empty());
            assert!(!calendar.thursday.is_empty());
            assert!(!calendar.friday.is_empty());
            assert!(!calendar.saturday.is_empty());
            assert!(!calendar.sunday.is_empty());
            assert!(!calendar.start_date.is_empty());
            assert!(!calendar.end_date.is_empty());
        }
    }
}

#[test]
fn dataset_parsing() {
    let dataset = make_dataset();
    assert!(!dataset.trips.is_empty());
    println!("Finished parsing in TODO");
}

#[test]
fn serialize_and_deserialize() {
    generate_serialized_data();
    RawParser::read_preprocessed_data_from_default();
    //    assert_eq!(original, deserialized);
}

// todo verify how many different stop_times there are for each route
#[test]
fn stop_times_per_route() {
    let dataset = make_dataset();

    let stop_times_per_route = dataset.routes
        .iter()
        .map(|route|
            (route.route_id,
             (route.trips
                  .iter()
                  .map(|trip_id| dataset.get_trip(*trip_id))
                  .map(|trip| trip.stop_times_id)
                  .unique()
                  .count(),
              route.trips.len(),
             )))
        .collect::<HashMap<usize, (_, _)>>();

    let multiple_paths = stop_times_per_route.iter().filter(|(_, (n, _))| *n > 1).count();
    let multiple_paths_4 = stop_times_per_route.iter().filter(|(_, (n, _))| *n > 3).count();
    println!("Routes with multiple paths: {}/{}", multiple_paths, dataset.routes.len());
    println!("Routes with more than 3 paths: {}/{}", multiple_paths_4, dataset.routes.len());
    //println!("{:?}", stop_times_per_route);
    assert!(multiple_paths < dataset.routes.len() / 4);
    assert!(multiple_paths_4 < dataset.routes.len() / 10);
}

#[test]
fn test_routes_per_stop() {
    let ds = make_dataset();
    let alone_stops = ds.stops
        .iter()
        .filter(|s| s.routes.len() == 0)
        .map(|s| (s.stop_name.clone(), s.routes.len()))
        .collect::<Vec<(String, usize)>>();

    println!("Stops without routes: {}/{}\n {:?}",alone_stops.len(), ds.stops.len(),alone_stops);
    assert!(alone_stops.len() < ds.stops.len() / 20);
}

#[test]
fn test_vector_equality() {
    let mut map: HashMap<StopTimes, usize> = HashMap::new();
    let v1 = StopTimes {
        stop_times: vec![StopTime { stop_id: 1, time: 1 }]
    };
    let v2 = StopTimes {
        stop_times: vec![StopTime { stop_id: 1, time: 1 }]
    };

    map.insert(v1, 1);
    assert_eq!(*(map.get(&v2).unwrap()), 1 as usize);
}