use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::methods::CalculationMethod;

const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;
const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;

/// Prayer times as (hour, minute) tuples in local time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrayerTimes {
    pub fajr: (u8, u8),
    pub sunrise: (u8, u8),
    pub dhuhr: (u8, u8),
    pub asr: (u8, u8),
    pub maghrib: (u8, u8),
    pub isha: (u8, u8),
    pub method: CalculationMethod,
    pub date: NaiveDate,
}

impl PrayerTimes {
    /// Format a (hour, minute) tuple as "HH:MM".
    pub fn format_time(t: (u8, u8)) -> String {
        format!("{:02}:{:02}", t.0, t.1)
    }

    /// Check that times are in proper order throughout the day.
    pub fn is_ordered(&self) -> bool {
        let to_min = |t: (u8, u8)| t.0 as u16 * 60 + t.1 as u16;
        let vals = [
            to_min(self.fajr),
            to_min(self.sunrise),
            to_min(self.dhuhr),
            to_min(self.asr),
            to_min(self.maghrib),
            to_min(self.isha),
        ];
        vals.windows(2).all(|w| w[0] < w[1])
    }
}

/// Convert a fractional hour (0.0..24.0) to (hour, minute).
fn hours_to_hm(h: f64) -> (u8, u8) {
    let h = h.rem_euclid(24.0);
    let hour = h.floor() as u8;
    let minute = ((h - hour as f64) * 60.0).round() as u8;
    if minute == 60 {
        ((hour + 1) % 24, 0)
    } else {
        (hour, minute)
    }
}

