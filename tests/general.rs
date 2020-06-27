use std::iter::Iterator;
use std::borrow::ToOwned;
#[cfg(test)]
use std::time::{Duration, Instant};

#[cfg(test)]
use itertools::Itertools;

#[cfg(test)]
use log::{debug, error, info, trace, warn};

#[cfg(test)]
use crate::parser::finish_loading;

use std::sync::Mutex;

use lazy_static::lazy_static;


use fastgtfs::models;
use fastgtfs::models::GtfsData;
use fastgtfs::parser;


#[cfg(test)]
fn assert_dataset_filled(dataset: &GtfsData) {
    assert!(dataset.routes.len() > 0);
    assert!(dataset.trips.len() > 0);
    assert!(dataset.shapes.len() > 0);
    assert!(dataset.stops.len() > 0);
}

#[cfg(test)]
fn assert_dataset_empty(dataset: &GtfsData) {
    assert_eq!(dataset.routes.len(), 0);
    assert_eq!(dataset.trips.len(), 0);
    assert_eq!(dataset.shapes.len(), 0);
    assert_eq!(dataset.stops.len(), 0);
}

pub fn get_test_paths() -> Vec<String> {
    ["actv_aut", "actv_nav", "alilaguna"].iter()
        .map(|s| format!("./test_data/{}", s.to_owned()))
        .collect::<Vec<String>>()
}

#[test]
fn it_works() {
    let mut dataset : GtfsData = Default::default();
    let now = Instant::now();
    let path = vec![get_test_paths()[0].to_string()];
    dataset = parser::parse_from_paths(path);
    debug!("All parsing time: {}", now.elapsed().as_millis());

    assert_dataset_filled(&dataset)
}


#[test]
fn parse_multiple() {
    let paths = get_test_paths();

    let mut dataset : GtfsData = Default::default();
    let now = Instant::now();
    dataset = parser::parse_from_paths(paths);
    debug!("All parsing time: {}", now.elapsed().as_millis());

    assert_dataset_filled(&dataset)
}

#[test]
fn parse_multiple_with_final_aggregation() {
    let paths = get_test_paths();

    let mut datasets = Vec::<GtfsData>::new();
    let now = Instant::now();
    for path in paths {
        datasets.push(parser::parse_from_paths(vec![path]));
    }
    let final_dataset = finish_loading(datasets.as_mut());
    debug!("All parsing time: {}", now.elapsed().as_millis());

    assert_dataset_filled(&final_dataset);

    for ds in datasets {
        assert_dataset_empty(&ds);
    }
}

#[test]
fn groupby_test() {
    let s = "1,1,1,2,2,2,2";
    for (key, group) in &s.split(",").into_iter().group_by(|n| n.parse::<i32>().unwrap()) {
        print!("{}", key);
        print!("{:#?}", group.collect::<Vec<&str>>());
    }
}
