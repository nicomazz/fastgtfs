//use std::path::Path;
// 1.4.0
use std::sync::Mutex;


use lazy_static::lazy_static;

use crate::models::GtfsData;


//mod models;
pub mod parser;
pub mod models;

pub fn testing() {
    print!("it works!");
}


//
// pub fn load_dataset(path: String, dataset_id: i8) {}
//
// pub fn loading_ended() {}
//
// pub fn get_routes_iterator() {}
//
// pub fn get_shape(route_id: i32, trip_id: i32) {}
//
// pub fn get_route_desc(route_id: i32) {}
//
// pub fn get_stop_desc(stop_id: i32) {}
