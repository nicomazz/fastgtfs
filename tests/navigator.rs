use fastgtfs::test_utils::get_test_paths;
use fastgtfs::raw_parser::RawParser;
use fastgtfs::navigator::RaptorNavigator;
use fastgtfs::gtfs_data::LatLng;

#[test]
fn test_navigator() {
    let test_paths = get_test_paths();
    let mut parser = RawParser::new(test_paths);
    parser.ensure_data_serialized_created();
    let dataset = RawParser::read_preprocessed_data_from_default();
    println!("Dataset parsed!");
    let mut navigator = RaptorNavigator::new(&dataset);

    let venice = LatLng { lat: 45.437771117019466, lng: 12.31865644454956};
    let nave = LatLng { lat: 45.45926209023005, lng: 12.21256971359253};
    navigator.find_path_from_coordinates(venice, nave, 3, 0);
}