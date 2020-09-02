use std::collections::HashMap;
use std::env;
use std::fmt::Write as W;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use itertools::Itertools;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use reqwest::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use fastgtfs::gtfs_data::{LatLng, near_stops, Stop};
use fastgtfs::raw_models::{parse_gtfs, RawStop};
use fastgtfs::raw_parser::{RawParser, read_file};
use fastgtfs::test_utils::get_test_paths;

/// This reads several stop.txt files, and creates a time matrix between each stop and the nearest
/// N stops. We use the "HERE" api to do that, because it provides 250k free requests at month, and
/// a convenient matrix api.
///
/// It resumes previous work in case it is stopped, using the ihtermediate "TEMP_FILE_NAME" file

const NEAR_NUMBER: usize = 40;
const TEMP_FILE_NAME: &str = "temp_walk_results";
const FINAL_RESULT: &str = "walk_results.txt";

type DistancesResult = HashMap<String, usize>;

fn find_near_stops<'a>(base: &'a Stop, all: &'a Vec<Stop>) -> Vec<&'a Stop> {
    //pub fn near_stops(pos: &LatLng, number: usize, stops: &Vec<Stop>) -> Vec<usize> {
    let near = near_stops(&base.stop_pos, NEAR_NUMBER, all);
    near.iter().map(|&s| &all[s]).collect_vec()
}

fn restore_partial_data() -> DistancesResult {
    let path = Path::new(TEMP_FILE_NAME);
    if !path.exists() { return DistancesResult::new(); }

    let content = read_file(path);
    let r = flexbuffers::Reader::get_root(&content).unwrap();
    DistancesResult::deserialize(r).unwrap()
}

fn save_partial_data(res: &DistancesResult) {
    let mut buffer = flexbuffers::FlexbufferSerializer::new();
    res.serialize(&mut buffer).unwrap();
    let mut output_file = File::create(TEMP_FILE_NAME).unwrap_or_else(|_| panic!("Can't create {}", TEMP_FILE_NAME));
    output_file.write_all(buffer.view()).unwrap();
}

fn into_key(a: &Stop, b: &Stop) -> String {
    let tuple = if a.stop_id < b.stop_id {
        (a.stop_id, b.stop_id)
    } else { (b.stop_id, a.stop_id) };
    format!("{};{}", tuple.0, tuple.1)
}

fn do_request(url: String) -> String {
    let mut res = reqwest::blocking::get(&url).unwrap();
    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();
    body
}

fn here_distance_request(from: LatLng, tos: Vec<LatLng>) -> Result<Vec<Option<usize>>, Error> {
    let api_key = env::var("here_api_key").unwrap();

    let start_get_param = format!("start0={},{}", from.lat, from.lng);
    let destinations_get_param = tos
        .iter()
        .enumerate()
        .map(|(i, p)| format!("destination{}={},{}", i, p.lat, p.lng))
        .join("&");
    let request_url =
        format!("https://matrix.route.ls.hereapi.com/routing/7.2\
        /calculatematrix.json?\
        apikey={}&\
        mode=fastest;pedestrian;boatFerry:-3&\
        {}&\
        {}", api_key, start_get_param, destinations_get_param);

    let body = do_request(request_url);
    let json_reply: Value = serde_json::from_str(&body).unwrap();
    let entries = json_reply["response"]["matrixEntry"].as_array().unwrap();
    assert_eq!(entries.len(), tos.len());

    Ok(entries.into_iter()
        .map(|e| e.as_object().unwrap())
        .sorted_by_key(|e| e["destinationIndex"].as_i64().unwrap())
        .map(|e| {
            if e.contains_key("summary") {
                e["summary"]["costFactor"].as_i64()
            } else { None }
        })
        .map(|e| match e {
            None => { None }
            Some(s) => { Some(s as usize) }
        }).collect())
}

fn already_calculated(a: &Stop, b: &Stop, res: &DistancesResult) -> bool {
    cached_dist(a, b, res).is_some()
}

fn cached_dist(a: &Stop, b: &Stop, res: &DistancesResult) -> Option<usize> {
    let key = into_key(a, b);

    if res.contains_key(&key) {
        return Some(*res.get(&key).unwrap());
    }
    None
}

