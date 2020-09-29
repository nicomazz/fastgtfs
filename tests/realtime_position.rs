#[cfg(test)]
mod tests {

    use fastgtfs::gtfs_data::{GtfsData, TripId};
    use fastgtfs::navigator_models::SolutionComponent;
    use fastgtfs::raw_parser::RawParser;
    use fastgtfs::test_utils::get_test_paths;

    use fastgtfs::realtime_position::TripRealTimePositionData;
    use itertools::Itertools;
    use rayon::iter::IntoParallelRefIterator;
    use rayon::iter::ParallelIterator;

    fn init() {
        let _ = env_logger::builder()
            .is_test(true)
            .format_timestamp(None)
            .format_module_path(false)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    #[test]
    fn realtime_position_test() {
        init();
        let test_paths = get_test_paths();
        let mut parser = RawParser::new(test_paths);
        parser.ensure_data_serialized_created();
        let dataset = RawParser::read_preprocessed_data_from_default();
        println!("All data readed");

        dataset
            .trips
            .iter()
            .take(1000)
            .collect_vec()
            .par_iter()
            .for_each(|t| verify_realtime_position_for_trip_id(&dataset, t.trip_id));
    }

    fn verify_realtime_position_for_trip_id(ds: &GtfsData, trip_id: TripId) {
        let trip = ds.get_trip(trip_id);
        let shape = ds.get_shape(trip.shape_id);
        let realtime_handler = TripRealTimePositionData::new(&ds, trip_id);
        let stop_times = ds.get_stop_times(trip.stop_times_id);

        let start_pos = realtime_handler.get_position(0);
        let end_pos = realtime_handler.get_position(60 * 60 * 24 * 2);

        assert!(start_pos.distance_meters(&shape.points[0]) < 10);
        assert!(end_pos.distance_meters(&shape.points.last().unwrap()) < 10);

        let mut over = 0;
        for (i, stop_time) in stop_times.stop_times.iter().enumerate() {
            let real_stop_time = trip.start_time + stop_time.time;
            let real_stop_position = &ds.get_stop(stop_time.stop_id).stop_pos;

            let dist = realtime_handler
                .get_position(real_stop_time)
                .distance_meters(real_stop_position);
            if dist > 20 {
                over += 1;
            }
        }
        assert!(over <= stop_times.stop_times.len() / 2);
    }
}
