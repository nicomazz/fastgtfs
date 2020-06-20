use geo::Coordinate;
use std::collections::HashMap;
use std::fmt::format;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use rayon::prelude::*;

use itertools::Itertools;

pub trait Searchable {
    fn inx_of(&self, s: &str) -> usize;
}

impl Searchable for Vec<&str> {
    fn inx_of(&self, s: &str) -> usize {
        self.iter()
            .position(|&r| r == s)
            .expect(&format!("Missing field: {}", s))
    }
}

#[derive(Debug, Default)]
pub struct GtfsData {
    // pub calendar: HashMap<String, Calendar>,
    // pub calendar_dates: HashMap<String, Vec<CalendarDate>>,
    // pub stops: Vec<Stop>,
    pub routes: Vec<Route>,
    pub trips: Vec<Arc<Trip>>,
    pub shapes: HashMap<String, Shape>,
    //pub agencies: Vec<Agency>,
    //pub shapes: Vec<Shape>,
}
#[derive(Debug, Default)]
pub struct Route {
    route_id: u32,
    agency_id: String,
    route_short_name: String,
    route_long_name: String,
    route_desc: String,
    route_type: String,
    route_url: String,
    route_color: String,
    route_text_color: String,
    pub trips: Vec<Arc<Trip>>,
}

#[derive(Debug)]
pub struct Trip {
    pub route_id: u32,
    service_id: String,
    trip_id: String,
    trip_headsign: String,
    trip_short_name: String,
    direction_id: String,
    block_id: String,
    shape_id: String,
    wheelchair_accessible: String,
}
#[derive(Debug)]
pub struct Shape {
    /*
        shape_id,shape_pt_lat,shape_pt_lon,shape_pt_sequence,shape_dist_traveled
    1_0_1,45.417561,12.368731,0,0.0
    1_0_1,45.417545,12.368747,1,0.0021125906752124325
    1_0_1,45.417423,12.368521,2,0.02454935679179161
     */
    shape_id: String,
    points: Vec<Coordinate<f32>>,
   // dist_traveled: Vec<u64>, //meters
}

struct RouteCorrespondence {
    route_id: usize,
    agency_id: usize,
    route_short_name: usize,
    route_long_name: usize,
    route_desc: usize,
    route_type: usize,
    route_url: usize,
    route_color: usize,
    route_text_color: usize,
}

struct TripCorrespondence {
    route_id: usize,
    service_id: usize,
    trip_id: usize,
    trip_headsign: usize,
    trip_short_name: usize,
    direction_id: usize,
    block_id: usize,
    shape_id: usize,
    wheelchair_accessible: usize,
}

struct ShapeCorrespondence {
    shape_id: usize,
    shape_pt_lat: usize,
    shape_pt_lon: usize,
    shape_pt_sequence: usize,
    shape_dist_traveled: usize,
}

impl Trip {
    pub fn parse_trips(s: &str, route_mappings: HashMap<String, u32>) -> Vec<Arc<Trip>> {
        let mut lines = s.split("\r\n");

        let fields = lines.next().unwrap().split(",").collect();
        let c = Trip::find_fields(fields);

        lines
            .filter(|l| l.len() > 0)
            .map(|l| {
                let sp: Vec<_> = l.split(",").collect();
                let route_id = route_mappings
                    .get(sp[c.route_id])
                    .or(Some(&0))
                    .unwrap()
                    .to_owned();
                Arc::new(Trip {
                    route_id,
                    service_id: sp[c.service_id].to_string(),
                    trip_id: sp[c.trip_id].to_string(),
                    trip_headsign: sp[c.trip_headsign].to_string(),
                    trip_short_name: sp[c.trip_short_name].to_string(),
                    direction_id: sp[c.direction_id].to_string(),
                    block_id: sp[c.block_id].to_string(),
                    shape_id: sp[c.shape_id].to_string(),
                    wheelchair_accessible: sp[c.wheelchair_accessible].to_string(),
                })
            })
            .collect()
    }

