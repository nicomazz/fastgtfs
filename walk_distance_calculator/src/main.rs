use std::cmp::max;
use std::cmp::min;
use std::collections::HashMap;
use std::env;


use std::fmt::Write as W;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use fastgtfs::gtfs_data::LatLng;
use fastgtfs::gtfs_data::{near_stops, Stop};
use fastgtfs::raw_models::{parse_gtfs, RawStop};
use fastgtfs::raw_parser::RawParser;
use fastgtfs::test_utils::get_test_paths;
use itertools::Itertools;


use thiserror::Error;

use serde::{Deserialize, Serialize};


use crate::here_client::here_api;

pub mod here_client;
pub mod partial_save;

/// This reads several stop.txt files, and creates a time matrix between each stop and the nearest
/// N stops. We use the "HERE" api to do that, because it provides 250k free requests at month, and
/// a convenient matrix api.
///
/// It resumes previous work in case it is stopped, using the intermediate "TEMP_FILE_NAME" file

const NEAR_NUMBER: usize = 40;
const TEMP_FILE_NAME: &str = "temp_walk_results";
const FINAL_RESULT: &str = "walk_results.txt";

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("data store disconnected")]
    ReqwestError(#[from] reqwest::Error),
    #[error("{0}")]
    CustomError(String),
    #[error("unknown data store error")]
    Unknown,
}

// Represents a stop with nearby stops
#[derive(Debug)]
struct StopWithNearby<'a> {
    stop: &'a Stop,
    nearby: Vec<&'a Stop>,
}

type RResult<T> = std::result::Result<T, RequestError>;
type Distance = usize;
type StopId = usize;
type DistancesResult = HashMap<StopPair, Distance>;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
struct StopPair {
    a: StopId,
    b: StopId,
}

impl StopPair {
    fn new(stop1: usize, stop2: usize) -> StopPair {
        StopPair {
            a: min(stop1, stop2),
            b: max(stop1, stop2),
        }
    }
    fn from_stops(stop1: &Stop, stop2: &Stop) -> StopPair {
        StopPair::new(stop1.stop_id, stop2.stop_id)
    }
}

fn find_near_stops<'a>(base: &'a Stop, all: &'a Vec<Stop>) -> Vec<&'a Stop> {
    near_stops(&base.stop_pos, NEAR_NUMBER, all)
        .iter()
        .map(|&s| &all[s])
        .collect_vec()
}

fn already_calculated_stops(from: &Stop, to: &Stop, res: &DistancesResult) -> bool {
    res.contains_key(&StopPair::from_stops(from,to))
}

fn compute_distances(
    from: &Stop,
    near: &Vec<&Stop>,
    res: &DistancesResult,
) -> Vec<(StopPair, Distance)> {
    let missing = near
        .iter()
        .filter(|to| !res.contains_key(&StopPair::from_stops(from,to)))
        .map(|&to| to)
        .collect_vec();

    let missing_positions = missing.iter().map(|i| i.stop_pos.clone()).collect_vec();

    if missing_positions.is_empty() {
        return vec![];
    }

    let distances =
        here_api::here_distance_request(from.stop_pos.clone(), missing_positions).unwrap();
    //sleep for 1 second to avoid overloading the api
    std::thread::sleep(std::time::Duration::from_secs(1));
    let filled = distances.iter().filter(|i| !i.is_none()).count();
    println!(
        "From stop {}, {} computed, {} nones out of {}",
        from.stop_id,
        filled,
        (distances.len() - filled),
        distances.len()
    );

    missing
        .iter()
        .zip(distances)
        .filter_map(|(to, distance)| distance.map(|dist| (StopPair::from_stops(from, &to), dist)))
        .collect_vec()
}


fn output_file(stops: &Vec<Stop>, near_stops: Vec<StopWithNearby>, res: DistancesResult) {
    let mut output = String::new();

    // First: numbers of stops, and for each stop how many nearby we consider.
    writeln!(&mut output, "{};{}", stops.len(), NEAR_NUMBER).unwrap();
    // Position of each stop
    stops.iter().for_each(|s| {
        writeln!(&mut output, "{};{}", s.stop_pos.lat, s.stop_pos.lng).unwrap();
    });

    // For each stop: his id, and for each nearby: id and distance
    for stop_with_nearby in near_stops {
        write!(&mut output, "{} ", stop_with_nearby.stop.stop_id + 1).unwrap();
        for nearby in stop_with_nearby.nearby {
            let distance = res.get(&StopPair::from_stops(
                stop_with_nearby.stop,
                nearby,
            )).unwrap_or(&100000);
            write!(&mut output, "{} {} ", nearby.stop_id + 1, distance).unwrap();
        }
        writeln!(&mut output).unwrap();
    }

    let path = Path::new(FINAL_RESULT);
    let mut file = File::create(&path).unwrap();
    file.write_all(output.as_bytes()).unwrap();
    println!("Written all in file: {}", path.display());
}

