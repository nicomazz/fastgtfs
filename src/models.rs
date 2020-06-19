use std::collections::HashMap;
use std::fmt::format;

pub trait Searchable {
    fn inx_of(&self, s: &str) -> usize;
}

impl Searchable for Vec<&str> {
    fn inx_of(&self, s: &str) -> usize {
        self.iter().position(|&r| r == s).expect(&format!("Missing field: {}",s))
    }
}

#[derive(Debug)]
pub struct GtfsData {
    // pub calendar: HashMap<String, Calendar>,
    // pub calendar_dates: HashMap<String, Vec<CalendarDate>>,
    // pub stops: Vec<Stop>,
    pub routes: Vec<Route>,
    pub trips: Vec<Trip>,
    //pub agencies: Vec<Agency>,
    //pub shapes: Vec<Shape>,
}


#[derive(Debug)]
pub struct Trip {
    route_id: u32,
    service_id: String,
    trip_id: String,
    trip_headsign: String,
    trip_short_name: String,
    direction_id: String,
    block_id: String,
    shape_id: String,
    wheelchair_accessible: String,
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

impl Trip {

    pub fn parse_trips(s: &str, route_mappings: HashMap<String, u32>) -> Vec<Trip> {
        let mut lines = s.split("\r\n");

        let fields = lines.next().unwrap().split(",").collect();
        let c = Trip::find_fields(fields);

        lines
            .filter(|l| l.len() > 0)
            .map(|l| {
                let sp: Vec<_> = l.split(",").collect();
                let route_id = route_mappings.get(sp[c.route_id]).or(Some(&0)).unwrap().to_owned();
                Trip {
                    route_id,
                    service_id: sp[c.service_id].to_string(),
                    trip_id: sp[c.trip_id].to_string(),
                    trip_headsign: sp[c.trip_headsign].to_string(),
                    trip_short_name: sp[c.trip_short_name].to_string(),
                    direction_id: sp[c.direction_id].to_string(),
                    block_id: sp[c.block_id].to_string(),
                    shape_id: sp[c.shape_id].to_string(),
                    wheelchair_accessible: sp[c.wheelchair_accessible].to_string(),
                }
            }).collect()
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

#[derive(Debug)]
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
        let mut att_route_inx = 1;
        let routes = lines
            .filter(|l| l.len() > 0)
            .map(|l| {
                let sp: Vec<_> = l.split(",").collect();
                let route_id_str = sp[c.route_id].to_string();
                let route_id = id_mapping.entry(route_id_str).or_insert(att_route_inx).to_owned();
                att_route_inx += 1;
                Route {
                    route_id: route_id.to_owned(),
                    agency_id: sp[c.agency_id].to_string(),
                    route_short_name: sp[c.route_short_name].to_string(),
                    route_long_name: sp[c.route_long_name].to_string(),
                    route_desc: sp[c.route_desc].to_string(),
                    route_type: sp[c.route_type].to_string(),
                    route_url: sp[c.route_url].to_string(),
                    route_color: sp[c.route_color].to_string(),
                    route_text_color: sp[c.route_text_color].to_string(),
                }
            }).collect();
        return ParseRouteResult {
            routes,
            id_mapping,
        };
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