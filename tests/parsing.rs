use std::path::Path;

use log::{debug, error, info, trace, warn};

use fastgtfs::raw_models::{parse_gtfs, RawCalendar, RawRoute};
use fastgtfs::raw_parser::RawParser;
use fastgtfs::test_utils::{assert_dataset_filled, generate_serialized_data, get_test_paths};

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
    let test_paths = get_test_paths();
    let mut parser = RawParser::new(test_paths);
    parser.parse();
    let dataset = parser.dataset;
    assert!(!dataset.trips.is_empty());
    println!("Finished parsing in TODO");
}

#[test]
fn test_serialization() {
    generate_serialized_data();
}

#[test]
fn read_serialized_data() {
    let ds = RawParser::read_preprocessed_data();
    assert_dataset_filled(&ds);
}

#[test]
fn serialize_and_deserialize() {
    generate_serialized_data();
    RawParser::read_preprocessed_data();
    //    assert_eq!(original, deserialized);
}
