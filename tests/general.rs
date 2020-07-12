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
use fastgtfs::test_utils::{assert_dataset_filled, get_test_paths};

#[test]
fn basic_parsing() {
    // let mut dataset: GtfsData = Default::default();
    // let now = Instant::now();
    // let path = vec![get_test_paths()[0].to_string()];
    // dataset = raw_parser::parse_from_paths(path);
    // debug!("All parsing time: {}", now.elapsed().as_millis());
    //
    // assert_dataset_filled(&dataset)
}

#[test]
fn parse_multiple() {
    // let paths = get_test_paths();
    //
    // let mut dataset: GtfsData = Default::default();
    // let now = Instant::now();
    // dataset = parser::parse_from_paths(paths);
    // debug!("All parsing time: {}", now.elapsed().as_millis());
    //
    // assert_dataset_filled(&dataset)
}

#[cfg(test)]
fn parse_all() -> Vec<GtfsData> {
    vec![]
    // get_test_paths()
    //     .into_iter()
    //     .map(|p| parser::parse_from_paths(vec![p]))
    //     .collect::<Vec<GtfsData>>()
}

#[test]
fn routes_stoptimes_filled() {
    //todo
}

#[test]
fn groupby_test() {
    let s = "1,1,1,2,2,2,2";
    for (key, group) in &s.split(',').group_by(|n| n.parse::<i32>().unwrap()) {
        print!("{}", key);
        print!("{:#?}", group.collect::<Vec<&str>>());
    }
}