    fn find_fields(fields: Vec<&str>) -> TripCorrespondence {
        TripCorrespondence {
            route_id: fields.inx_of("route_id"),
            service_id: fields.inx_of("service_id"),
            trip_id: fields.inx_of("trip_id"),
            trip_headsign: fields.inx_of("trip_headsign"),
            trip_short_name: fields.inx_of("trip_short_name"),
            direction_id: fields.inx_of("direction_id"),
            block_id: fields.inx_of("block_id"),
            shape_id: fields.inx_of("shape_id"),
            wheelchair_accessible: fields.inx_of("wheelchair_accessible"),
        }
    }
}

pub struct ParseRouteResult {
    pub(crate) routes: Vec<Route>,
    pub(crate) id_mapping: HashMap<String, u32>,
}

impl Route {
    pub fn parse_routes(s: &str) -> ParseRouteResult {
        let mut lines = s.split("\r\n");

        let fields = lines.next().unwrap().split(",").collect();
        let c = Route::find_fields(fields);
        let mut id_mapping: HashMap<String, u32> = HashMap::new();
        let mut att_route_inx = 0;
        let routes = lines
            .filter(|l| l.len() > 0)
            .map(|l| {
                let sp: Vec<_> = l.split(",").collect();
                let route_id_str = sp[c.route_id].to_string();
                let route_id = att_route_inx;
                id_mapping.insert(route_id_str, route_id);
                att_route_inx += 1;
                Route {
                    route_id,
                    agency_id: sp[c.agency_id].to_string(),
                    route_short_name: sp[c.route_short_name].to_string(),
                    route_long_name: sp[c.route_long_name].to_string(),
                    route_desc: sp[c.route_desc].to_string(),
                    route_type: sp[c.route_type].to_string(),
                    route_url: sp[c.route_url].to_string(),
                    route_color: sp[c.route_color].to_string(),
                    route_text_color: sp[c.route_text_color].to_string(),
                    ..Default::default()
                }
            })
            .collect();
        return ParseRouteResult { routes, id_mapping };
    }

    fn find_fields(fields: Vec<&str>) -> RouteCorrespondence {
        RouteCorrespondence {
            route_id: fields.inx_of("route_id"),
            agency_id: fields.inx_of("agency_id"),
            route_short_name: fields.inx_of("route_short_name"),
            route_long_name: fields.inx_of("route_long_name"),
            route_desc: fields.inx_of("route_desc"),
            route_type: fields.inx_of("route_type"),
            route_url: fields.inx_of("route_url"),
            route_color: fields.inx_of("route_color"),
            route_text_color: fields.inx_of("route_text_color"),
        }
    }
}


impl Shape {
    pub fn parse_shapes(s: &str) -> HashMap<String, Shape> {
        let mut lines = s.split("\r\n");

        let fields = lines.next().unwrap().split(",").collect();
        let c = Shape::find_fields(fields);

        lines
            .into_iter()
            .filter(|l| l.len() > 0)
            .map(|l| l.split(",").collect())
            .group_by(|v: &Vec<&str>| v[c.shape_id])
            .into_iter()
            .map(|(shape_id, vals)| Shape {
                shape_id: shape_id.to_string(),
                points: vals
                    .into_iter()
                    .map(|v| Shape::to_coordinates( 
                        v[c.shape_pt_lat], v[c.shape_pt_lon])
                    )
                    .collect(),
            })
            .map(|shape| (shape.shape_id.to_owned(), shape))
            .collect::<HashMap<String, Shape>>()
    }
    fn to_meters(km: &str) -> u64  {
        (km.parse::<f64>().unwrap() * 1000.0) as u64
    }
    fn to_coordinates(lat: &str, lng: &str) -> Coordinate<f32> {
        Coordinate{
            x: lat.parse::<f32>().unwrap() ,
            y: lng.parse::<f32>().unwrap() 
        }
    }
    fn find_fields(fields: Vec<&str>) -> ShapeCorrespondence {
        ShapeCorrespondence {
            shape_id: fields.inx_of("shape_id"),
            shape_pt_lat: fields.inx_of("shape_pt_lat"),
            shape_pt_lon: fields.inx_of("shape_pt_lon"),
            shape_pt_sequence: fields.inx_of("shape_pt_sequence"),
            shape_dist_traveled: fields.inx_of("shape_dist_traveled"),
        }
    }
}
