use std::sync::RwLock;

use chrono::NaiveDate;
use fastgtfs::gtfs_data::{GtfsData, GtfsTime, LatLng};
use fastgtfs::navigator::RaptorNavigator;
use fastgtfs::navigator_models::{NavigationParams, Solution};
use fastgtfs::raw_parser::RawParser;
use lazy_static::lazy_static;
use log::trace;
use wasm_bindgen::prelude::*;
use wasm_bindgen_console_logger::DEFAULT_LOGGER;

mod utils;

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

lazy_static! {
    static ref GTFS_DATASET: RwLock<GtfsData> = RwLock::new(Default::default());
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub async fn download_and_parse(file_url: String) {
    log::set_logger(&DEFAULT_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    trace!("downloading and parsing data");
    let data = RawParser::read_from_url(&file_url).await;
    trace!("data len: {:?}", data.stops.len());
    let mut datasets = GTFS_DATASET.write().unwrap();
    *datasets = data;
    trace!("global dataset set successfully");
}

#[wasm_bindgen]
pub fn get_stop_number() -> usize {
    let stop_number = GTFS_DATASET.read().unwrap().stops.len();
    trace!("Trying to get stop number from rust: {}", stop_number);
    stop_number
}

#[wasm_bindgen]
pub fn try_navigate() -> JsValue {
    trace!("Starting navigation from venice to nave de vero");

    let venice = LatLng {
        lat: 45.437_771_117_019_466,
        lng: 12.318_656_444_549_56,
    };
    let nave_de_vero = LatLng {
        lat: 45.459_262_090_230_05,
        lng: 12.212_569_713_592_53,
    };
    let solutions = get_solutions(venice.lat, venice.lng, nave_de_vero.lat, nave_de_vero.lng);
    trace!("solutions: {:?}", solutions);
    solutions
}

#[wasm_bindgen]
pub fn get_solutions(from_lat: f64, from_lng: f64, to_lat: f64, to_lng: f64) -> JsValue {
    let dataset = GTFS_DATASET.read().unwrap();

    let from = LatLng {
        lat: from_lat,
        lng: from_lng,
    };
    let to = LatLng {
        lat: to_lat,
        lng: to_lng,
    };

    trace!("Starting navigation");
    let solutions = navigate(&dataset, &from, &to);
    trace!("Found {} solutions", solutions.len());
    JsValue::from_serde(&solutions).unwrap()
}

fn navigate(dataset: &GtfsData, from: &LatLng, to: &LatLng) -> Vec<Solution> {
    let params = NavigationParams {
        from: from.clone(),
        to: to.clone(),
        max_changes: 4,
        start_time: default_start_time(),
        num_solutions_to_find: 3,
    };
    RaptorNavigator::navigate_blocking(&dataset, params)
}

fn default_start_time() -> GtfsTime {
    let start_timestamp = NaiveDate::from_ymd(2020, 8, 30)
        .and_hms(13, 30, 00)
        .timestamp();
    GtfsTime::new_from_timestamp(start_timestamp)
}
