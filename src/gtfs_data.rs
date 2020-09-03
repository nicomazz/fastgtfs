extern crate flexbuffers;
extern crate serde;

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};
use std::fmt;
use std::time::SystemTime;

use cached::{
    proc_macro::cached,
    SizedCache,
};
use chrono::{Datelike, DateTime, NaiveDate, NaiveTime, Timelike, TimeZone, Utc};
use geo::{Coordinate, Point};
use geo::algorithm::euclidean_distance::EuclideanDistance;
use geo::algorithm::geodesic_distance::GeodesicDistance;
use itertools::Itertools;
use log::error;
use serde::{Deserialize, Serialize};

use self::serde::export::Formatter;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GtfsData {
    pub dataset_id: u32,
    // pub calendar: HashMap<String, Calendar>,
    // pub calendar_dates: HashMap<String, Vec<CalendarDate>>,
    pub routes: Vec<Route>,
    pub trips: Vec<Trip>,
    pub shapes: Vec<Shape>,
    pub stops: Vec<Stop>,
    pub services: Vec<Service>,
    pub stop_times: Vec<StopTimes>,

    pub walk_times: Vec<StopWalkTime>,
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

    pub fn get_near_stops_by_walk(&self, stop_id: usize) -> &StopWalkTime { &self.walk_times[stop_id] }

    pub fn get_service(&self, id: usize) -> &Service {
        &self.services[id]
    }

    pub fn get_route_stop_times(&self, route_id: usize) -> Vec<&StopTimes> {
        let route = self.get_route(route_id);
        route.stop_times.iter().map(|&st| self.get_stop_times(st)).collect()
    }

    /// returns the first trip that has `stop` (with inx after `start_stop_inx`) after time (not in excluded_trips)
    /// We make the `almost right` assumption that all the trips are from the same route id, and that they all share the same stop_time.
    /// This is true most of the times.
    /// TODO: handle this correctly
    pub fn trip_after_time(&self, trips: &[TripId], stop_id: StopId, min_time: &GtfsTime, start_stop_inx: StopIndex, banned_trip_ids: &HashSet<TripId>) -> Option<(&Trip, StopIndex)> {
        if trips.is_empty() { return None; }

        let first_trip = self.get_trip(*trips.first().unwrap());
        let stop_times = &self.get_stop_times(first_trip.stop_times_id).stop_times;
        let trips_duration = stop_times.last().unwrap().time;

        // indexes of the stops in `stop_times` matching `stop_id`
        let inxes_for_stop = stop_times
            .iter()
            .skip(start_stop_inx)
            .enumerate()
            .filter(|(_inx, stop_time)| stop_time.stop_id == stop_id)
            .map(|(inx, _)| inx)
            .collect::<Vec<usize>>();

        trips.iter()
            .filter(|t_id| !banned_trip_ids.contains(t_id))
            .map(|t_id| self.get_trip(*t_id))
            .filter(|t| self.trip_active_on_time(t, min_time, None) &&
                    t.start_time + trips_duration >= min_time.since_midnight() as i64)
            .find_map(|trip| { // this returns the first for which the content is an Ok result
                inxes_for_stop
                    .iter()
                    .filter(|&&inx| stop_times[inx].time + trip.start_time >= min_time.since_midnight() as i64)
                    .map(|inx| (trip, *inx))
                    .next()
            })
    }

    // todo: overoptimize
    pub fn find_nearest_stop(&self, pos: &LatLng) -> &Stop {
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

    pub fn trip_id_active_on_time(&self, trip_id: usize, day: &GtfsTime, within_hours: Option<i64>) -> bool {
        let trip = self.get_trip(trip_id);
        self.trip_active_on_time(trip, day, within_hours)
    }

    /// returns the number of seconds since midnight this trip departs and arrive
    fn get_trip_departure_arrival_times(&self, trip: &Trip) -> (i64, i64) {
        let stop_times = self.get_stop_times(trip.stop_times_id);
        let trip_duration = stop_times.stop_times.last().unwrap().time;
        (trip.start_time, trip.start_time + trip_duration)
    }
    /// returns true if this trip goes within `[time, time + within_hours]`,
    /// taking care of the service and its exceptions
    pub fn trip_active_on_time(&self, trip: &Trip, time: &GtfsTime, within_hours: Option<i64>) -> bool {
        if trip.service_id.is_none() {
            error!("Trip without service id! {}", trip.trip_short_name);
            return true;
        }
        let seconds_in_h = 60 * 60;
        let within_seconds = within_hours.unwrap_or(24) * seconds_in_h;
        let target_time = time.since_midnight() as i64;

        // It starts afterwards our window.
        if trip.start_time > target_time + within_seconds {
            return false;
        }
        let (_, arrival_time) = self.get_trip_departure_arrival_times(trip);
        // It finishes before our window
        if arrival_time < target_time {
            return false;
        }

        let service = self.get_service(trip.service_id.unwrap());

        for exception in &service.exceptions {
            if exception.date.is_same_day(&time) {
                return exception.running;
            }
        }
        service.days[time.day_of_week() as usize]
    }

    pub fn route_active_on_day(&self, route_id: usize, day: &GtfsTime) -> bool {
        let route = self.get_route(route_id);
        route.trips.iter().any(|&t| self.trip_id_active_on_time(t, &day, None))
    }

    pub fn trips_active_on_date_within_hours(&self, route_id: usize, time: &GtfsTime, within_h: i64) -> Vec<usize> {
        let route = self.get_route(route_id);
        route.trips
            .iter()
            .filter(|&&t|
                self.trip_id_active_on_time(t, time, Some(within_h)))
            .cloned()
            .collect::<Vec<usize>>()
    }
}

#[cached(
type = "SizedCache<(u64,u64, usize), Vec<usize>>",
create = "{ SizedCache::with_size(5000) }",
convert = r#"{ ((pos.lat * 1000.0) as u64 , (pos.lng * 1000.0) as u64,number) }"#
)]
pub fn near_stops(pos: &LatLng, number: usize, stops: &Vec<Stop>) -> Vec<usize> {
    let coord = pos.as_point();
    stops
        .iter()
        .sorted_by_key(|stop|
            (stop.stop_pos.as_point().euclidean_distance(&coord) * 1000.0) as i64)
        .take(number)
        .map(|stop| stop.stop_id)
        .collect::<Vec<usize>>()
}

