use std::fmt;

use serde::{Deserialize, Serialize};

use crate::gtfs_data::{GtfsData, GtfsTime, LatLng, Route, StopId, StopTimes, Trip};
use crate::navigator::{BacktrackingInfo, RaptorNavigator};
use crate::navigator_models::SolutionComponent::{Bus, Walk};

#[derive(Debug, Clone, Default)]
pub struct NavigationParams {
    pub from: LatLng,
    pub to: LatLng,
    pub max_changes: u8,
    pub start_time: GtfsTime,
    pub num_solutions_to_find: u8,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
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
        if let Some(Walk(w)) = self.components.last_mut() {
            w.from_stop_id = stop_id
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
            shape: Default::default(),
            from_inx,
            to_inx,
        };
        self.set_last_component_start(stop_id);
        self.components.push(SolutionComponent::Bus(component));
    }

    pub(crate) fn complete(&mut self, dataset: &GtfsData) {
        self.components.reverse();
        self.compute_bus_shapes(dataset);
    }

    fn compute_bus_shapes(&mut self, dataset: &GtfsData) {
        for component in &mut self.components {
            if let Bus(b) = component {
                let from_stop = dataset.get_stop(b.path.stop_times[b.from_inx].stop_id);
                let to_stop = dataset.get_stop(b.path.stop_times[b.to_inx].stop_id);
                b.shape = dataset.get_shape_between(
                    b.trip.shape_id,
                    &from_stop.stop_pos,
                    &to_stop.stop_pos,
                )
            }
        }
    }

    pub fn start_time(&self) -> GtfsTime {
        if self.components.is_empty() {
            return GtfsTime::new_from_midnight(self.navigation_start_time.since_midnight() as i64);
        }

        let mut walk_time = 0;
        for component in self.components.iter() {
            match component {
                Walk(w) => {
                    walk_time += RaptorNavigator::seconds_by_walk(w.distance);
                }
                SolutionComponent::Bus(bus) => {
                    return GtfsTime::new_from_midnight(
                        (bus.departure_time().since_midnight() - walk_time) as i64,
                    );
                }
            }
        }
        // Only 1 walk path.
        GtfsTime::new_from_midnight(self.navigation_start_time.since_midnight() as i64)
    }
    pub fn end_time(&self) -> GtfsTime {
        if self.components.is_empty() {
            return GtfsTime::new_from_midnight(self.navigation_start_time.since_midnight() as i64);
        }

        let mut walk_time = 0;
        for component in self.components.iter().rev() {
            match component {
                Walk(w) => {
                    walk_time += RaptorNavigator::seconds_by_walk(w.distance);
                }
                SolutionComponent::Bus(bus) => {
                    return GtfsTime::new_from_midnight(
                        (bus.arrival_time().since_midnight() + walk_time) as i64,
                    );
                }
            }
        }

        // Only walk paths.
        GtfsTime::new_from_midnight(
            (self.navigation_start_time.since_midnight() + walk_time) as i64,
        )
    }
    pub fn duration_seconds(&self) -> usize {
        self.start_time().distance(&self.end_time()) as usize
    }
}

//noinspection RsExternalLinter
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SolutionComponent {
    Walk(WalkSolutionComponent),
    //noinspection ALL
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

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct BusSolutionComponent {
    pub route: Route,
    pub trip: Trip,
    pub path: StopTimes,
    /// Within the trip path, `from` and `to` which index
    pub from_inx: usize,
    pub to_inx: usize,
    pub shape: Vec<LatLng>,
}

impl BusSolutionComponent {
    pub fn departure_time(&self) -> GtfsTime {
        GtfsTime::new_from_midnight(self.trip.start_time + self.path.stop_times[self.from_inx].time)
    }

    pub fn arrival_time(&self) -> GtfsTime {
        GtfsTime::new_from_midnight(self.trip.start_time + self.path.stop_times[self.to_inx].time)
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
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
