
use fastgtfs::raw_models::{RawRoute, parse_gtfs, RawCalendar};
use fastgtfs::test_utils::get_test_paths;
use std::path::Path;

#[test]
fn routes_parsing() {
    let test_paths = get_test_paths();
    for path in test_paths {
        let routes_path = Path::new(&path).join(Path::new("routes.txt"));
        let routes: Vec<RawRoute> = parse_gtfs(&routes_path).unwrap();
        assert!(routes.len() > 0);
        for route in routes {
            assert!(route.route_id.len() > 0);
            assert!(route.agency_id.len() > 0);
            assert!(route.route_short_name.len() > 0);
            assert!(route.route_long_name.len() > 0);
            assert!(route.route_type.len() > 0);
            assert!(route.route_color.len() > 0);
            assert!(route.route_text_color.len() > 0);
        }
    }
}

#[test]
fn calendar_parsing() {
    let test_paths = get_test_paths();
    for path in test_paths {
        let calendar_path = Path::new(&path).join(Path::new("calendar.txt"));
        let calendars: Vec<RawCalendar> = parse_gtfs(&calendar_path).unwrap();
        assert!(calendars.len() > 0);
        for calendar in calendars {
            assert!(calendar.service_id.len() > 0);
            assert!(calendar.monday.len() > 0);
            assert!(calendar.tuesday.len() > 0);
            assert!(calendar.wednesday.len() > 0);
            assert!(calendar.thursday.len() > 0);
            assert!(calendar.friday.len() > 0);
            assert!(calendar.saturday.len() > 0);
            assert!(calendar.sunday.len() > 0);
            assert!(calendar.start_date.len() > 0);
            assert!(calendar.end_date.len() > 0);
        }
    }
}