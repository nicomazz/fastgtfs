use fastgtfs::test_utils::get_test_paths;
use fastgtfs::raw_parser::RawParser;
use fastgtfs::navigator::{RaptorNavigator, NavigationParams};
use fastgtfs::gtfs_data::{LatLng, GtfsTime};
use chrono::NaiveDate;

fn default_start_time() -> GtfsTime {
    let start_timestamp = NaiveDate::from_ymd(2020, 07, 25).and_hms(17, 33, 44).timestamp();
    GtfsTime::new_from_timestamp(start_timestamp)
}
#[test]
fn test_navigator() {
    let test_paths = get_test_paths();
    let mut parser = RawParser::new(test_paths);
    parser.ensure_data_serialized_created();
    let dataset = RawParser::read_preprocessed_data_from_default();
    println!("Dataset parsed!");

    let mut navigator = RaptorNavigator::new(&dataset);

    let venice = LatLng { lat: 45.437771117019466, lng: 12.31865644454956};
    let nave_de_vero = LatLng { lat: 45.45926209023005, lng: 12.21256971359253};

    let params = NavigationParams {
        from: venice,
        to: nave_de_vero,
        max_changes: 3,
        start_time: default_start_time()
    };
    navigator.find_path(params);
}