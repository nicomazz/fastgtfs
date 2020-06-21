use std::path::Path;
// 1.4.0
use std::sync::Mutex;
use std::time::{Duration, Instant};

use itertools::Itertools;

use lazy_static::lazy_static;

use crate::models::GtfsData;

mod models;
mod parser;
lazy_static! {
    static ref gtfs_dataset: Mutex<GtfsData> = Mutex::new(Default::default());
}

pub fn testing() {
    print!("it works!");
}

pub fn load_dataset(path: String, dataset_id: i8) {}

pub fn loading_ended() {}

pub fn get_routes_iterator() {}

pub fn get_shape(route_id: i32, trip_id: i32) {}

pub fn get_route_desc(route_id: i32) {}

pub fn get_stop_desc(stop_id: i32) {}


fn assert_dataset_filled(dataset: &GtfsData) {
    assert!(dataset.routes.len() > 0);
    assert!(dataset.trips.len() > 0);
    assert!(dataset.shapes.len() > 0);
    assert!(dataset.stops.len() > 0);
}

fn get_test_paths() -> Vec<String> {
    ["actv_aut", "actv_nav"].iter()
        .map(|s| format!("./test_data/{}", s.to_owned()))
        .collect::<Vec<String>>()
}

#[test]
fn it_works() {
    let mut dataset = gtfs_dataset.lock().unwrap();
    let now = Instant::now();
    let path = vec![get_test_paths()[0].to_string()];
    *dataset = parser::parse_from_paths(path);
    println!("All parsing time: {}", now.elapsed().as_millis());

    assert_dataset_filled(&dataset)
}


#[test]
fn parse_multiple() {
    let paths = get_test_paths();

    let mut dataset = gtfs_dataset.lock().unwrap();
    let now = Instant::now();
    *dataset = parser::parse_from_paths(paths);
    println!("All parsing time: {}", now.elapsed().as_millis());

    assert_dataset_filled(&dataset)
}

#[test]
fn groupby_test() {
    let s = "1,1,1,2,2,2,2";
    for (key, group) in &s.split(",").into_iter().group_by(|n| n.parse::<i32>().unwrap()) {
        print!("{}", key);
        print!("{:#?}", group.collect::<Vec<&str>>());
    }
}
