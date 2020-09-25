use std::fmt;

use crate::gtfs_data::{GtfsTime, LatLng, Route, StopId, StopTimes, Trip};
use crate::navigator::{BacktrackingInfo, RaptorNavigator};
use crate::navigator_models::SolutionComponent::Bus;

#[derive(Debug, Clone, Default)]
pub struct NavigationParams {
    pub from: LatLng,
    pub to: LatLng,
    pub max_changes: u8,
    pub start_time: GtfsTime,
    pub num_solutions_to_find: u8,
    //pub sol_callback: Box<dyn Fn(Solution)>,
}

#[derive(Debug, Default, Clone)]
pub struct Solution {
    pub navigation_start_time: GtfsTime,
    pub components: Vec<SolutionComponent>,
}

impl fmt::Display for Solution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "\n---- New solution ----: {} -> {}",
            self.start_time(),
            self.end_time()
        )
        .unwrap();
        writeln!(
            f,
            "\n####### Solution components: {}",
            self.components.len()
        )
        .unwrap();
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
                w.from_stop_id = stop_id
            }
        }
    }
    pub(crate) fn add_walking_path(
        &mut self,
        from_stop_id: usize,
        to_stop_id: usize,
        distance: usize,
    ) {
        let component = WalkSolutionComponent {
            from_stop_id,
            to_stop_id,
            distance,
        };
        self.set_last_component_start(from_stop_id);
        self.components.push(SolutionComponent::Walk(component));
    }

    pub(crate) fn add_bus_path(
        &mut self,
        stop_id: usize,
        route: &Route,
        trip: &Trip,
        path: &StopTimes,
        from_inx: usize,
        to_inx: usize,
    ) {
        let component = BusSolutionComponent {
            route: route.clone(),
            trip: trip.clone(),
            path: path.clone(),
            from_inx,
            to_inx,
        };
        self.set_last_component_start(stop_id);
        self.components.push(SolutionComponent::Bus(component));
    }

    pub(crate) fn complete(&mut self) {
        self.components.reverse();
    }

    pub fn start_time(&self) -> GtfsTime {
        if self.components.is_empty() {
            return GtfsTime::new_from_midnight(self.navigation_start_time.since_midnight() as i64);
        }

        match self.components.first().unwrap() {
            SolutionComponent::Walk(w) => {
                if self.components.len() == 1 {
                    GtfsTime::new_from_midnight(self.navigation_start_time.since_midnight() as i64)
                } else {
                    let walk_time = RaptorNavigator::seconds_by_walk(w.distance);
                    assert!(self.components.len() >= 2);
                    let second = &self.components[1];
                    match second {
                        Bus(sl) => GtfsTime::new_from_midnight(
                            (sl.departure_time().since_midnight() - walk_time) as i64,
                        ),
                        _ => {
                            panic!("Can't have 2 consecutive walk compnents!");
                        }
                    }
                }
            }
            Bus(b) => b.departure_time(),
        }
    }
    pub fn end_time(&self) -> GtfsTime {
        if self.components.is_empty() {
            return GtfsTime::new_from_midnight(self.navigation_start_time.since_midnight() as i64);
        }

        match self.components.last().unwrap() {
            SolutionComponent::Walk(walk) => {
                let walk_time = RaptorNavigator::seconds_by_walk(walk.distance);
                // only walk path
                if self.components.len() == 1 {
                    GtfsTime::new_from_midnight(
                        (self.navigation_start_time.since_midnight() + walk_time) as i64,
                    )
                } else {
                    // walk path preceeded by bus
                    assert!(self.components.len() >= 2);
                    let second_last = &self.components[self.components.len() - 2];
                    match second_last {
                        Bus(sl) => GtfsTime::new_from_midnight(
                            (sl.arrival_time().since_midnight() + walk_time) as i64,
                        ),
                        _ => {
                            panic!("Can't have 2 consecutive walk components!");
                        }
                    }
                }
            }
            SolutionComponent::Bus(bus) => {
                GtfsTime::new_from_midnight(bus.arrival_time().since_midnight() as i64)
            }
        }
    }
    pub fn duration_seconds(&self) -> usize {
        self.start_time().distance(&self.end_time()) as usize
    }
}

#[derive(Debug, Clone)]
pub enum SolutionComponent {
    Walk(WalkSolutionComponent),
    Bus(BusSolutionComponent),
}

impl fmt::Display for SolutionComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolutionComponent::Walk(_w) => {
                writeln!(f, "Walk path").unwrap();
            }
            SolutionComponent::Bus(b) => {
                writeln!(
                    f,
                    "Route {} - {} trip {} \nfrom {} ({}) to {} ({})",
                    b.route.route_long_name,
                    b.route.route_short_name,
                    b.trip.trip_id,
                    b.from_inx,
                    b.departure_time(),
                    b.to_inx,
                    b.arrival_time()
                )
                .unwrap();
            }
        };
        writeln!(f, "               ↓")
    }
}

#[derive(Debug, Default, Clone)]
pub struct BusSolutionComponent {
    pub route: Route,
    pub trip: Trip,
    pub path: StopTimes,
    /// Within the trip path, `from` and `to` which index
    pub from_inx: usize,
    pub to_inx: usize,
}

impl BusSolutionComponent {
    pub fn departure_time(&self) -> GtfsTime {
        GtfsTime::new_from_midnight(self.trip.start_time + self.path.stop_times[self.from_inx].time)
    }

    pub fn arrival_time(&self) -> GtfsTime {
        GtfsTime::new_from_midnight(self.trip.start_time + self.path.stop_times[self.to_inx].time)
    }
}

#[derive(Debug, Default, Clone)]
pub struct WalkSolutionComponent {
    pub from_stop_id: usize,
    pub to_stop_id: usize,
    pub distance: usize, // meters
}

pub struct TimeUpdate {
    pub to_stop_id: StopId,
    //pub cost_seconds: u64,
    pub destination_time: GtfsTime,
    // This is none only for walking paths
    pub backtrack_info: BacktrackingInfo,
}