/// Julian day number for a given date.
fn julian_day(date: NaiveDate) -> f64 {
    let y = date.year() as f64;
    let m = date.month() as f64;
    let d = date.day() as f64;

    // Adjust January/February to be months 13/14 of previous year
    let (y2, m2) = if m <= 2.0 {
        (y - 1.0, m + 12.0)
    } else {
        (y, m)
    };

    let a = (y2 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();

    (365.25 * (y2 + 4716.0)).floor() + (30.6001 * (m2 + 1.0)).floor() + d + b - 1524.5
}

/// Sun's geometric mean longitude (degrees).
fn sun_mean_longitude(jc: f64) -> f64 {
    (280.46646 + jc * (36000.76983 + jc * 0.0003032)) % 360.0
}

/// Sun's mean anomaly (degrees).
fn sun_mean_anomaly(jc: f64) -> f64 {
    357.52911 + jc * (35999.05029 - jc * 0.0001537)
}

/// Earth's orbital eccentricity.
fn eccentricity(jc: f64) -> f64 {
    0.016708634 - jc * (0.000042037 + jc * 0.0000001267)
}

/// Sun's equation of center (degrees).
fn sun_equation_of_center(jc: f64) -> f64 {
    let m = sun_mean_anomaly(jc) * DEG_TO_RAD;
    let c = m.sin() * (1.914602 - jc * (0.004817 + 0.000014 * jc))
        + (2.0 * m).sin() * (0.019993 - 0.000101 * jc)
        + (3.0 * m).sin() * 0.000289;
    c
}

/// Sun's true longitude (degrees).
fn sun_true_longitude(jc: f64) -> f64 {
    sun_mean_longitude(jc) + sun_equation_of_center(jc)
}

/// Sun's apparent longitude (degrees).
fn sun_apparent_longitude(jc: f64) -> f64 {
    let omega = 125.04 - 1934.136 * jc;
    sun_true_longitude(jc) - 0.00569 - 0.00478 * (omega * DEG_TO_RAD).sin()
}

/// Mean obliquity of the ecliptic (degrees).
fn mean_obliquity(jc: f64) -> f64 {
    23.0 + (26.0 + (21.448 - jc * (46.815 + jc * (0.00059 - jc * 0.001813))) / 60.0) / 60.0
}

/// Corrected obliquity of the ecliptic (degrees).
fn obliquity_corrected(jc: f64) -> f64 {
    let omega = 125.04 - 1934.136 * jc;
    mean_obliquity(jc) + 0.00256 * (omega * DEG_TO_RAD).cos()
}

/// Sun's declination (degrees).
fn sun_declination(jc: f64) -> f64 {
    let e = obliquity_corrected(jc) * DEG_TO_RAD;
    let lam = sun_apparent_longitude(jc) * DEG_TO_RAD;
    (e.sin() * lam.sin()).asin() * RAD_TO_DEG
}

/// Equation of time (minutes).
fn equation_of_time(jc: f64) -> f64 {
    let e = eccentricity(jc);
    let l0 = sun_mean_longitude(jc) * DEG_TO_RAD;
    let m = sun_mean_anomaly(jc) * DEG_TO_RAD;
    let obl = obliquity_corrected(jc) * DEG_TO_RAD;

    let y = (obl / 2.0).tan().powi(2);

    let eq = y * (2.0 * l0).sin()
        - 2.0 * e * m.sin()
        + 4.0 * e * y * m.sin() * (2.0 * l0).cos()
        - 0.5 * y * y * (4.0 * l0).sin()
        - 1.25 * e * e * (2.0 * m).sin();

    4.0 * eq * RAD_TO_DEG // convert radians to minutes (4 min per degree)
}

/// Hour angle for sun at given angle below horizon (degrees).
/// Returns the hour angle in hours. angle is positive (e.g. 18 for Fajr).
fn hour_angle(lat: f64, decl: f64, angle: f64) -> f64 {
    let lat_r = lat * DEG_TO_RAD;
    let decl_r = decl * DEG_TO_RAD;

    let cos_ha =
        ((-angle * DEG_TO_RAD).sin() - lat_r.sin() * decl_r.sin()) / (lat_r.cos() * decl_r.cos());

    // Clamp to [-1, 1] for extreme latitudes
    let cos_ha = cos_ha.clamp(-1.0, 1.0);

    cos_ha.acos() * RAD_TO_DEG / 15.0 // convert degrees to hours
}

/// Hour angle for sunrise/sunset (standard: 0.833 degrees for atmospheric refraction + solar disc).
fn hour_angle_sunrise(lat: f64, decl: f64) -> f64 {
    hour_angle(lat, decl, 0.833)
}

/// Asr time using Shafi'i method (shadow = object length + 1).
fn asr_hour_angle(lat: f64, decl: f64) -> f64 {
    let lat_r = lat * DEG_TO_RAD;
    let decl_r = decl * DEG_TO_RAD;

    // Shadow factor for Shafi'i: 1 + tan(|lat - decl|)
    // The asr shadow ratio: when shadow of object = its length + noontime shadow
    // cot(asr_angle) = 1 + cot(|lat - decl|) ... but the standard formula:
    // asr_angle = acot(1 + tan(|lat - decl|))
    let diff = (lat - decl).abs() * DEG_TO_RAD;
    let asr_alt = (1.0 / (1.0 + diff.tan())).atan(); // altitude angle for asr

    let cos_ha = (asr_alt.sin() - lat_r.sin() * decl_r.sin()) / (lat_r.cos() * decl_r.cos());
    let cos_ha = cos_ha.clamp(-1.0, 1.0);

    cos_ha.acos() * RAD_TO_DEG / 15.0
}

/// Calculate prayer times for a given date, location, timezone offset, and method.
///
/// - `date`: the Gregorian date
/// - `lat`: latitude in degrees (positive = North)
/// - `lon`: longitude in degrees (positive = East)
/// - `timezone`: UTC offset in hours (e.g. +8.0 for Malaysia)
/// - `method`: the calculation method to use
pub fn calculate(
    date: NaiveDate,
    lat: f64,
    lon: f64,
    timezone: f64,
    method: CalculationMethod,
) -> PrayerTimes {
    let jd = julian_day(date);
    // Julian century for noon on this day
    let jc = (jd - 2451545.0) / 36525.0;

    let decl = sun_declination(jc);
    let eot = equation_of_time(jc);
    let params = method.params();

    // Solar noon in local time (hours)
    let noon = 12.0 + timezone - lon / 15.0 - eot / 60.0;

    // Sunrise & sunset
    let ha_sun = hour_angle_sunrise(lat, decl);
    let sunrise = noon - ha_sun;
    let sunset = noon + ha_sun;

    // Fajr
    let ha_fajr = hour_angle(lat, decl, params.fajr_angle);
    let fajr = noon - ha_fajr;

    // Dhuhr (a minute or two after solar noon for safety)
    let dhuhr = noon + 1.0 / 60.0; // add ~1 minute

    // Asr (Shafi'i)
    let ha_asr = asr_hour_angle(lat, decl);
    let asr = noon + ha_asr;

    // Maghrib = sunset
    let maghrib = sunset;

    // Isha
    let isha = match params.isha_minutes {
        Some(mins) => maghrib + mins as f64 / 60.0,
        None => {
            let ha_isha = hour_angle(lat, decl, params.isha_angle);
            noon + ha_isha
        }
    };

    PrayerTimes {
        fajr: hours_to_hm(fajr),
        sunrise: hours_to_hm(sunrise),
        dhuhr: hours_to_hm(dhuhr),
        asr: hours_to_hm(asr),
        maghrib: hours_to_hm(maghrib),
        isha: hours_to_hm(isha),
        method,
        date,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_minutes(t: (u8, u8)) -> u16 {
        t.0 as u16 * 60 + t.1 as u16
    }

    #[test]
    fn test_kuala_lumpur_mwl() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();
        let times = calculate(date, 3.1390, 101.6869, 8.0, CalculationMethod::MWL);

        // Fajr around 5-7
        assert!(times.fajr.0 >= 5 && times.fajr.0 <= 7, "Fajr hour: {}", times.fajr.0);
        // Dhuhr around 12-14
        assert!(
            times.dhuhr.0 >= 12 && times.dhuhr.0 <= 14,
            "Dhuhr hour: {}",
            times.dhuhr.0
        );
        // Maghrib around 18-20
        assert!(
            times.maghrib.0 >= 18 && times.maghrib.0 <= 20,
            "Maghrib hour: {}",
            times.maghrib.0
        );

        assert!(times.is_ordered(), "Times not in order: {times:?}");
    }

    #[test]
    fn test_new_york_isna_summer() {
        // Summer in NY — Fajr can be very early
        let date = NaiveDate::from_ymd_opt(2026, 6, 21).unwrap();
        let times = calculate(date, 40.7128, -74.0060, -4.0, CalculationMethod::ISNA);

        // Fajr around 3-5 in summer
        assert!(times.fajr.0 >= 3 && times.fajr.0 <= 5, "Fajr hour: {}", times.fajr.0);
        assert!(times.is_ordered(), "Times not in order: {times:?}");
    }

    #[test]
    fn test_makkah_umm_al_qura() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();
        let times = calculate(date, 21.4225, 39.8262, 3.0, CalculationMethod::UmmAlQura);

        // Isha should be exactly 90 minutes after Maghrib (+-5 min tolerance)
        let maghrib_min = to_minutes(times.maghrib);
        let isha_min = to_minutes(times.isha);
        let diff = isha_min as i32 - maghrib_min as i32;
        assert!(
            (diff - 90).abs() <= 5,
            "Isha should be ~90min after Maghrib, got {diff} min diff"
        );

        assert!(times.is_ordered(), "Times not in order: {times:?}");
    }

    #[test]
    fn test_all_methods_ordered() {
        let date = NaiveDate::from_ymd_opt(2026, 3, 9).unwrap();
        for method in CalculationMethod::all() {
            // Test with KL coordinates
            let times = calculate(date, 3.1390, 101.6869, 8.0, *method);
            assert!(times.is_ordered(), "{method} times not in order: {times:?}");

            // Test with Makkah
            let times = calculate(date, 21.4225, 39.8262, 3.0, *method);
            assert!(times.is_ordered(), "{method} Makkah times not in order: {times:?}");

            // Test with London
            let times = calculate(date, 51.5074, -0.1278, 0.0, *method);
            assert!(times.is_ordered(), "{method} London times not in order: {times:?}");
        }
    }

    #[test]
    fn test_format_time() {
        assert_eq!(PrayerTimes::format_time((5, 3)), "05:03");
        assert_eq!(PrayerTimes::format_time((13, 45)), "13:45");
        assert_eq!(PrayerTimes::format_time((0, 0)), "00:00");
        assert_eq!(PrayerTimes::format_time((23, 59)), "23:59");
    }

    #[test]
    fn test_from_str_loose() {
        assert_eq!(CalculationMethod::from_str_loose("mwl"), Some(CalculationMethod::MWL));
        assert_eq!(CalculationMethod::from_str_loose("MWL"), Some(CalculationMethod::MWL));
        assert_eq!(CalculationMethod::from_str_loose("isna"), Some(CalculationMethod::ISNA));
        assert_eq!(
            CalculationMethod::from_str_loose("egyptian"),
            Some(CalculationMethod::Egyptian)
        );
        assert_eq!(
            CalculationMethod::from_str_loose("karachi"),
            Some(CalculationMethod::Karachi)
        );
        assert_eq!(
            CalculationMethod::from_str_loose("umm_al_qura"),
            Some(CalculationMethod::UmmAlQura)
        );
        assert_eq!(
            CalculationMethod::from_str_loose("Umm Al Qura"),
            Some(CalculationMethod::UmmAlQura)
        );
        assert_eq!(
            CalculationMethod::from_str_loose("makkah"),
            Some(CalculationMethod::UmmAlQura)
        );
        assert_eq!(CalculationMethod::from_str_loose("jakim"), Some(CalculationMethod::JAKIM));
        assert_eq!(CalculationMethod::from_str_loose("invalid"), None);
    }

    #[test]
    fn test_julian_day() {
        // Known value: Jan 1, 2000 noon = JD 2451545.0
        let jd = julian_day(NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
        assert!((jd - 2451544.5).abs() < 0.01, "JD for 2000-01-01: {jd}");
    }
}
