use std::collections::HashMap;
use std::path::Path;

use itertools::Itertools;

use fastgtfs::gtfs_data::{StopTime, StopTimes};
use fastgtfs::raw_models::{parse_gtfs, RawCalendar, RawRoute};
use fastgtfs::raw_parser::{str_time_to_seconds, RawParser};
use fastgtfs::test_utils::{generate_serialized_data, get_test_paths, make_dataset};

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

    let stop_times_per_route = dataset
        .routes
        .iter()
        .map(|route| {
            (
                route.route_id,
                (
                    route
                        .trips
                        .iter()
                        .map(|trip_id| dataset.get_trip(*trip_id))
                        .map(|trip| trip.stop_times_id)
                        .unique()
                        .count(),
                    route.trips.len(),
                ),
            )
        })
        .collect::<HashMap<usize, (_, _)>>();

    let multiple_paths = stop_times_per_route
        .iter()
        .filter(|(_, (n, _))| *n > 1)
        .count();
    let multiple_paths_4 = stop_times_per_route
        .iter()
        .filter(|(_, (n, _))| *n > 3)
        .count();
    println!(
        "Routes with multiple paths: {}/{}",
        multiple_paths,
        dataset.routes.len()
    );
    println!(
        "Routes with more than 3 paths: {}/{}",
        multiple_paths_4,
        dataset.routes.len()
    );
    //println!("{:?}", stop_times_per_route);
    assert!(multiple_paths < dataset.routes.len() / 4);
    assert!(multiple_paths_4 < dataset.routes.len() / 10);
}

#[test]
fn test_routes_per_stop() {
    let ds = make_dataset();
    let alone_stops = ds
        .stops
        .iter()
        .filter(|s| s.routes.is_empty())
        .map(|s| (s.stop_name.clone(), s.routes.len()))
        .collect::<Vec<(String, usize)>>();

    println!(
        "Stops without routes: {}/{}\n {:?}",
        alone_stops.len(),
        ds.stops.len(),
        alone_stops
    );
    assert!(alone_stops.len() < ds.stops.len() / 20);
}

#[test]
fn test_vector_equality() {
    let mut map: HashMap<StopTimes, usize> = HashMap::new();
    let v1 = StopTimes {
        stop_times_id: 0,
        stop_times: vec![StopTime {
            stop_id: 1,
            time: 1,
        }],
    };
    let v2 = StopTimes {
        stop_times_id: 0,
        stop_times: vec![StopTime {
            stop_id: 1,
            time: 1,
        }],
    };

    map.insert(v1, 1);
    assert_eq!(*(map.get(&v2).unwrap()), 1 as usize);
}

#[test]
fn test_trip_services() {
    let ds = make_dataset();
    for trip in ds.trips {
        assert!(trip.service_id.is_some());
    }
}

#[test]
fn valid_stop_times() {
    let ds = RawParser::read_preprocessed_data_from_default();
    let seconds_in_hour = 60 * 60;

    ds.stop_times.iter().for_each(|st| {
        assert!(!st.stop_times.is_empty());
        for stop_time in &st.stop_times {
            assert!(stop_time.time < seconds_in_hour * 4); // is there a trip that is more than 4 h? if so, remove this
        }
    });
}

#[test]
fn valid_trip_start_times() {
    let ds = RawParser::read_preprocessed_data_from_default();
    let seconds_in_hour = 60 * 60;
    let four_am = 4 * seconds_in_hour;
    let mut starting_after_four = 0;

    ds.trips.iter().for_each(|t| {
        if t.start_time > four_am {
            starting_after_four += 1;
        }
    });
    println!(
        "Starting after four : {}/{}",
        starting_after_four,
        ds.trips.len()
    );
    assert!(starting_after_four > ds.trips.len() / 20);
}

#[test]
fn valid_trips() {
    let ds = RawParser::read_preprocessed_data_from_default();
    for route in &ds.routes {
        assert!(!route.trips.is_empty(), "Route without trips: {:?}", route);
    }
}

#[test]
fn valid_trip_times() {
    // verifies that all times are sequential
    let ds = RawParser::read_preprocessed_data_from_default();
    for stop_times in &ds.stop_times {
        let st = &stop_times.stop_times;
        let mut prec_time = st.first().unwrap().time;
        for i in st {
            assert!(i.time >= prec_time);
            prec_time = i.time;
        }
        assert_ne!(st.first().unwrap().time, st.last().unwrap().time);
    }
}

#[test]
fn repeated_stops_in_stop_times() {
    // Checks how many stopTimes with duplicate stops there are.
    let ds = RawParser::read_preprocessed_data_from_default();
    let mut cnt = 0;
    for stop_times in &ds.stop_times {
        let st = &stop_times.stop_times;
        if st.len()
            != st
                .clone()
                .into_iter()
                .unique_by(|i| i.stop_id)
                .collect_vec()
                .len()
        {
            cnt += 1
        }
    }
    println!("dup: {}/{}", cnt, ds.stop_times.len());
}

#[test]
fn date_parsing() {
    let seconds_in_hour = 60 * 60;
    let five_am = 5 * seconds_in_hour;
    assert_eq!(str_time_to_seconds("05:01:02"), five_am + 60 + 2);
}

#[test]
fn test_walk_distance() {
    let ds = make_dataset();
    assert_eq!(ds.walk_times.len(), ds.stops.len());
    assert_eq!(ds.stops[10].stop_id, ds.walk_times[10].stop_id);

    let without = ds
        .walk_times
        .iter()
        .filter(|i| i.near_stops.is_empty())
        .count();
    println!("Without: {}", without);
    assert!(
        without < (ds.stops.len() as f64 * 0.06) as usize,
        "number of stops without near data: {} out of {}",
        without,
        ds.stops.len()
    );

    // ideally, this is to uncomment
    /* for i in ds.walk_times {
        if !i.near_stops.is_empty() {
            assert_eq!(i.near_stops[0].stop_id, i.stop_id)
        }
    }*/
}
