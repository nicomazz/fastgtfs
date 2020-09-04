#[macro_use]
extern crate log;

use chrono::NaiveDate;

use fastgtfs::gtfs_data::GtfsTime;

fn default_start_time() -> GtfsTime {
    let start_timestamp = NaiveDate::from_ymd(2020, 08, 30)
        .and_hms(13, 30, 00)
        .timestamp();
    GtfsTime::new_from_timestamp(start_timestamp)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use log::debug;

    use fastgtfs::gtfs_data::LatLng;
    use fastgtfs::navigator::RaptorNavigator;
    use fastgtfs::navigator_models::{NavigationParams, Solution, SolutionComponent};
    use fastgtfs::raw_parser::RawParser;
    use fastgtfs::test_utils::get_test_paths;

    use crate::default_start_time;

    fn init() {
        let _ = env_logger::builder()
            .is_test(true)
            .format_timestamp(None)
            .format_module_path(false)
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    #[test]
    fn test_walk_time() {
        let seconds_1_km = RaptorNavigator::seconds_by_walk(1000);
        println!("Time 1km: {} min ({} s)", seconds_1_km / 60, seconds_1_km);
        assert!(seconds_1_km > 60 * 60 / 5); // slower than 5 km/h
        assert!(seconds_1_km < 60 * 60 / 2); // faster than 2 km/h
    }

    #[test]
    fn test_navigator() {
        init();

        let test_paths = get_test_paths();
        let mut parser = RawParser::new(test_paths);
        parser.ensure_data_serialized_created();
        let dataset = RawParser::read_preprocessed_data_from_default();
        trace!("Dataset parsed!");

        //let (tx, rx): (Sender<Solution>, Receiver<Solution>) = mpsc::channel();
        let sol_cnt = Arc::new(Mutex::new(0));
        let sol_cnt_2 = Arc::clone(&sol_cnt);
        let on_solution = Box::new(move |sol: Solution| {
            debug!("A SOLUTION HAS BEEN RECEIVED! {}", sol);
            let mut cnt = sol_cnt_2.lock().unwrap();
            *cnt += 1;
            let mut last_time = default_start_time();
            for component in &sol.components {
                if let SolutionComponent::Bus(b) = component {
                    assert!(last_time <= b.departure_time()); // todo fix this
                    last_time = b.arrival_time();
                }
            }
        });

        let mut navigator = RaptorNavigator::new(&dataset, on_solution);

        let venice = LatLng {
            lat: 45.437771117019466,
            lng: 12.31865644454956,
        };
        let nave_de_vero = LatLng {
            lat: 45.45926209023005,
            lng: 12.21256971359253,
        };

        let params = NavigationParams {
            from: venice,
            to: nave_de_vero,
            max_changes: 3,
            start_time: default_start_time(),
            num_solutions_to_find: 3,
        };
        navigator.find_path_multiple(params);

        assert_eq!(*sol_cnt.lock().unwrap(), 3);
    }
}
