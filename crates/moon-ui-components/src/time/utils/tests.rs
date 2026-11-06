use chrono::{Datelike, NaiveDate};

use super::{NaiveDateExt, days_in_month};

#[test]
fn test_days_in_month() {
    assert_eq!(
        NaiveDate::from_ymd_opt(2024, 2, 1).unwrap().days_in_month(),
        29
    );
    assert_eq!(
        NaiveDate::from_ymd_opt(2023, 2, 1).unwrap().days_in_month(),
        28
    );
    assert_eq!(
        NaiveDate::from_ymd_opt(2023, 1, 1).unwrap().days_in_month(),
        31
    );
    assert_eq!(
        NaiveDate::from_ymd_opt(2023, 4, 1).unwrap().days_in_month(),
        30
    );
}

#[test]
fn test_days() {
    #[track_caller]
    fn assert_case(date: NaiveDate, expected: Vec<&str>) {
        let out = days_in_month(date.year(), date.month())
            .iter()
            .map(|week| {
                week.iter()
                    .map(|d| {
                        if d.year() == date.year() && d.month() == date.month() {
                            format!("{:2}", d.day())
                        } else if d.year() == date.year() {
                            format!("{}-{}", d.month(), d.day())
                        } else {
                            format!("{}-{}-{}", d.year(), d.month(), d.day())
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .collect::<Vec<_>>();

        assert_eq!(out, expected);
    }

    assert_case(
        NaiveDate::from_ymd_opt(2024, 8, 1).unwrap(),
        vec![
            "7-28|7-29|7-30|7-31| 1| 2| 3",
            " 4| 5| 6| 7| 8| 9|10",
            "11|12|13|14|15|16|17",
            "18|19|20|21|22|23|24",
            "25|26|27|28|29|30|31",
        ],
    );
    assert_case(
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        vec![
            "2024-12-29|2024-12-30|2024-12-31| 1| 2| 3| 4",
            " 5| 6| 7| 8| 9|10|11",
            "12|13|14|15|16|17|18",
            "19|20|21|22|23|24|25",
            "26|27|28|29|30|31|2-1",
        ],
    );

    assert_case(
        NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
        vec![
            "1-28|1-29|1-30|1-31| 1| 2| 3",
            " 4| 5| 6| 7| 8| 9|10",
            "11|12|13|14|15|16|17",
            "18|19|20|21|22|23|24",
            "25|26|27|28|29|3-1|3-2",
        ],
    );
    assert_case(
        NaiveDate::from_ymd_opt(2023, 2, 20).unwrap(),
        vec![
            "1-29|1-30|1-31| 1| 2| 3| 4",
            " 5| 6| 7| 8| 9|10|11",
            "12|13|14|15|16|17|18",
            "19|20|21|22|23|24|25",
            "26|27|28|3-1|3-2|3-3|3-4",
        ],
    );
}

/// Replacing the required sixth week with five rows hides August 30/31 from date selection.
#[test]
fn every_day_is_present_once_in_calendar_weeks() {
    for year in [2024, 2026, 2100] {
        for month in 1..=12 {
            let weeks = days_in_month(year, month);
            for day in 1..=NaiveDate::from_ymd_opt(year, month, 1)
                .unwrap()
                .days_in_month()
            {
                let date = NaiveDate::from_ymd_opt(year, month, day as u32).unwrap();
                assert_eq!(
                    weeks.iter().flatten().filter(|&&d| d == date).count(),
                    1,
                    "{date}"
                );
            }
            assert!(weeks.iter().all(|week| week.len() == 7));
        }
    }
}
