extern crate flexbuffers;
extern crate serde;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::format;
use std::fs::File;
use std::io::{Error, Read, Seek, SeekFrom};
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::thread;

use geo::Coordinate;
use itertools::{enumerate, Itertools};
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GtfsData {
    pub dataset_id: u32,
    // pub calendar: HashMap<String, Calendar>,
    // pub calendar_dates: HashMap<String, Vec<CalendarDate>>,
    pub routes: Vec<Route>,
    pub trips: Vec<Trip>,
    pub shapes: Vec<Shape>,
    pub stops: Vec<Stop>,

    pub stop_times: Vec<StopTimes>,
    //pub agencies: Vec<Agency>,
}

// contains a list of stops, and the time for each in seconds (the first has time 0)
#[derive(Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct StopTimes {
    pub(crate) stop_times: Vec<StopTime>,
}

#[derive(Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct StopTime {
    pub stop_id: u64,
    pub time: i64, // in seconds
}

impl GtfsData {
    pub fn get_trip(&self, id: usize) -> &Trip {
        &self.trips[id]
    }
}

impl Ord for GtfsData {
    fn cmp(&self, other: &Self) -> Ordering {
        self.trips.len().cmp(&other.trips.len())
    }
}

impl PartialOrd for GtfsData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for GtfsData {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for GtfsData {}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Route {
    pub route_id: i64,
    pub(crate) route_short_name: String,
    pub(crate) route_long_name: String,

    pub trips: Vec<i64>,
    pub stops: Vec<i64>,
    pub dataset_index: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Trip {
    pub route_id: i64,
    pub trip_id: i64,
    pub shape_id: i64,
    pub stop_times_id: i64,
    // todo: this points to a vec<StopTime>
    pub start_time: i64, // in seconds since midnight. To get all stop times use stop_times_id and add the start time to each.

    pub(crate) service_id: String,
    pub(crate) trip_headsign: String,
    pub(crate) trip_short_name: String,
    pub(crate) direction_id: String,
    pub(crate) block_id: String,
    pub(crate) wheelchair_accessible: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Shape {
    pub(crate) shape_id: u64,
    pub(crate) points: Vec<LatLng>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Stop {
    pub stop_id: i64,
    pub(crate) stop_name: String,
    pub(crate) stop_pos: LatLng,
    pub(crate) stop_timezone: String,
}

pub fn to_coordinates(lat: &str, lng: &str) -> LatLng {
    //println!("lat {}, lng:{}",lat,lng);
    LatLng {
        lat: lat.parse::<f64>().unwrap_or(0.0),
        lng: lng.parse::<f64>().unwrap_or(0.0),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LatLng {
    pub lat: f64,
    pub lng: f64,
}

impl LatLng {
    fn as_coordinates(&self) -> Coordinate<f64> {
        Coordinate {
            x: self.lat,
            y: self.lng,
        }
    }
}
