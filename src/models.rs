use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::format;
use std::io::{Error, Seek, SeekFrom, Read};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use geo::Coordinate;
use itertools::{enumerate};
use log::error;

use std::path::Path;
use std::fs::File;

pub trait Searchable {
    fn inx_of(&self, s: &str) -> usize;
}

impl Searchable for Vec<&str> {
    fn inx_of(&self, s: &str) -> usize {
        match self.iter().position(|&r| r == s) {
            None => {
                error!("Missing field {} for string {}", s, self.join(","));
                self.len()
            }
            Some(s) => s,
        }
    }
}

#[derive(Debug, Default)]
pub struct Route {
    pub route_id: i64,
    pub(crate) route_short_name: String,
    pub(crate) route_long_name: String,

    //pub times_dt: Vec<u8>, // time in minutes between each stop
    pub trips: Vec<i64>, //trip ids
    pub dataset_index: u64,
    pub stops: Vec<i64>,
}

#[derive(Debug)]
pub struct Trip {
    pub route_id: i64,
    pub trip_id: i64,
    pub shape_id: i64,
    pub stop_times_id: i64, // todo: this points to a vec<StopTime>
    pub start_time : i64, // in seconds since midnight. To get all stop times use stop_times_id and add the start time to each.

    pub(crate) service_id: String,
    pub(crate) trip_headsign: String,
    pub(crate) trip_short_name: String,
    pub(crate) direction_id: String,
    pub(crate) block_id: String,
    pub(crate) wheelchair_accessible: String,

}


#[derive(Debug)]
pub struct Shape {
    pub(crate) shape_id: u64,
    pub(crate) points: Vec<Coordinate<f64>>,
}

#[derive(Debug)]
pub struct Stop {
    pub stop_id: i64,
    stop_name: String,
    stop_pos: Coordinate<f64>,
    stop_timezone: String,
}

impl Trip {
    /*pub fn parse_trips(s: &str, dataset_inx: u32) -> Vec<Trip> {
        let mut lines = s.split("\r\n");

        let fields = lines.next().unwrap().split(",").collect();
        let c = Trip::find_fields(fields);

        lines
            .filter(|l| l.len() > 0)
            .collect::<Vec<&str>>()
            .par_iter()
            .map(|l| {
                // the last is a fallback in case the field is not available
                let mut sp: Vec<&str> = l.split(",").collect::<Vec<&str>>();
                sp.push("");
                /*  let route_id = route_mappings
                .get(sp[c.route_id])
                .or(Some(&0))
                .unwrap()
                .to_owned(); */
                Trip {
                    fast_trip_id: -1,
                    fast_route_id: -1,
                    route_id: sp[c.route_id].to_string(),
                    service_id: sp[c.service_id].to_string(),
                    trip_id: sp[c.trip_id].to_string(),
                    trip_headsign: sp[c.trip_headsign].to_string(),
                    trip_short_name: sp[c.trip_short_name].to_string(),
                    direction_id: sp[c.direction_id].to_string(),
                    block_id: sp[c.block_id].to_string(),
                    shape_id: sp[c.shape_id].to_string(),
                    wheelchair_accessible: sp[c.wheelchair_accessible].to_string(),
                    stop_times_indexes: Default::default(),
                    dataset_index: dataset_inx,
                    stop_times: vec![]
                }
            })
            .collect()
    }*/

}


fn str_time_to_int(s: &str) -> i64 {
    // 05:00:00 -> 5 * 60 * 60
    return 42;
}
pub struct ParseRouteResult {
    pub(crate) routes: Vec<Route>,
    pub(crate) id_mapping: HashMap<String, u32>,
}

impl Route {
    /*fn parse_csv_line(l: &str, c: &RouteCorrespondence) -> Route {
        let mut sp: Vec<_> = l.split(",").collect();
        sp.push("");

        let route_id_str = sp[c.route_id].to_string();
        /*  let route_id = att_route_inx;
        id_mapping.insert(route_id_str, route_id);
        att_route_inx += 1; */
        Route {
            route_id: route_id_str,
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
    }*/
    /*pub fn parse_routes(s: &str, dataset_inx: u32) -> ParseRouteResult {
        let mut lines = s.split("\r\n");

        let fields = lines.next().unwrap().split(",").collect();
        let c = Route::find_fields(fields);
        let id_mapping: HashMap<String, u32> = HashMap::new();
        let routes = lines
            .filter(|l| l.len() > 0)
            .map(|l| {
                let mut r = Route::parse_csv_line(l, &c);
                r.dataset_index = dataset_inx;
                r
            })
            .collect::<Vec<Route>>();

        return ParseRouteResult { routes, id_mapping };
    }*/
}

struct ShapeInConsturction<'a> {
    id: String,
    shape_points_strings: Vec<&'a str>,
}

fn first_component(s: &str) -> &str {
    return &s[..s.find(',').unwrap_or(s.len())];
}

impl Shape {
    pub fn parse_shapes(s: &str) -> Vec<Shape> {
        vec![]
        // let mut lines = s.split("\r\n");
        //
        // let fields = lines.next().unwrap().split(",").collect();
        // let c = Shape::find_fields(fields);
        //
        // lines
        //     .into_iter()
        //     .filter(|l| l.len() > 0)
        //     .group_by(|l| first_component(l))
        //     .into_iter()
        //     .collect::<Vec<(&str, _)>>()
        //     .into_iter()
        //     .map(|(shape_id, vals)| ShapeInConsturction {
        //         id: shape_id.to_string(),
        //         shape_points_strings: vals.into_iter().collect::<Vec<&str>>(),
        //     })
        //     .collect::<Vec<ShapeInConsturction>>()
        //     .par_iter()
        //     .map(|sh| {
        //         let shape_id = String::from(&sh.id);
        //         Shape {
        //             shape_id,
        //             points: (&sh.shape_points_strings)
        //                 .into_iter()
        //                 .map(|l| l.split(',').collect())
        //                 .map(|v: Vec<&str>| to_coordinates(v[c.shape_pt_lat], v[c.shape_pt_lon]))
        //                 .collect(),
        //         }
        //     })
        //     .collect::<Vec<Shape>>()
        // .map(|shape| (shape.shape_id.to_owned(), shape))
        //.collect::<HashMap<String, Shape>>()

        //.map(|shape| (shape.shape_id.to_owned(), shape))
        //.collect::<HashMap<String, Shape>>()
    }
    /*fn _to_meters(km: &str) -> u64 {
        (km.parse::<f64>().unwrap() * 1000.0) as u64
    }*/
    /*fn find_fields(fields: Vec<&str>) -> ShapeCorrespondence {
        ShapeCorrespondence {
            shape_id: fields.inx_of("shape_id"),
            shape_pt_lat: fields.inx_of("shape_pt_lat"),
            shape_pt_lon: fields.inx_of("shape_pt_lon"),
            shape_pt_sequence: fields.inx_of("shape_pt_sequence"),
            shape_dist_traveled: fields.inx_of("shape_dist_traveled"),
        }
    }*/
}

pub fn to_coordinates(lat: &str, lng: &str) -> Coordinate<f64> {
    //println!("lat {}, lng:{}",lat,lng);
    Coordinate {
        x: lat.parse::<f64>().unwrap_or(0.0),
        y: lng.parse::<f64>().unwrap_or(0.0),
    }
}

impl Stop {

}
