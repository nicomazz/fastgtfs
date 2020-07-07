use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::format;
use std::io::{Error, Seek, SeekFrom, Read};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use geo::Coordinate;
use itertools::{enumerate, Itertools};
use log::error;
use rayon::prelude::*;
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
    pub fast_id: i32,

    pub route_id: String,
    agency_id: String,
    route_short_name: String,
    route_long_name: String,
    route_desc: String,
    route_type: String,
    route_url: String,
    route_color: String,
    route_text_color: String,
    pub trips: Vec<i64>, //trip ids

    pub dataset_index: u32,
    pub stops: Vec<i32>,
    pub times_dt: Vec<u8>, // time in minutes between each stop
}

#[derive(Debug)]
pub struct Trip {
    fast_route_id: i64,
    pub(crate) fast_trip_id: i64,

    pub(crate) route_id: String,
    service_id: String,
    pub trip_id: String,
    trip_headsign: String,
    trip_short_name: String,
    direction_id: String,
    block_id: String,
    shape_id: String,
    wheelchair_accessible: String,
    //  stop_times_cursor : u64

    pub stop_times_indexes: TripStopInfo,
    pub dataset_index: u32,
    pub stop_times: Vec<StopTime>,
}

// this struct defines where to find the info into the stop_times.txt
#[derive(Debug, Default, Copy, Clone)]
pub struct TripStopInfo {
    start: usize,
    end: usize,
    pub size: u16,
}

#[derive(Debug)]
pub struct StopTime {
    pub(crate) stop_id: String,
    pub(crate) time: i64,
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
    points: Vec<Coordinate<f64>>,
}

#[derive(Debug)]
pub struct Stop {
    pub(crate) fast_id: u64,

    pub(crate) stop_id: String,
    stop_code: String,
    stop_name: String,
    stop_desc: String,
    stop_pos: Coordinate<f64>,
    zone_id: String,
    stop_url: String,
    location_type: String,
    parent_station: String,
    stop_timezone: String,
    wheelchair_boarding: String,
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
// stop_id,stop_code,stop_name,stop_desc,stop_lat,stop_lon,zone_id,stop_url,location_type,parent_station,stop_timezone,wheelchair_boarding

struct StopCorrespondence {
    stop_id: usize,
    stop_code: usize,
    stop_name: usize,
    stop_desc: usize,
    stop_lat: usize,
    stop_lon: usize,
    zone_id: usize,
    stop_url: usize,
    location_type: usize,
    parent_station: usize,
    stop_timezone: usize,
    wheelchair_boarding: usize,
}

impl Trip {
    pub fn parse_trips(s: &str, dataset_inx: u32) -> Vec<Trip> {
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

    pub(crate) fn get_stop_times(&self) -> Vec<StopTime> {
        vec![]
    //     let path = Path::new("stop_times.txt");
    //     let mut file =  File::open(&path)?;
    //     file.seek(SeekFrom::start(&self.stop_times_indexes.start));
    //     let indexes = &self.stop_times_indexes;
    //     let mut buffer = vec![0; indexes.end-indexes.start];
    //     ///
    //     ///     // read exactly 10 bytes
    //     ///     f.read_exact(&mut buffer)?;
    //     let readed = file.read_exact(buffer.as_mut_slice());
    //
    //     readed.split("\r\n").map(|s| StopTime::parse_line(s)).collect()
    //
    }
}

struct StopTimesCorrespondence {
    trip_id: usize,
    arrival_time: usize,
    departure_time: usize,
    stop_id: usize,
    stop_sequence: usize,
    stop_headsign: usize,
    pickup_type: usize,
    drop_off_type: usize,
    shape_dist_traveled: usize,
}

impl StopTime {
    pub fn parse_stop_times(s: &str) -> HashMap<String, TripStopInfo> {
        let mut lines = s.split("\r\n");

        const NEW_LINE_SIZE: usize = 2;
        let mut cursor = 0;

        let first_line = lines.next().unwrap();
        let fields = first_line.split(",").collect();
        let c = Trip::find_fields(fields);

        cursor += first_line.len() + NEW_LINE_SIZE;

        let mut res = HashMap::<String, TripStopInfo>::new();
        let mut last_trip_id = String::from("");
        let mut last_start = cursor;
        let mut last_size = 0;
        let mut this_trip_id = "";
        for line in lines {
            this_trip_id = first_component(line);
            if this_trip_id != last_trip_id {
                res.insert(
                    this_trip_id.to_string(),
                    TripStopInfo {
                        start: last_start,
                        end: cursor,
                        size: last_size,
                    },
                );
                last_start = cursor;
                last_size = 0;
                last_trip_id = this_trip_id.to_string();
            }
            last_size += 1;
            cursor += line.len() + NEW_LINE_SIZE;
        }
        res.insert(
            this_trip_id.to_string(),
            TripStopInfo {
                start: last_start,
                end: cursor,
                size: last_size,
            },
        );

        res
    }

