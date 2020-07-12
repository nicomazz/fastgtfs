use crate::gtfs_data::GtfsData;
use crate::raw_parser::RawParser;
use std::time::Instant;

pub fn get_test_paths() -> Vec<String> {
    ["actv_aut", "actv_nav", "alilaguna"]
        .iter()
        .map(|s| format!("./test_data/{}", s.to_owned()))
        .collect::<Vec<String>>()
}

pub fn assert_dataset_filled(dataset: &GtfsData) {
    assert!(dataset.routes.len() > 0, "Routes empty!");
    assert!(dataset.trips.len() > 0, "Trips empty!");
    assert!(dataset.shapes.len() > 0, "Shapes empty!");
    assert!(dataset.stops.len() > 0, "Stops empty!");
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
    parser.generate_serialized_data();
    println!(
        "Generating serialized data in: {}",
        now.elapsed().as_millis()
    );
}
