use crate::models::Shape;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Error;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::time::{Instant};


use crate::models::{GtfsData, ParseRouteResult, Route, Trip,Stop};

pub(crate) fn read_routes() -> Result<ParseRouteResult, Error> {
    let path = env::current_dir()?;
    println!("current path: {}", path.display());

    let path = Path::new("test_data/routes.txt");
    let mut file = File::open(&path).expect("Can't open trips.txt");
    let mut s = String::new();

    file.read_to_string(&mut s).expect("Can't read trips.txt");

    let res = Route::parse_routes(s.as_ref());
    Ok(res)
}

pub(crate) fn read_trips(route_mappings: HashMap<String, u32>) -> Result<Vec<Arc<Trip>>, Error> {
    println!("read trips");
    let path = env::current_dir()?;
    println!("current path: {}", path.display());

    let path = Path::new("test_data/trips.txt");

    let mut file = File::open(&path).expect("Can't open trips.txt");

    let mut s = String::new();

    let now = Instant::now();
    file.read_to_string(&mut s).expect("Can't read trips.txt");

    let res = Trip::parse_trips(s.as_ref(), route_mappings);
    println!("trips time: {}", now.elapsed().as_millis());

    Ok(res)
}

pub fn read_shapes() -> Result<HashMap<String,Shape>,Error>{
    let path = Path::new("test_data/shapes.txt");

    let mut file = File::open(&path).expect("Can't open trips.txt");

    let mut s = String::new();
    let now = Instant::now();

    file.read_to_string(&mut s).expect("Can't read shapes.txt");
    println!("read string time: {}", now.elapsed().as_millis());

    let res = Shape::parse_shapes(s.as_ref());
    println!("shape time: {}", now.elapsed().as_millis());

    Ok(res)
}


pub fn read_stops() -> Result<Vec<Stop>, Error> {
    let path = Path::new("test_data/stops.txt");

    let mut file = File::open(&path).expect("Can't open stops.txt");

    let mut s = String::new();
    let now = Instant::now();

    file.read_to_string(&mut s).expect("Can't read stops.txt");
    println!("read stop time: {}", now.elapsed().as_millis());

    let res = Stop::parse_stops(s.as_ref());
    println!("parse stop time: {}", now.elapsed().as_millis());

    Ok(res)
} 

/* pub fn parse_from_path(path: &Path, dataset: GtfsData) -> GtfsData{
    let mut routes= read_routes().unwrap();
    let trips : Vec<Arc<Trip>> = read_trips(routes.id_mapping).unwrap();
    let shapes = read_shapes().unwrap();
    let stops = read_stops().unwrap();
}  */
pub fn parse_all() -> GtfsData {
    let mut routes= read_routes().unwrap();
    let trips : Vec<Arc<Trip>> = read_trips(routes.id_mapping).unwrap();
    let shapes = read_shapes().unwrap();
    let stops = read_stops().unwrap();
    /* for trip in trips.iter() {
        routes.routes[trip.route_id as usize].trips.push(trip.clone());
    } */

    GtfsData {
        routes: routes.routes,
        shapes,
        trips,
        stops
    }
   // println!("{:#?}", gtfs_data_set.trips.iter().take(10));
    //println!("{:#?}", gtfs_data_set.routes.iter().take(10));
    //println!("{:#?}", gtfs_data_set.trips);
}

