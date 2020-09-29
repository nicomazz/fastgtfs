#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Instant;

    use itertools::Itertools;
    use rayon::iter::IntoParallelRefIterator;
    use rayon::iter::ParallelIterator;

    use fastgtfs::gtfs_data::{GtfsData, StopId, TripId};
    use fastgtfs::raw_parser::RawParser;
    use fastgtfs::realtime_position::TripRealTimePositionData;
    use fastgtfs::test_utils::get_test_paths;
    use fastgtfs::timetable::TimeTable;

    fn init() {
        let _ = env_logger::builder()
            .is_test(true)
            .format_timestamp(None)
            .format_module_path(false)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    #[test]
    fn test_timetable_creation() {
        init();
        let test_paths = get_test_paths();
        let mut parser = RawParser::new(test_paths);
        parser.ensure_data_serialized_created();
        let ds = RawParser::read_preprocessed_data_from_default();
        let now = Instant::now();

        let errors: i64 = ds
            .routes
            .par_iter()
            .map(|route| {
                let routes = vec![route.route_id];
                let timetable = TimeTable::new(&ds, routes.clone(), 0)
                    .or_else(|_| TimeTable::new(&ds, routes.clone(), 1));
                if timetable.is_err() {
                    return 1;
                }
                let timetable = timetable.unwrap();

                let sorted_stops = timetable.stops;
                let route_stop_times = route
                    .trips
                    .iter()
                    .map(|&t| ds.get_trip(t).stop_times_id)
                    .unique()
                    .map(|st| &ds.get_stop_times(st).stop_times)
                    .collect_vec();

                // The result should contains all the stops of the longest trip.
                let max_trip_len = route_stop_times.iter().map(|st| st.len()).max().unwrap();
                assert!(
                    sorted_stops.len() >= max_trip_len,
                    "sorted stops: {:?}",
                    sorted_stops
                );

                // Check that the first stop of the sorted set is the frist stop of any trip.
                let first_stops: HashSet<StopId> = route_stop_times
                    .iter()
                    .map(|st| st.first().unwrap().stop_id)
                    .unique()
                    .collect();
                assert!(first_stops.contains(sorted_stops.first().unwrap()));

                // Same as before, but with the last.
                let last_stops: HashSet<StopId> = route_stop_times
                    .iter()
                    .map(|st| st.last().unwrap().stop_id)
                    .unique()
                    .collect();
                assert!(last_stops.contains(sorted_stops.last().unwrap()));
                0
            })
            .sum();

        println!(
            "timetable for all routes created in {}",
            now.elapsed().as_millis()
        );
        print!("Errors: {}", errors);
        assert!(errors < 5, "{} Errors", errors);
    }
}
