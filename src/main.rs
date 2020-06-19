mod models;

use std::path::Path;
use std::fs::{File, read};
use std::io::{Read, Error};
use std::env;

mod lib;

#[derive(Debug)]
struct GtfsData {
    // pub calendar: HashMap<String, Calendar>,
    // pub calendar_dates: HashMap<String, Vec<CalendarDate>>,
    // pub stops: Vec<Stop>,
    //pub routes: Vec<Route>,
    pub trips: Vec<Trip>,
    //pub agencies: Vec<Agency>,
    //pub shapes: Vec<Shape>,
}

#[derive(Debug)]
struct Trip {
    route_id: String,
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
    route_id: i8,
    service_id: i8,
    trip_id: i8,
    trip_headsign: i8,
    trip_short_name: i8,
    direction_id: i8,
    block_id: i8,
    shape_id: i8,
    wheelchair_accessible: i8,
}

impl Trip {
    fn init(csv_line: &str, c: &TripCorrespondence) -> Trip {
        assert_eq!(csv_line, "route_id,service_id,trip_id,trip_headsign,trip_short_name,direction_id,block_id,shape_id,wheelchair_accessible");
        let sp : Vec<_> = csv_line.split(",").collect();
        Trip {
            route_id: String::sp[c.route_id].clone(),
            service_id: sp[c.service_id],
            trip_id: sp[c.trip_id],
            trip_headsign: sp[c.trip_headsign],
            trip_short_name: sp[c.trip_short_name],
            direction_id: sp[c.direction_id],
            block_id: sp[c.block_id],
            shape_id: sp[c.shape_id],
            wheelchair_accessible: sp[c.wheelchair_accessible],
        }
    }
}

pub trait Searchable {
    fn inx_of(&self, s: &str) -> i8;
}

impl Searchable for Vec<&str> {
    fn inx_of(&self, s: &str) -> i8 {
        return self.iter().position(|&r| r == s).unwrap() as i8;
    }
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

fn read_trips(dataset: &GtfsData) -> Result<Vec<&'static Trip>, Error> {
    println!("read trips");
    let path = env::current_dir()?;
    println!("current path: {}", path.display());

    let path = Path::new("test_data/trips.txt");

    let mut file = File::open(&path).expect("Can't open trips.txt");

    let mut s = String::new();

    file.read_to_string(&mut s).expect("Can't read trips.txt");
    let lines: Vec<&str> = s.split("\r\n").collect();
    println!("len: {}", lines.len());
    let fields = lines[0].split(",");
    let mapping = find_fields(fields);
    let result: Vec<Trip> = lines[1..].iter().map(|l| Trip::init(l, mapping)).collect();

    Ok(result)
}

/*fn test(){

}*/

fn main() {
    let gtfsDataSet = GtfsData::new();
    gtfsDataSet.trips = read_trips(gtfsDataSet);
    println!("{}", gtfsDataSet.trips);
}