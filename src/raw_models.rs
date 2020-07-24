use std::fs::File;
use std::io::Error;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct RawAgency {
    pub agency_id: String,
    pub agency_name: String,
    pub agency_url: String,
    pub agency_timezone: String,
    pub agency_lang: String,
    pub agency_phone: String,
    pub agency_fare_url: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawCalendar {
    pub service_id: String,
    pub monday: String,
    pub tuesday: String,
    pub wednesday: String,
    pub thursday: String,
    pub friday: String,
    pub saturday: String,
    pub sunday: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawCalendarDates {
    pub service_id: String,
    pub date: String,
    pub exception_type: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawRoute {
    pub route_id: String,
    pub agency_id: String,
    pub route_short_name: String,
    pub route_long_name: String,
    pub route_desc: String,
    pub route_type: String,
    pub route_url: String,
    pub route_color: String,
    pub route_text_color: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawShape {
    pub shape_id: String,
    pub shape_pt_lat: String,
    pub shape_pt_lon: String,
    pub shape_pt_sequence: String,
    pub shape_dist_traveled: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawStopTime {
    pub trip_id: String,
    pub arrival_time: String,
    pub departure_time: String,
    pub stop_id: String,
    pub stop_sequence: String,
    pub stop_headsign: String,
    pub pickup_type: String,
    pub drop_off_type: String,
    pub shape_dist_traveled: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawStop {
    pub stop_id: String,
    pub stop_code: String,
    pub stop_name: String,
    pub stop_desc: String,
    pub stop_lat: String,
    pub stop_lon: String,
    pub zone_id: String,
    pub stop_url: String,
    pub location_type: String,
    pub parent_station: String,
    pub stop_timezone: String,
    pub wheelchair_boarding: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawTrip {
    pub route_id: String,
    pub service_id: String,
    pub trip_id: String,
    pub trip_headsign: String,
    pub trip_short_name: String,
    pub direction_id: String,
    pub block_id: String,
    pub shape_id: String,
    pub wheelchair_accessible: String,
}

pub fn parse_gtfs<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> Result<Vec<T>, Error> {
    let file = File::open(&path).expect("File not found during parsing!");
    Ok(csv::Reader::from_reader(file)
        .deserialize()
        .filter_map(Result::ok)
        .collect::<Vec<T>>())
}
