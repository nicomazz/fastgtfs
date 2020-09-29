use std::iter::Iterator;

#[cfg(test)]
#[cfg(test)]
use itertools::Itertools;

#[cfg(test)]
use fastgtfs::gtfs_data::GtfsTime;

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