fn compute_distances(from: &Stop, near: &Vec<&Stop>, res: &DistancesResult) -> Vec<(String, usize)> {
    let missing = near.iter().filter(|&to| !already_calculated(from, to, res)).collect_vec();
    let missing_positions = missing.iter().map(|i| i.stop_pos.clone()).collect_vec();
    if missing_positions.is_empty() { return vec![]; }

    let distances = here_distance_request(from.stop_pos.clone(), missing_positions).unwrap();
    let nones = distances.iter().filter(|i| i.is_none()).count();
    println!("{} computed, {} nones out of {}", (distances.len() - nones), nones, distances.len());

    missing.iter().zip(distances).filter_map(|(to, distance)| {
        let key = into_key(from, &to);
        if let Some(dist) = distance {
            Some((key, dist))
        } else {
            None
        }
    }).collect_vec()
}

#[test]
fn test_req() {
    let venice = LatLng { lat: 45.437771117019466, lng: 12.31865644454956 };
    let nave_de_vero = LatLng { lat: 45.45926209023005, lng: 12.21256971359253 };

    let res = here_distance_request(venice, vec![nave_de_vero.clone(), nave_de_vero]).unwrap();
    assert!(res.iter().all(|e| e.is_some()));
    assert_eq!(res[0].unwrap(), res[1].unwrap());
}

fn output_file(stops: &Vec<Stop>, near_stops: Vec<(&Stop, Vec<&Stop>)>, res: DistancesResult) {
    // todo output following the format
    let mut o = String::new();

    writeln!(&mut o, "{};{}", stops.len(), NEAR_NUMBER).unwrap();
    stops.iter().for_each(|s| {
        writeln!(&mut o, "{};{}", s.stop_pos.lat, s.stop_pos.lng).unwrap();
    });

    for (from, near) in near_stops {
        write!(&mut o, "{} ", from.stop_id + 1).unwrap();
        near.iter().for_each(|to| {
            write!(&mut o, "{} {} ", to.stop_id + 1, res.get(&into_key(from, to)).unwrap_or(&100000)).unwrap();
        });
        writeln!(&mut o).unwrap();
    }

    let path = Path::new(FINAL_RESULT);
    let mut file = File::create(&path).unwrap();
    file.write_all(o.as_bytes()).unwrap();
    println!("Written all the file!")
}

fn missing<'a>(near_stops: &'a Vec<(&Stop, Vec<&Stop>)>, res: &DistancesResult) -> Vec<(&'a Stop, &'a Vec<&'a Stop>)> {
    let mut out: Vec<(&Stop, &Vec<&Stop>)> = vec![];
    for (from, near) in near_stops {
        for to in near {
            if !res.contains_key(&into_key(from, to)) {
                out.push((from, near));
                break;
            }
        }
    }
    out
}

fn main() {
    println!("cwd: {}", env::current_dir().unwrap().to_str().unwrap());
    let test_paths = get_test_paths();

    let all_raw_stops: Vec<RawStop> = test_paths.iter().flat_map(|path| {
        let stops_file = Path::new(&path).join(Path::new("stops.txt"));
        parse_gtfs(&stops_file).unwrap()
    }).collect_vec();

    let stops = all_raw_stops
        .into_iter()
        .enumerate()
        .map(|(id, stop)| RawParser::create_stop(stop, id))
        .collect_vec();

    println!("Creating walk times for {} stops", stops.len());


    let near_stops =
        stops.iter().map(|s|
            (s, find_near_stops(s, &stops))
        ).collect_vec();

    let mut res = restore_partial_data();

    let mut todo = missing(&near_stops, &res);

    println!("---> Missing: {}", todo.len());

    let chunks = todo.iter().chunks(1000);
    for chunk in &chunks {
        print!(".");
        let this_chunk_results = chunk.collect_vec()
            .into_par_iter()
            .flat_map(|(stop, near_stops)| compute_distances(stop, &near_stops, &res))
            .collect::<Vec<(String, usize)>>();
        for (key, value) in this_chunk_results {
            res.entry(key).or_insert(value);
        }
        save_partial_data(&res);
        println!("Computed {}/{} distances", res.len(), stops.len() * NEAR_NUMBER / 2);
    }
    //todo = missing(&near_stops, &res);

    save_partial_data(&res);
    output_file(&stops, near_stops, res);
}