// contains a list of stops, and the time for each in seconds (the first has time 0)
#[derive(Hash, Eq, Default, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct StopTimes {
    pub stop_times: Vec<StopTime>,
}

#[derive(Hash, Eq, PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct StopTime {
    pub stop_id: usize,
    pub time: i64, // in seconds
}

impl StopTime {
    pub fn time_from_offset(&self, trip_start_time: i64) -> i64 {
        self.time + trip_start_time
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

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Route {
    pub route_id: usize,
    pub route_short_name: String,
    pub route_long_name: String,

    pub trips: Vec<usize>,
    //pub stops: Vec<usize>, we get the stops from a trip stop_times
    pub dataset_index: u64,
    /// set of all the trips stop_times. Those are usually a very low number (<5?)
    pub stop_times: BTreeSet<usize>,
}

impl Route {
    // TODO: it should be considered that a route might have several different trips
    pub fn get_stop_inx(&self, ds: &GtfsData, stop_id: usize) -> Option<usize> {
        assert_ne!(self.trips.len(), 0);
        // TODO handle multiple stop times per route
        let stop_times = *ds.get_route_stop_times(self.route_id).first().unwrap();
        stop_times.stop_times
            .iter()
            .map(|stop_time| stop_time.stop_id)
            .position(|s| s == stop_id)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Trip {
    pub route_id: usize,
    pub trip_id: usize,
    pub shape_id: usize,
    // this points to a vec<StopTime>
    pub stop_times_id: usize,

    pub service_id: Option<usize>,
    pub start_time: i64, // in seconds since midnight. To get all stop times use stop_times_id and add the start time to each.

    pub trip_headsign: String,
    pub trip_short_name: String,
    pub direction_id: String,
    pub block_id: String,
    pub wheelchair_accessible: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Eq, Ord, PartialEq, PartialOrd)]
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

    pub fn new_replacing_time(&self, seconds_from_midnight: i64) -> GtfsTime {
        GtfsTime {
            timestamp: self.timestamp - (self.since_midnight() as i64) + seconds_from_midnight
        }
    }
    pub fn new_from_timestamp(timestamp: i64) -> GtfsTime { GtfsTime { timestamp } }

    pub fn new_infinite() -> GtfsTime {
        GtfsTime::new_from_timestamp(32503680000) // First January 3000
    }

    pub fn from_date(yyyymmdd: &String) -> GtfsTime {
        let date = NaiveDate::parse_from_str(yyyymmdd, "%Y%m%d").unwrap();
        let date = Utc.from_utc_date(&date).and_time(NaiveTime::from_hms(0, 0, 0)).unwrap();
        GtfsTime::new_from_timestamp(date.timestamp())
    }

    pub fn set_day_from(&mut self, other: &GtfsTime) {
        let since_midnight = self.since_midnight();
        self.timestamp = (other.timestamp as u64 - other.since_midnight() + since_midnight) as i64;
    }

    pub fn add_seconds(&mut self, sec: u64) -> &GtfsTime{
        self.timestamp += sec as i64;
        self
    }
    fn date_time(&self) -> DateTime<Utc> {
        Utc.timestamp(self.timestamp, 0)
    }
    pub fn is_same_day(&self, other: &GtfsTime) -> bool {
        self.date_time().num_days_from_ce() == other.date_time().num_days_from_ce()
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
    pub fn since_midnight(&self) -> u64 {
        self.date_time().num_seconds_from_midnight() as u64
    }

    pub fn distance(&self, other: &GtfsTime) -> u64 {
        (self.timestamp - other.timestamp).abs() as u64
    }
    pub fn timestamp(&self) -> i64 { self.timestamp }
}

impl fmt::Display for GtfsTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let t = Utc.timestamp(self.timestamp as i64, 0);
        write!(f, "{}", t.to_rfc2822())
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
    pub stop_pos: LatLng,
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
pub struct Service {
    pub service_id: usize,
    pub days: Vec<bool>,
    pub start_date: GtfsTime,
    pub end_date: GtfsTime,
    pub exceptions: Vec<ServiceException>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ServiceException {
    pub date: GtfsTime,
    pub running: bool,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct LatLng {
    pub lat: f64,
    pub lng: f64,
}

impl LatLng {
    pub(crate) fn as_point(&self) -> Point<f64> {
        Coordinate {
            x: self.lat,
            y: self.lng,
        }.into()
    }
    pub fn distance_meters(&self, other: &LatLng) -> u64 {
        self.as_point().geodesic_distance(&other.as_point()) as u64
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct StopWalkTime {
    pub stop_id: usize,
    pub near_stops: Vec<StopDistance>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct StopDistance {
    pub stop_id: usize,
    pub distance_meters: usize,
}

pub type RouteId = usize;
pub type TripId = usize;
pub type StopId = usize;
pub type StopTimesId = usize;
pub type StopIndex = usize;
