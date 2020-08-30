use std::iter::Iterator;

#[cfg(test)]
#[cfg(test)]
use itertools::Itertools;

#[cfg(test)]
use fastgtfs::gtfs_data::GtfsData;
use fastgtfs::gtfs_data::GtfsTime;

#[test]
fn basic_parsing() {
    // let mut dataset: GtfsData = Default::default();
    // let now = Instant::now();
    // let path = vec![get_test_paths()[0].to_string()];
    // dataset = raw_parser::parse_from_paths(path);
    // debug!("All parsing time: {}", now.elapsed().as_millis());
    //
    // assert_dataset_filled(&dataset)
}

#[test]
fn parse_multiple() {
    // let paths = get_test_paths();
    //
    // let mut dataset: GtfsData = Default::default();
    // let now = Instant::now();
    // dataset = parser::parse_from_paths(paths);
    // debug!("All parsing time: {}", now.elapsed().as_millis());
    //
    // assert_dataset_filled(&dataset)
}

#[cfg(test)]
fn parse_all() -> Vec<GtfsData> {
    vec![]
    // get_test_paths()
    //     .into_iter()
    //     .map(|p| parser::parse_from_paths(vec![p]))
    //     .collect::<Vec<GtfsData>>()
}

#[test]
fn routes_stoptimes_filled() {
    //todo
}

#[test]
fn groupby_test() {
    let s = "1,1,1,2,2,2,2";
    for (key, group) in &s.split(',').group_by(|n| n.parse::<i32>().unwrap()) {
        print!("{}", key);
        print!("{:#?}", group.collect::<Vec<&str>>());
    }
}

#[test]
fn test_gtfs_time() {
    let seeconds_in_day = 60 * 60 * 24;

    let mut t = GtfsTime::from_date(&"20200830".to_string());
    let t2 = GtfsTime::from_date(&"20200831".to_string());
    assert_eq!(t2.day_of_week(), 0);
    assert!(t < t2);

    t.add_seconds(seeconds_in_day);
    assert_eq!(t, t2);

    t.add_seconds(seeconds_in_day);
    assert!(t > t2);

    t.set_day_from(&t2);
    assert_eq!(t, t2);

    let inf = GtfsTime::new_infinite();
    assert!(GtfsTime::from_date(&"21000101".to_string()) < inf);


    let from_mid = GtfsTime::new_from_midnight(42);
    assert_eq!(from_mid.since_midnight(), 42);
}