use core::sync::atomic;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Error;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use log::{debug, error, info, trace, warn};

use crate::models::{GtfsData, ParseRouteResult, Route, Stop, Trip};
use crate::models::Shape;

static COUNTER: atomic::AtomicUsize = atomic::ATOMIC_USIZE_INIT;

struct Parser {
    path: String,
    inx: u32,
}

fn bump_counter() {
    // Add one using the most conservative ordering.
    COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
}

pub fn get_counter() -> usize {
    let res = COUNTER.load(atomic::Ordering::SeqCst);
    bump_counter();
    res
}
/*
let path = env::current_dir()?;
debug!("current path: {}", path.display());
*/
impl Parser {
    fn read_file(&self, file: &Path) -> Result<String, Error> {
        let path = Path::new(&self.path).join(Path::new(file));
        let mut file = File::open(&path)?;
        let mut s = String::new();
        file.read_to_string(&mut s)?;
        Ok(s)
    }

    pub(crate) fn read_routes(&self) -> Result<ParseRouteResult, Error> {
        let s = self.read_file(Path::new("routes.txt"))?;
        let res = Route::parse_routes(&s, self.inx);
        Ok(res)
    }

    pub(crate) fn read_trips(&self, route_mappings: HashMap<String, u32>) -> Result<Vec<Arc<Trip>>, Error> {
        let s = self.read_file(Path::new("trips.txt"))?;
        let now = Instant::now();
        let res = Trip::parse_trips(&s, route_mappings);
        debug!("trips time: {}", now.elapsed().as_millis());
        Ok(res)
    }

    pub fn read_shapes(&self) -> Result<Vec<Shape>, Error> {
        let s = self.read_file(Path::new("shapes.txt"))?;
        let now = Instant::now();
        let res = Shape::parse_shapes(&s);
        debug!("shape time: {}", now.elapsed().as_millis());
        Ok(res)
    }


    pub fn read_stops(&self) -> Result<Vec<Stop>, Error> {
        let s = self.read_file(Path::new("stops.txt"))?;
        let now = Instant::now();
        let res = Stop::parse_stops(&s);
        debug!("stops time: {}", now.elapsed().as_millis());
        Ok(res)
    }

    pub fn parse_all(&self) -> Result<GtfsData, Error> {
        let routes = self.read_routes()?;
        let trips: Vec<Arc<Trip>> = self.read_trips(routes.id_mapping)?;
        let shapes = self.read_shapes()?;
        let stops = self.read_stops()?;
        /* for trip in trips.iter() {
            routes.routes[trip.route_id as usize].trips.push(trip.clone());
        } */

        Ok(GtfsData {
            dataset_id: self.inx,
            routes: routes.routes,
            shapes,
            trips,
            stops,
        })
    }
}

pub fn parse_from_path<'a>(path: &String, dataset: &'a mut GtfsData) -> Result<&'a GtfsData, Error> {
    let parser = Parser { path: path.to_string(), inx: get_counter() as u32 + 1 };
    debug!("This parser inx: {}",parser.inx);
    let mut new_dataset: GtfsData = parser.parse_all()?;
    Ok(dataset.merge_dataset(&mut new_dataset))
}

pub fn parse_from_paths(paths: Vec<String>) -> GtfsData {
    let path = env::current_dir().unwrap();
    debug!("current path: {}", path.display());
    let mut dataset: GtfsData = Default::default();
    for path in paths {
        match parse_from_path(&path, &mut dataset) {
            Ok(_) => { debug!("{} Loaded sucessfully", path); }
            Err(e) => { error!("Error loading from {}: {}", path, e); }
        }
    }
    dataset
}

// redo indexing using integers instead of strings, and makes optimizations
pub fn finish_loading(datasets: &mut Vec<GtfsData>) -> GtfsData {
    let mut result: GtfsData = Default::default();

    //let mut biggest = datasets.iter().fold(&datasets[0], std::cmp::max);
    //datasets.remove(inx);
    for ds in datasets.iter_mut() {
        result.merge_dataset(ds);
    }
    result.do_postprocessing();
    trace!("Finish data loading in rust");
    result
}

