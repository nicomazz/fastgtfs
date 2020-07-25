extern crate flexbuffers;
extern crate serde;

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::format;
use std::fs::File;
use std::io::{Error, Read, Seek, SeekFrom};
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use std::time::SystemTime;

use cached::{
    proc_macro::cached,
    SizedCache,
};
use chrono::{Datelike, DateTime, Timelike, TimeZone, Utc};
use geo::{Coordinate, Point};
use geo::algorithm::geodesic_distance::GeodesicDistance;
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

impl GtfsData {
    pub fn get_stop(&self, id: usize) -> &Stop {
        &self.stops[id]
    }

    pub fn get_trip(&self, id: usize) -> &Trip {
        &self.trips[id]
    }

    pub fn get_route(&self, id: usize) -> &Route {
        &self.routes[id]
    }

    pub fn get_stop_times(&self, id: usize) -> &StopTimes {
        &self.stop_times[id]
    }


    pub fn get_route_stop_times(&self, route_id: usize) -> &StopTimes {
        let route = self.get_route(route_id);
        let trip = self.get_trip(route.trips[0]);
        self.get_stop_times(trip.stop_times_id)
    }
    /// returns the first trip that has `stop` (with inx after `start_stop_inx`) after time (not in excluded_trips)
    pub fn trip_after_time(&self, route_id: usize, stop_id: usize, min_time: i64, start_stop_inx: usize, excluded_trips: HashSet<usize>) -> Option<(&Trip, usize)> {
        let route = self.get_route(route_id);
        let trips = &route.trips;
        let stop_times = &self.get_route_stop_times(route_id).stop_times;
        let trips_duration = stop_times.last().unwrap().time;

        /// indexes of the stops in `stop_times` matching `stop_id`
        let inxes_for_stop = stop_times
            .iter()
            .skip(start_stop_inx)
            .enumerate()
            .filter(|(inx, stop_time)| stop_time.stop_id == stop_id)
            .map(|(inx, _)| inx)
            .collect::<Vec<usize>>();

        trips.iter()
            .map(|t_id| self.get_trip(*t_id))
            .filter(|trip| trip.start_time + trips_duration >= min_time)
            .find_map(|trip| {
                inxes_for_stop
                    .iter()
                    .filter(|&&inx| stop_times[inx].time + trip.start_time >= min_time)
                    .map(|inx| (trip, *inx))
                    .next()
            })
    }

    pub fn trip_active_in_day(&self, trip_id: usize, time: GtfsTime) -> bool {
        //todo
        true
    }

    // todo: overoptimize
    pub fn find_nearest_stop(&self, pos: LatLng) -> &Stop {
        let coord = pos.as_point();
        let item = self.stops.iter().min_by_key(|s| {
            s.stop_pos.as_point().geodesic_distance(&coord) as i64
        });
        item.unwrap()
    }

    pub fn get_stops_in_range(&self, pos: LatLng, meters: f64) -> Vec<usize> {
        let coord = pos.as_point();
        self.stops
            .iter()
            .filter(|&stop| stop.stop_pos.as_point().geodesic_distance(&coord) < meters)
            .map(|s| s.stop_id)
            .collect::<Vec<usize>>()
    }
    
    pub fn get_near_stops(&self, pos: &LatLng, number: usize) -> Vec<usize> {
        near_stops(pos, number, &self.stops)
    }
}

#[cached(
type = "SizedCache<(u64,u64, usize), Vec<usize>>",
create = "{ SizedCache::with_size(5000) }",
convert = r#"{ ((pos.lat * 1000.0) as u64 , (pos.lng * 1000.0) as u64,number) }"#
)]
fn near_stops(pos: &LatLng, number: usize, stops: &Vec<Stop>) -> Vec<usize> {
    let coord = pos.as_point();
    stops
        .iter()
        .sorted_by_key(|stop|
            (stop.stop_pos.as_point().geodesic_distance(&coord) * 1000.0) as i64)
        .take(number)
        .map(|stop| stop.stop_id)
        .collect::<Vec<usize>>()
}

// contains a list of stops, and the time for each in seconds (the first has time 0)
#[derive(Hash, Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct StopTimes {
    pub stop_times: Vec<StopTime>,
}

#[derive(Hash, Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct StopTime {
    pub stop_id: usize,
    pub time: i64, // in seconds
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
    pub route_id: usize,
    pub(crate) route_short_name: String,
    pub(crate) route_long_name: String,

    pub trips: Vec<usize>,
    //pub stops: Vec<usize>, we get the stops from a trip stop_times
    pub dataset_index: u64,
}

impl Route {
    // TODO: it shound be considered that a route might have several different trips
    pub fn get_stop_inx(&self, ds: &GtfsData, stop_id: usize) -> usize {
        assert_ne!(self.trips.len(), 0);
        let stop_times = ds.get_route_stop_times(self.route_id);
        stop_times.stop_times
            .iter()
            .map(|stop_time| stop_time.stop_id)
            .position(|s| s == stop_id)
            .expect(&format!("{} doesn't have {}", self.route_long_name, stop_id))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Trip {
    pub route_id: usize,
    pub trip_id: usize,
    pub shape_id: usize,
    pub stop_times_id: usize,
    // todo: this points to a vec<StopTime>
    pub start_time: i64, // in seconds since midnight. To get all stop times use stop_times_id and add the start time to each.

    pub(crate) service_id: String,
    pub(crate) trip_headsign: String,
    pub(crate) trip_short_name: String,
    pub(crate) direction_id: String,
    pub(crate) block_id: String,
    pub(crate) wheelchair_accessible: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct GtfsTime {
    timestamp: i64,
}

impl GtfsTime {
    pub fn new_from_midnight(time: i64) -> GtfsTime {
        let ts = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64;
        let sec_since_midnight = Utc.timestamp(ts as i64, 0).num_seconds_from_midnight() as i64;
        let last_midnight = ts - sec_since_midnight;

        GtfsTime {
            timestamp: last_midnight + time
        }
    }

    pub fn new_from_timestamp(timestamp: i64) -> GtfsTime {
        GtfsTime {
            timestamp
        }
    }


    fn date_time(&self) -> DateTime<Utc> {
        Utc.timestamp(self.timestamp, 0)
    }
    pub fn day_of_week(&self) -> u32 {
        self.date_time().weekday().num_days_from_monday()
    }
    pub fn h(&self) -> u32 {
        self.date_time().hour()
    }
    pub fn m(&self) -> u32 {
        self.date_time().minute()
    }
    pub fn s(&self) -> u32 {
        self.date_time().second()
    }
    pub fn since_midnight(&self) -> u32 {
        self.date_time().num_seconds_from_midnight()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Shape {
    pub(crate) shape_id: usize,
    pub(crate) points: Vec<LatLng>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Stop {
    pub stop_id: usize,
    pub stop_name: String,
    pub(crate) stop_pos: LatLng,
    pub(crate) stop_timezone: String,

    pub routes: BTreeSet<usize>,
}

pub fn to_coordinates(lat: &str, lng: &str) -> LatLng {
    //println!("lat {}, lng:{}",lat,lng);
    LatLng {
        lat: lat.parse::<f64>().unwrap_or(0.0),
        lng: lng.parse::<f64>().unwrap_or(0.0),
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct LatLng {
    pub lat: f64,
    pub lng: f64,
}

impl LatLng {
    fn as_point(&self) -> Point<f64> {
        Coordinate {
            x: self.lat,
            y: self.lng,
        }.into()
    }
}
