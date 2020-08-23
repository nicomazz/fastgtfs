use std::fmt;

use crate::gtfs_data::{GtfsTime, LatLng, Route, Trip};

#[derive(Debug, Clone, Default)]
pub struct NavigationParams {
    pub from: LatLng,
    pub to: LatLng,
    pub max_changes: u8,
    pub start_time: GtfsTime,
    //pub sol_callback: Box<dyn Fn(Solution)>,
}

#[derive(Debug, Default)]
pub struct Solution {
    pub start_time: GtfsTime,
    pub duration_seconds: usize,
    pub components: Vec<SolutionComponent>,
}

impl fmt::Display for Solution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\n####### Solution components: {}", self.components.len()).unwrap();
        for c in &self.components {
            write!(f, "{}", c).unwrap();
        }
        write!(f, "####### ")
    }
}


impl Solution {
    pub(crate) fn set_last_component_start(&mut self, stop_id: usize) {
        if let Some(last) = self.components.last_mut() {
            if let SolutionComponent::Walk(w) = last {
                w.stop_id = stop_id
            }
        }
    }
    pub(crate) fn add_walking_path(&mut self, stop_id: usize) {
        let component = WalkSolutionComponent { stop_id };
        self.set_last_component_start(stop_id);
        self.components.push(SolutionComponent::Walk(component));
    }

    pub(crate) fn add_bus_path(&mut self, stop_id: usize, route: &Route, trip: &Trip, from_inx: usize,
                               to_inx: usize) {
        let component = BusSolutionComponent {
            route: route.clone(),
            trip: trip.clone(),
            from_inx: Some(from_inx),
            to_inx: Some(to_inx),
        };
        self.set_last_component_start(stop_id);
        self.components.push(SolutionComponent::Bus(component));
    }

    pub(crate) fn complete(&mut self) {
        self.components.reverse();
    }
}


#[derive(Debug)]
pub enum SolutionComponent {
    Walk(WalkSolutionComponent),
    Bus(BusSolutionComponent),
}

impl fmt::Display for SolutionComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolutionComponent::Walk(_w) => {
                writeln!(f, "Walk path")
            }
            SolutionComponent::Bus(b) => {
                writeln!(f, "Route {} from {} to {}", b.route.route_long_name, b.from_inx.unwrap(), b.to_inx.unwrap())
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct BusSolutionComponent {
    pub route: Route,
    pub trip: Trip,
    /// Within the trip path, `from` and `to` which index
    pub from_inx: Option<usize>,
    pub to_inx: Option<usize>,
}

#[derive(Debug, Default)]
pub struct WalkSolutionComponent {
    pub stop_id: usize,
}
