use crate::models::GtfsData;

mod models;
mod parser;
use lazy_static::lazy_static; // 1.4.0
use std::sync::Mutex;
use itertools::Itertools;

lazy_static! {
    static ref gtfs_dataset: Mutex<GtfsData> = Mutex::new(GtfsData {
        routes: Vec::new(),
        trips: Vec::new()
    });
}

pub fn testing() {
    print!("it works!");
}

pub fn load_dataset(path: String, dataset_id: i8) {}

pub fn loading_ended() {}

pub fn get_routes_iterator()  {

}

pub fn get_shape(route_id: i32, trip_id: i32) {

}

pub fn get_route_desc(route_id: i32) {

}
pub fn get_stop_desc(stop_id: i32) {

}

#[test]
fn it_works() {
    let mut dataset = gtfs_dataset.lock().unwrap();
    *dataset = parser::parse_all();
    assert!(dataset.routes.len() > 0);
    assert!(dataset.trips.len() > 0);
}

#[test]
fn groupby_test() {
    let s = "1,1,1,2,2,2,2";
    for (key, group) in &s.split(",").into_iter().group_by(|n| n.parse::<i32>().unwrap()) {
        print!("{}", key);
        print!("{:#?}",group.collect::<Vec<&str>>());
    }
}
