#[macro_use]
extern crate log;

use chrono::NaiveDate;
use geo::algorithm::contains::Contains;
use geo::{Coordinate, Rect};
use log::debug;
use rand::Rng;

use fastgtfs::gtfs_data::{GtfsData, GtfsTime, LatLng};
use fastgtfs::navigator::RaptorNavigator;
use fastgtfs::navigator_models::{NavigationParams, Solution};
use fastgtfs::raw_parser::RawParser;
use fastgtfs::test_utils::get_test_paths;

#[cfg(test)]
mod tests {
    use crate::random_point;

    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .is_test(true)
            .format_timestamp(None)
            .format_module_path(false)
            .filter_level(log::LevelFilter::Trace)
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
    fn test_simple_navigation() {
        init();
        let dataset = get_dataset();

        let venice = LatLng {
            lat: 45.437_771_117_019_466,
            lng: 12.318_656_444_549_56,
        };
        let nave_de_vero = LatLng {
            lat: 45.459_262_090_230_05,
            lng: 12.212_569_713_592_53,
        };

        let solutions = navigate(dataset, &venice, &nave_de_vero);
        assert!(!solutions.is_empty());
        assert_eq!(solutions.len(), 3);

        for sol in solutions {
            debug!("solution: {}", sol);
        }
    }

    #[test]
    fn test_many_random_navigations_in_venice() {
        init();
        let dataset = get_dataset();
        test_many_random_navigations(dataset, get_venice_rect(), get_venice_rect());
    }

    #[test]
    fn test_many_random_navigations_from_marghera_to_venice() {
        init();
        let dataset = get_dataset();
        test_many_random_navigations(dataset, get_venice_rect(), get_marghera_rect());
    }

    #[test]
    fn test_many_random_navigations_from_marghera_to_lido() {
        init();
        let dataset = get_dataset();
        test_many_random_navigations(dataset, get_marghera_rect(), get_lido_rect());
    }

    #[test]
    fn test_random_point_in_rect() {
        init();

        let venice_rect = Rect::new(
            Coordinate { y: 0.0, x: 0.0 },
            Coordinate { y: 100.0, x: 10.0 },
        );
        (1..1000).for_each(|_| {
            let random_point = random_point(&venice_rect);
            assert!(venice_rect.contains(&random_point.as_point()));
        })
    }
}

fn get_dataset() -> GtfsData {
    let test_paths = get_test_paths();
    let mut parser = RawParser::new(test_paths);
    parser.ensure_data_serialized_created();
    RawParser::read_preprocessed_data_from_default()
}

fn test_many_random_navigations(dataset: GtfsData, from_rect: Rect<f64>, to_rect: Rect<f64>) {
    let total_runs = 50;
    let without_results = (0..total_runs)
        .map(|_| spawn_random_navigation(from_rect, to_rect, dataset.clone()))
        .filter(|v| v.is_empty())
        .count();
    println!("without results: {}/{}", without_results, total_runs);
    // Less than 50% can be without solution.
    assert!(without_results < total_runs / 2);
}

fn default_start_time() -> GtfsTime {
    let start_timestamp = NaiveDate::from_ymd(2020, 8, 30)
        .and_hms(13, 30, 00)
        .timestamp();
    GtfsTime::new_from_timestamp(start_timestamp)
}

fn navigate(dataset: GtfsData, from: &LatLng, to: &LatLng) -> Vec<Solution> {
    let params = NavigationParams {
        from: from.clone(),
        to: to.clone(),
        max_changes: 4,
        start_time: default_start_time(),
        num_solutions_to_find: 3,
    };
    let solutions = RaptorNavigator::navigate_blocking(dataset, params);
    validate_solutions(&solutions);
    solutions
}

fn random_point(rect: &Rect<f64>) -> LatLng {
    let mut rng = rand::thread_rng();
    LatLng {
        lat: rng.gen_range(rect.min().y..rect.max().y),
        lng: rng.gen_range(rect.min().x..rect.max().x),
    }
}

fn spawn_random_navigation(
    from_rect: Rect<f64>,
    to_rect: Rect<f64>,
    dataset: fastgtfs::gtfs_data::GtfsData,
) -> Vec<Solution> {
    let from = random_point(&from_rect);
    let to = random_point(&to_rect);

    let solutions = navigate(dataset, &from, &to);

    if solutions.is_empty() {
        error!(
            "No solutions. Path: https://www.google.com/maps/dir/{},{}/{},{}",
            from.lat, from.lng, to.lat, to.lng
        )
    }

    solutions
}

fn validate_solutions(solutions: &[Solution]) {
    for sol in solutions {
        let last_time = default_start_time();
        RaptorNavigator::validate_solution(sol, &last_time);
    }
}

fn get_venice_rect() -> Rect<f64> {
    Rect::new(
        Coordinate {
            y: 45.424_602_681_318_376,
            x: 12.303_094_520_121_759,
        },
        Coordinate {
            y: 45.450_890_160_854_27,
            x: 12.368_869_537_836_584,
        },
    )
}

fn get_marghera_rect() -> Rect<f64> {
    Rect::new(
        Coordinate {
            y: 45.454_297_120_360_685,
            x: 12.206_721_034_086_588,
        },
        Coordinate {
            y: 45.481_887_219_441_8,
            x: 12.236_169_051_095_533,
        },
    )
}

fn get_lido_rect() -> Rect<f64> {
    Rect::new(
        Coordinate {
            y: 45.338_964_245_294_484,
            x: 12.307_974_898_081_248,
        },
        Coordinate {
            y: 45.418_940_534_627_6,
            x: 12.387_781_177_677_978,
        },
    )
}
