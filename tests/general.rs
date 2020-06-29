use std::borrow::ToOwned;
use std::iter::Iterator;
use std::sync::Mutex;
#[cfg(test)]
use std::time::{Duration, Instant};

#[cfg(test)]
use itertools::Itertools;
use lazy_static::lazy_static;
#[cfg(test)]
use log::{debug, error, info, trace, warn};
use rayon::iter::IntoParallelRefIterator;

use fastgtfs::gtfs_data::GtfsData;
use fastgtfs::models;
use fastgtfs::parser;

#[cfg(test)]
use crate::parser::finish_loading;

#[cfg(test)]
fn assert_dataset_filled(dataset: &GtfsData) {
    assert!(dataset.routes.len() > 0, "Routes empty!");
    assert!(dataset.trips.len() > 0, "Trips empty!");
    assert!(dataset.shapes.len() > 0, "Shapes empty!");
    assert!(dataset.stops.len() > 0, "Stops empty!");
}

#[cfg(test)]
fn assert_dataset_empty(dataset: &GtfsData) {
    assert_eq!(dataset.routes.len(), 0);
    assert_eq!(dataset.trips.len(), 0);
    assert_eq!(dataset.shapes.len(), 0);
    assert_eq!(dataset.stops.len(), 0);
}

pub fn get_test_paths() -> Vec<String> {
    ["actv_aut", "actv_nav", "alilaguna"]
        .iter()
        .map(|s| format!("./test_data/{}", s.to_owned()))
        .collect::<Vec<String>>()
}

#[test]
fn it_works() {
    let mut dataset: GtfsData = Default::default();
    let now = Instant::now();
    let path = vec![get_test_paths()[0].to_string()];
    dataset = parser::parse_from_paths(path);
    debug!("All parsing time: {}", now.elapsed().as_millis());

    assert_dataset_filled(&dataset)
}

#[test]
fn parse_multiple() {
    let paths = get_test_paths();

    let mut dataset: GtfsData = Default::default();
    let now = Instant::now();
    dataset = parser::parse_from_paths(paths);
    debug!("All parsing time: {}", now.elapsed().as_millis());

    assert_dataset_filled(&dataset)
}

#[cfg(test)]
fn parse_all() -> Vec<GtfsData> {
    get_test_paths()
        .into_iter()
        .map(|p| parser::parse_from_paths(vec![p]))
        .collect::<Vec<GtfsData>>()
}

#[test]
fn parse_multiple_with_final_aggregation() {
    let paths = get_test_paths();
    let now = Instant::now();
    let mut datasets = parse_all();
    let final_dataset = finish_loading(&mut datasets);
    debug!("All parsing time: {}", now.elapsed().as_millis());

    assert_dataset_filled(&final_dataset);
}

#[test]
fn assert_trip_info_filled() {
    let mut datasets = parse_all();
    let final_dataset = finish_loading(&mut datasets);
    assert_dataset_filled(&final_dataset);
    for trip in final_dataset.trips {
        assert_ne!(
            trip.stop_times_indexes.size, 0,
            "trip with id {:?} has null stop_times",
            trip.trip_id
        );
    }
}

#[test]
fn groupby_test() {
    let s = "1,1,1,2,2,2,2";
    for (key, group) in &s
        .split(",")
        .into_iter()
        .group_by(|n| n.parse::<i32>().unwrap())
    {
        print!("{}", key);
        print!("{:#?}", group.collect::<Vec<&str>>());
    }
}
