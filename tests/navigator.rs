#[macro_use]
extern crate log;

use chrono::NaiveDate;

use fastgtfs::gtfs_data::GtfsTime;

fn default_start_time() -> GtfsTime {
    let start_timestamp = NaiveDate::from_ymd(2020, 08, 30).and_hms(17, 33, 44).timestamp();
    GtfsTime::new_from_timestamp(start_timestamp)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::sync::mpsc::{Receiver, Sender};
    use std::thread;

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
            .filter_level(log::LevelFilter::Trace)
            .try_init();
    }

    #[test]
    fn test_navigator() {
        init();

        let test_paths = get_test_paths();
        let mut parser = RawParser::new(test_paths);
        parser.ensure_data_serialized_created();
        let dataset = RawParser::read_preprocessed_data_from_default();
        trace!("Dataset parsed!");

        let (tx, rx): (Sender<Solution>, Receiver<Solution>) = mpsc::channel();
        thread::spawn(move || {
            let mut navigator = RaptorNavigator::new(&dataset, tx);

            let venice = LatLng { lat: 45.437771117019466, lng: 12.31865644454956 };
            let nave_de_vero = LatLng { lat: 45.45926209023005, lng: 12.21256971359253 };

            let params = NavigationParams {
                from: venice,
                to: nave_de_vero,
                max_changes: 3,
                start_time: default_start_time(),
                num_solutions_to_find: 3
            };
            navigator.find_path_multiple(params);
        });

        let mut sol_cnt = 0;
        for sol in rx {
            debug!("A SOLUTION HAS BEEN RECEIVED! {}", sol);

            sol_cnt += 1;
            let mut last_time = default_start_time();
            for component in &sol.components {
                if let SolutionComponent::Bus(b) = component {
                    assert!(last_time <= b.departure_time());
                    last_time = b.arrival_time();
                }
            }
        }
        assert_eq!(sol_cnt, 3);
    }
}