    fn find_fields(fields: Vec<&str>) -> StopTimesCorrespondence {
        StopTimesCorrespondence {
            trip_id: fields.inx_of("trip_id"),
            arrival_time: fields.inx_of("arrival_time"),
            departure_time: fields.inx_of("departure_time"),
            stop_id: fields.inx_of("stop_id"),
            stop_sequence: fields.inx_of("stop_sequence"),
            stop_headsign: fields.inx_of("stop_headsign"),
            pickup_type: fields.inx_of("pickup_type"),
            drop_off_type: fields.inx_of("drop_off_type"),
            shape_dist_traveled: fields.inx_of("shape_dist_traveled"),
        }
    }

    fn parse_line(line : &str) -> StopTime {
      //  trip_id,arrival_time,departure_time,stop_id,stop_sequence,stop_headsign,pickup_type,drop_off_type,shape_dist_traveled
      //  12661,05:00:00,05:00:00,723,1,VENEZIA,0,1,
        // TODO make this aware of the dataset, without hardcoded indexes
        //let components = line.split(",").collect();
        StopTime {
            stop_id: "".to_string(),//components.get(3).unwrap().to_string(),
            time: 0,//str_time_to_int(components.get(2)),
        }
    }
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
    fn parse_csv_line(l: &str, c: &RouteCorrespondence) -> Route {
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
    }
    pub fn parse_routes(s: &str, dataset_inx: u32) -> ParseRouteResult {
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

struct ShapeInConsturction<'a> {
    id: String,
    shape_points_strings: Vec<&'a str>,
}

fn first_component(s: &str) -> &str {
    return &s[..s.find(',').unwrap_or(s.len())];
}

impl Shape {
    pub fn parse_shapes(s: &str) -> Vec<Shape> {
        let mut lines = s.split("\r\n");

        let fields = lines.next().unwrap().split(",").collect();
        let c = Shape::find_fields(fields);

        lines
            .into_iter()
            .filter(|l| l.len() > 0)
            .group_by(|l| first_component(l))
            .into_iter()
            .collect::<Vec<(&str, _)>>()
            .into_iter()
            .map(|(shape_id, vals)| ShapeInConsturction {
                id: shape_id.to_string(),
                shape_points_strings: vals.into_iter().collect::<Vec<&str>>(),
            })
            .collect::<Vec<ShapeInConsturction>>()
            .par_iter()
            .map(|sh| {
                let shape_id = String::from(&sh.id);
                Shape {
                    shape_id,
                    points: (&sh.shape_points_strings)
                        .into_iter()
                        .map(|l| l.split(',').collect())
                        .map(|v: Vec<&str>| to_coordinates(v[c.shape_pt_lat], v[c.shape_pt_lon]))
                        .collect(),
                }
            })
            .collect::<Vec<Shape>>()
        // .map(|shape| (shape.shape_id.to_owned(), shape))
        //.collect::<HashMap<String, Shape>>()

        //.map(|shape| (shape.shape_id.to_owned(), shape))
        //.collect::<HashMap<String, Shape>>()
    }
    fn _to_meters(km: &str) -> u64 {
        (km.parse::<f64>().unwrap() * 1000.0) as u64
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

fn to_coordinates(lat: &str, lng: &str) -> Coordinate<f64> {
    //println!("lat {}, lng:{}",lat,lng);
    Coordinate {
        x: lat.parse::<f64>().unwrap_or(0.0),
        y: lng.parse::<f64>().unwrap_or(0.0),
    }
}

impl Stop {
    fn parse_csv_line(l: &str, c: &StopCorrespondence) -> Stop {
        let mut v = l.split(',').collect::<Vec<&str>>();
        v.push("");

        Stop {
            stop_id: v[c.stop_id].to_string(),
            stop_code: v[c.stop_code].to_string(),
            stop_name: v[c.stop_name].to_string(),
            stop_desc: v[c.stop_desc].to_string(),
            stop_pos: to_coordinates(v[c.stop_lat], v[c.stop_lon]),
            zone_id: v[c.zone_id].to_string(),
            stop_url: v[c.stop_url].to_string(),
            location_type: v[c.location_type].to_string(),
            parent_station: v[c.parent_station].to_string(),
            stop_timezone: v[c.stop_timezone].to_string(),
            wheelchair_boarding: v[c.wheelchair_boarding].to_string(),
            fast_id: 0,
        }
    }
    pub fn parse_stops(s: &str) -> Vec<Stop> {
        let mut lines = s.split("\r\n");

        let fields = lines.next().unwrap().split(",").collect();
        let c = Stop::find_fields(fields);

        lines
            .collect::<Vec<&str>>()
            .par_iter()
            .filter(|l| l.len() > 0)
            .map(|l| Stop::parse_csv_line(l, &c))
            .collect()
    }

    fn find_fields(fields: Vec<&str>) -> StopCorrespondence {
        StopCorrespondence {
            stop_id: fields.inx_of("stop_id"),
            stop_code: fields.inx_of("stop_code"),
            stop_name: fields.inx_of("stop_name"),
            stop_desc: fields.inx_of("stop_desc"),
            stop_lat: fields.inx_of("stop_lat"),
            stop_lon: fields.inx_of("stop_lon"),
            zone_id: fields.inx_of("zone_id"),
            stop_url: fields.inx_of("stop_url"),
            location_type: fields.inx_of("location_type"),
            parent_station: fields.inx_of("parent_station"),
            stop_timezone: fields.inx_of("stop_timezone"),
            wheelchair_boarding: fields.inx_of("wheelchair_boarding"),
        }
    }
}
