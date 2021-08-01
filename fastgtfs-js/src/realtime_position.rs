use std::collections::HashMap;
use std::sync::RwLock;

use lazy_static::lazy_static;
use wasm_bindgen::prelude::*;

use fastgtfs::gtfs_data::TripId;
use fastgtfs::gtfs_data::{GtfsTime, LatLng, Trip};
use fastgtfs::realtime_position::TripRealTimePositionData;

use super::GTFS_DATASET;

lazy_static! {
    static ref REALTIME_POSITION_HANDLER: RwLock<HashMap<TripId, TripRealTimePositionData>> =
        RwLock::new(Default::default());
    static ref REALTIME_POSITION_REFERENCE_COUNT: RwLock<HashMap<TripId, i32>> =
        RwLock::new(Default::default());
}

#[wasm_bindgen]
pub fn get_near_trips(
    lat: f64,
    lng: f64,
    seconds_since_midnight: i32,
    date: &str,
    number: usize,
) -> JsValue {
    let start_day = GtfsTime::from_date(&date);

    let mut start_date = GtfsTime::new_from_midnight(seconds_since_midnight as i64);
    start_date.set_day_from(&start_day);
    let position = LatLng { lat, lng };

    let ds = GTFS_DATASET.read().unwrap();
    let trips: Vec<&Trip> =
        ds.get_near_trips_near_stops(&start_date, &position, number, 10_000_000);
    let res: Vec<usize> = trips
        .iter()
        .flat_map(|t| vec![t.route_id, t.trip_id])
        .collect();
    JsValue::from_serde(&res).unwrap()
}

#[wasm_bindgen]
pub fn init_trip_position_in_real_time(trip_id: TripId) -> i32 {
    let trip_id = trip_id as usize;
    let ds = GTFS_DATASET.read().unwrap();
    let mut map = REALTIME_POSITION_HANDLER.write().unwrap();
    let mut reference_count = REALTIME_POSITION_REFERENCE_COUNT.write().unwrap();
    map.entry(trip_id)
        .or_insert_with(|| TripRealTimePositionData::new(&ds, trip_id));
    *reference_count.entry(trip_id).or_insert(0) += 1;
    ds.get_trip(trip_id).start_time as i32
}

#[wasm_bindgen]
pub fn delete_trip_position_in_real_time(trip_id: TripId) {
    let trip_id = trip_id as usize;

    let mut reference_count = REALTIME_POSITION_REFERENCE_COUNT.write().unwrap();
    *reference_count.entry(trip_id).or_insert(1) -= 1;

    if reference_count[&trip_id] == 0 {
        let mut map = REALTIME_POSITION_HANDLER.write().unwrap();
        map.remove(&trip_id);
    }
}

#[wasm_bindgen]
pub fn get_trip_position(trip_id: TripId, seconds_since_started: i32) -> JsValue {
    let map = REALTIME_POSITION_HANDLER.read().unwrap();
    let handler = map
        .get(&(trip_id as usize))
        .expect("Querying for a Trip id not inserted in real time position handler");
    let pos = handler.get_position(seconds_since_started as i64);
    JsValue::from_serde(&pos).unwrap()
}
