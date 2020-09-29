use std::time::Instant;

use crate::gtfs_data::GtfsData;
use crate::raw_parser::RawParser;

pub fn get_test_paths() -> Vec<String> {
    ["actv_aut", "actv_nav", "alilaguna"]
        .iter()
        .map(|s| format!("./test_data/{}", s.to_owned()))
        .collect::<Vec<String>>()
}

pub fn assert_dataset_filled(dataset: &GtfsData) {
    assert!(!dataset.routes.is_empty(), "Routes empty!");
    assert!(!dataset.trips.is_empty(), "Trips empty!");
    assert!(!dataset.shapes.is_empty(), "Shapes empty!");
    assert!(!dataset.stops.is_empty(), "Stops empty!");
}

pub fn assert_dataset_empty(dataset: &GtfsData) {
    assert_eq!(dataset.routes.len(), 0);
    assert_eq!(dataset.trips.len(), 0);
    assert_eq!(dataset.shapes.len(), 0);
    assert_eq!(dataset.stops.len(), 0);
}

pub fn generate_serialized_data() {
    let test_paths = get_test_paths();
    let mut parser = RawParser::new(test_paths);
    let now = Instant::now();
    parser.generate_serialized_data_into_default();
    println!(
        "Generating serialized data in: {}",
        now.elapsed().as_millis()
    );
}

pub fn make_dataset() -> GtfsData {
    let test_paths = get_test_paths();
    let mut parser = RawParser::new(test_paths);
    parser.parse();
    parser.dataset
}
