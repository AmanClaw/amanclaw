use chrono::{Datelike, NaiveDate};

pub struct HijriDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub month_name_ar: &'static str,
    pub month_name_ms: &'static str,
}

const HIJRI_MONTHS_AR: [&str; 12] = [
    "Muharram",
    "Safar",
    "Rabi'ul Awal",
    "Rabi'ul Akhir",
    "Jamadil Awal",
    "Jamadil Akhir",
    "Rejab",
    "Sya'ban",
    "Ramadan",
    "Syawal",
    "Zulkaedah",
    "Zulhijjah",
];

const HIJRI_MONTHS_MS: [&str; 12] = [
    "Muharram",
    "Safar",
    "Rabiulawal",
    "Rabiulakhir",
    "Jamadilawal",
    "Jamadilakhir",
    "Rejab",
    "Syaaban",
    "Ramadan",
    "Syawal",
    "Zulkaedah",
    "Zulhijjah",
];

/// Leap years in the 30-year Hijri cycle (tabular Islamic calendar).
const LEAP_YEARS: [i32; 11] = [2, 5, 7, 10, 13, 16, 18, 21, 24, 26, 29];

/// Islamic calendar epoch in Julian Day Number (July 16, 622 CE Julian).
const HIJRI_EPOCH_JDN: i64 = 1948439;

/// Check if a Hijri year is a leap year in the tabular calendar.
fn is_hijri_leap(year: i64) -> bool {
    let pos = ((year - 1) % 30 + 1) as i32;
    LEAP_YEARS.contains(&pos)
}

/// Length of a Hijri month (1-12). Odd months have 30 days, even months 29.
/// Month 12 has 30 days in leap years.
fn hijri_month_length(month: u32, leap: bool) -> i64 {
    if month % 2 == 1 || (month == 12 && leap) {
        30
    } else {
        29
    }
}

/// Number of days in a complete Hijri year.
fn hijri_year_length(year: i64) -> i64 {
    if is_hijri_leap(year) { 355 } else { 354 }
}

/// Compute Julian Day Number from a Gregorian date.
fn gregorian_to_jdn(year: i32, month: i32, day: i32) -> i64 {
    let (y, m) = if month <= 2 {
        ((year - 1) as i64, (month + 12) as i64)
    } else {
        (year as i64, month as i64)
    };
    let a = y / 100;
    let b = 2 - a + a / 4;
    (365.25 * (y + 4716) as f64) as i64 + (30.6001 * (m + 1) as f64) as i64 + day as i64 + b - 1524
}

/// Convert a Gregorian date to a Hijri date using the tabular Islamic calendar.
///
/// The tabular Islamic calendar uses a fixed arithmetic scheme based on a
/// 30-year cycle. It may differ from the observational calendar by 1-2 days.
pub fn gregorian_to_hijri(date: NaiveDate) -> HijriDate {
    let jd = gregorian_to_jdn(date.year(), date.month() as i32, date.day() as i32);
    let days_since_epoch = jd - HIJRI_EPOCH_JDN;

    // 30-year cycles, each cycle = 10631 days
    let cycle = days_since_epoch / 10631;
    let remaining = days_since_epoch - cycle * 10631;

    // Approximate year within the cycle
    let year_in_cycle = ((remaining as f64 - 0.5) / 354.36667).floor() as i64;
    let year_in_cycle = year_in_cycle.clamp(0, 29);

    let year = cycle * 30 + year_in_cycle + 1;

    // Days elapsed in complete years of this cycle
    let mut days_in_years: i64 = 0;
    for yr in 1..=year_in_cycle {
        let abs_year = cycle * 30 + yr;
        days_in_years += hijri_year_length(abs_year);
    }

    let day_of_year = remaining - days_in_years;

    // If day_of_year exceeds the current year length, advance to next year
    let current_year_len = hijri_year_length(year);
    let (year, day_of_year) = if day_of_year > current_year_len {
        (year + 1, day_of_year - current_year_len)
    } else {
        (year, day_of_year)
    };

    // Determine month and day
    let leap = is_hijri_leap(year);
    let mut month = 1u32;
    let mut remaining_days = day_of_year;
    for m in 1..=12u32 {
        let mlen = hijri_month_length(m, leap);
        if remaining_days <= mlen {
            month = m;
            break;
        }
        remaining_days -= mlen;
        if m == 12 {
            month = 12;
        }
    }

    let day = remaining_days.max(1) as u32;
    let year = year as i32;

    let mi = (month - 1) as usize;
    HijriDate {
        year,
        month,
        day,
        month_name_ar: HIJRI_MONTHS_AR[mi],
        month_name_ms: HIJRI_MONTHS_MS[mi],
    }
}

/// Return the Malay month name for a given Hijri month number (1-12).
pub fn month_name_ms(month: u32) -> &'static str {
    let idx = ((month.saturating_sub(1)) as usize).min(11);
    HIJRI_MONTHS_MS[idx]
}

impl std::fmt::Display for HijriDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.day, self.month_name_ms, self.year)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_date_2024_01_01() {
        // 1 January 2024 = approximately 19 Jamadilakhir 1445
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let hijri = gregorian_to_hijri(date);
        assert_eq!(hijri.year, 1445, "year mismatch, got {}", hijri.year);
        assert_eq!(
            hijri.month, 6,
            "month mismatch: got {} ({})",
            hijri.month, hijri.month_name_ms
        );
        assert!(
            (18..=20).contains(&hijri.day),
            "day was {}, expected around 19",
            hijri.day
        );
    }

    #[test]
    fn test_known_date_2025_03_01() {
        // 1 March 2025 = approximately 1 Ramadan 1446
        let date = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
        let hijri = gregorian_to_hijri(date);
        assert_eq!(hijri.year, 1446, "year mismatch, got {}", hijri.year);
        // May be Sha'ban 30 or Ramadan 1 depending on algorithm
        assert!(
            (hijri.month == 8 && hijri.day >= 29) || hijri.month == 9,
            "expected ~Ramadan 1446, got {} {} {}",
            hijri.day,
            hijri.month_name_ms,
            hijri.year
        );
    }

    #[test]
    fn test_known_date_eid_alfitr_2023() {
        // 22 April 2023 = approximately 1 Syawal 1444 (Eid al-Fitr)
        let date = NaiveDate::from_ymd_opt(2023, 4, 22).unwrap();
        let hijri = gregorian_to_hijri(date);
        assert_eq!(hijri.year, 1444, "year mismatch, got {}", hijri.year);
        assert!(
            (hijri.month == 10 && hijri.day <= 2) || (hijri.month == 9 && hijri.day >= 29),
            "expected ~Syawal 1 1444, got {} {} {}",
            hijri.day,
            hijri.month_name_ms,
            hijri.year
        );
    }

    #[test]
    fn test_display() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let hijri = gregorian_to_hijri(date);
        let s = format!("{hijri}");
        assert!(s.contains("1445"), "Display should contain year 1445: {s}");
        assert!(
            s.contains("Jamadilakhir"),
            "Display should contain month name: {s}"
        );
    }

    #[test]
    fn test_month_name_ms() {
        assert_eq!(month_name_ms(1), "Muharram");
        assert_eq!(month_name_ms(9), "Ramadan");
        assert_eq!(month_name_ms(12), "Zulhijjah");
    }

    #[test]
    fn test_leap_years() {
        assert!(is_hijri_leap(2));
        assert!(is_hijri_leap(5));
        assert!(!is_hijri_leap(1));
        assert!(!is_hijri_leap(3));
        assert!(is_hijri_leap(1445)); // 1445 % 30 = 15 => pos = 16, which is leap
    }
}
