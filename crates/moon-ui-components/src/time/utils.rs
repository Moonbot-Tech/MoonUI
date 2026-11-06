use chrono::{Datelike, Duration, NaiveDate};

trait NaiveDateExt {
    fn days_in_month(&self) -> i32;
    fn is_leap_year(&self) -> bool;
}

impl NaiveDateExt for chrono::NaiveDate {
    fn days_in_month(&self) -> i32 {
        let month = self.month();
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if self.is_leap_year() {
                    29
                } else {
                    28
                }
            }
            _ => panic!("Invalid month: {}", month),
        }
    }

    fn is_leap_year(&self) -> bool {
        let year = self.year();
        return year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    }
}

/// Return complete Sunday-first weeks for a month, retaining the five-row minimum.
///
/// Month zero and months past December are normalized across year boundaries.
/// Panics when the requested year or the surrounding grid exceeds chrono's date range.
pub(crate) fn days_in_month(year: i32, month: u32) -> Vec<Vec<NaiveDate>> {
    let offset = i64::from(month) - 1;
    let year = year + offset.div_euclid(12) as i32;
    let month = offset.rem_euclid(12) as u32 + 1;
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let leading = first.weekday().num_days_from_sunday() as i32;
    let weeks = ((leading + first.days_in_month() + 6) / 7).max(5);
    (0..weeks)
        .map(|week| {
            (0..7)
                .map(|day| first + Duration::days(i64::from(week * 7 + day - leading)))
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests;
