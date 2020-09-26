use crate::gtfs_data::{GtfsData, LatLng, Shape, TripId};

#[derive(Debug, Default)]
pub struct TripRealTimePositionData {
    trip_id: TripId,
    shape: Shape,
    time_at_shape_point_seconds: Vec<i64>,
}

impl TripRealTimePositionData {
    pub fn new(ds: &GtfsData, trip_id: TripId) -> TripRealTimePositionData {
        TripRealTimePositionData {
            trip_id,
            shape: ds.get_shape(ds.get_trip(trip_id).shape_id).clone(),
            time_at_shape_point_seconds: calculate_time_at_each_path_point(ds, trip_id),
        }
    }
    pub fn get_position(&self, seconds_since_midnight: i64) -> LatLng {
        let points = &self.shape.points;
        let start_point = match self
            .time_at_shape_point_seconds
            .binary_search(&seconds_since_midnight)
        {
            Ok(exact) => exact,
            Err(next) => {
                if next == 0 {
                    next
                } else {
                    next - 1
                }
            }
        };

        if start_point == 0 {
            return points[0].clone();
        }
        if start_point == points.len() - 1 {
            return points.last().unwrap().clone();
        }
        assert!(start_point < points.len() - 1);
        let att = &points[start_point];
        let succ = &points[start_point + 1];

        // seconds after the initial_point
        let att_time = seconds_since_midnight - self.time_at_shape_point_seconds[start_point];
        let delta_time_ = self.time_at_shape_point_seconds[start_point + 1]
            - self.time_at_shape_point_seconds[start_point]
            + 1;

        let dx = succ.lat - att.lat;
        let dy = succ.lng - att.lng;

        let percent = att_time as f64 / delta_time_ as f64;

        LatLng {
            lat: att.lat + dx * percent,
            lng: att.lng + dy * percent,
        }
    }
    pub fn get_time_point_path(&self) -> Vec<usize> {
        self.time_at_shape_point_seconds
            .iter()
            .map(|t| *t as usize)
            .collect()
    }
}

fn create_cumulative_distances(points: &Vec<LatLng>) -> Vec<u64> {
    let mut cum_dist = vec![0; points.len()];
    for i in 1..points.len() {
        cum_dist[i] = cum_dist[i - 1] + points[i - 1].distance_meters(&points[i]);
    }
    cum_dist
}

fn calculate_time_at_each_path_point(ds: &GtfsData, trip_id: TripId) -> Vec<i64> {
    let trip = ds.get_trip(trip_id);
    let shape = ds.get_shape(trip.shape_id).clone();
    let points = &shape.points;
    let cum_dist = create_cumulative_distances(&shape.points);
    let stop_times = ds.get_stop_times(trip.stop_times_id);

    let mut time_at_shape_point = vec![0; points.len()];
    time_at_shape_point[0] = stop_times.stop_times[0].time + trip.start_time;

    assert_eq!(cum_dist.len(), points.len());
    assert_eq!(time_at_shape_point.len(), points.len());

    let mut shape_inx = 1;
    let mut prec_stop_inx_in_shape = 0;
    let mut prec_time = time_at_shape_point[0];

    // let's iterate all the stops, skipping the first. Each time we set the times of all the
    // shape points until the one of the current stop.
    // prec_stop_inx_in_shape ----------------------> next_stop ----(next for iteration)---> ...
    //     (prec_time)                               (next_time)
    for (ith, next_stop_time) in stop_times.stop_times.iter().enumerate().skip(1) {
        let (mut next_time, next_stop) = (
            next_stop_time.time + trip.start_time,
            ds.get_stop(next_stop_time.stop_id),
        );

        // There is no time difference between this stop and the previous one
        // (time needed would be 0, speed infinite)
        if next_time == prec_time {
            // If it is the last one, we just advance the next time, so we have the bus moving from the previous to the last.
            if ith == stop_times.stop_times.len() - 1 {
                next_time += 1;
            } else {
                // we skip this stop, and calculate the speed using the next one.
                continue;
            }
        }

        // let's find the nearest shape point to the stop position
        let stop_inx_in_shape =
            nearest_point_index(points, &next_stop.stop_pos, prec_stop_inx_in_shape);
        let stops_delta_dist =
            (cum_dist[stop_inx_in_shape] - cum_dist[prec_stop_inx_in_shape]) as f64;
        // maybe 2 repeated stops
        if stops_delta_dist == 0.0 {
            continue;
        }
        let stops_delta_time = (next_time - prec_time) as f64;
        // this is the speed of the bus from the precedent stop to the next one.
        // We assume it is constant for all this piece.
        let speed_between_stops = stops_delta_dist / stops_delta_time;

        /* Now, let's find a time for each shape point between these two stops */
        while shape_inx <= stop_inx_in_shape {
            let att_dist = (cum_dist[shape_inx] - cum_dist[prec_stop_inx_in_shape]) as f64;
            time_at_shape_point[shape_inx] = time_at_shape_point[prec_stop_inx_in_shape]
                + (att_dist / speed_between_stops) as i64;
            shape_inx += 1;
        }
        assert_eq!(shape_inx, stop_inx_in_shape + 1);
        assert!((time_at_shape_point[stop_inx_in_shape] - next_time).abs() < 2);
        // due to float rounding, the destination might be missing by a few seconds. In this way,
        // we do not propagate the error.
        time_at_shape_point[stop_inx_in_shape] = next_time;
        prec_stop_inx_in_shape = stop_inx_in_shape;
        prec_time = next_time;
    }
    time_at_shape_point
}

fn nearest_point_index(points: &Vec<LatLng>, target: &LatLng, start_from: usize) -> usize {
    let mut best_dist = u64::MAX;
    let mut best_inx = start_from;
    for (i, p) in points.iter().skip(start_from).enumerate() {
        let d = p.distance_meters(target);
        if d < best_dist {
            best_dist = d;
            best_inx = i;
        }
    }
    best_inx + start_from
}