// Returns the stops that are not in the cache
fn missing<'a>(
    near_stops: &'a Vec<StopWithNearby>,
    cached: DistancesResult,
) -> Vec<&'a StopWithNearby<'a>> {
    near_stops
        .iter()
        .filter_map(|stop_with_nearby| {
            let from = stop_with_nearby.stop;
            let missing = stop_with_nearby
                .nearby
                .iter()
                .any(|&to| !cached.contains_key(&StopPair::from_stops(from, to)));
            if missing {
                Some(stop_with_nearby)
            } else {
                None
            }
        })
        .collect()
}

fn find_raw_stops(test_paths: Vec<String>) -> Vec<RawStop> {
    let all_raw_stops: Vec<RawStop> = test_paths
        .iter()
        .flat_map(|path| {
            let stops_file = Path::new(&path).join(Path::new("stops.txt"));
            if !stops_file.exists() {
                panic!(
                    "Stop file {} doesn't exist. Make sure it's there.",
                    stops_file.display()
                );
            }
            parse_gtfs(&stops_file).unwrap()
        })
        .collect_vec();

    all_raw_stops
}

fn find_stops(test_paths: Vec<String>) -> Vec<Stop> {
    let all_raw_stops: Vec<RawStop> = find_raw_stops(test_paths);

    all_raw_stops
        .into_iter()
        .enumerate()
        .map(|(id, stop)| RawParser::create_stop(stop, id))
        .collect_vec()
}

fn create_nearby_arrays(stops: &Vec<Stop>) -> Vec<StopWithNearby> {
    stops
        .iter()
        .map(|s| StopWithNearby {
            stop: s,
            nearby: find_near_stops(s, &stops),
        })
        .collect_vec()
}

fn process_chunk(chunk: Vec<&StopWithNearby>, res: &DistancesResult) -> Vec<(StopPair, Distance)> {
    chunk
        //.into_par_iter() // uncomment for more parallelism
        .into_iter()
        .flat_map(|s| compute_distances(s.stop, &s.nearby, &res))
        .collect()
}

#[test]
fn test_req() {
    let venice = LatLng {
        lat: 45.437771117019466,
        lng: 12.31865644454956,
    };
    let nave_de_vero = LatLng {
        lat: 45.45926209023005,
        lng: 12.21256971359253,
    };

    let res =
        here_api::here_distance_request(venice, vec![nave_de_vero.clone(), nave_de_vero]).unwrap();
    assert!(res.iter().all(|e| e.is_some()));
    assert_eq!(res[0].unwrap(), res[1].unwrap());
}

fn main() {
    println!("cwd: {}", env::current_dir().unwrap().to_str().unwrap());
    let test_paths = get_test_paths();

    let stops = find_stops(test_paths);
    println!("Creating walk times for {} stops", stops.len());
    let near_stops = create_nearby_arrays(&stops);
    let mut res = partial_save::restore_partial_data();

    let stops_todo = missing(&near_stops, res.clone());

    println!("---> Missing stops to process: {}", stops_todo.len());

    // Creating chunks to save results in the meantime, so that processing can be restored in case of crashes.
    let chunks = stops_todo.iter().chunks(10);
    for chunk in &chunks {
        let chunk = chunk.map(|&s| s).collect_vec();
        println!("Processing new chunk...");
        let this_chunk_results = process_chunk(chunk, &res);
        for (key, value) in this_chunk_results {
            res.entry(key).or_insert(value);
        }
        partial_save::save_partial_data(&res);
        println!(
            "Computed {}/{} distances",
            res.len(),
            stops.len() * NEAR_NUMBER / 2
        );
    }

    partial_save::save_partial_data(&res);
    output_file(&stops, near_stops, res);
}
