use serde::Deserialize;

const ESOLAT_API: &str = "https://www.e-solat.gov.my/index.php";

#[derive(Debug, Deserialize)]
pub struct PrayerTimesResponse {
    #[serde(rename = "prayerTime")]
    pub prayer_time: Option<Vec<PrayerTime>>,
    #[serde(default)]
    #[allow(dead_code)]
    pub status: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PrayerTime {
    pub hijri: String,
    pub date: String,
    pub day: String,
    pub imsak: String,
    pub fajr: String,
    pub syuruk: String,
    pub dhuhr: String,
    pub asr: String,
    pub maghrib: String,
    pub isha: String,
}

pub async fn fetch_prayer_times(zone: &str) -> Result<Vec<PrayerTime>, String> {
    let url = format!(
        "{}?r=esolatApi/takwimsolat&period=today&zone={}",
        ESOLAT_API, zone
    );
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let data: PrayerTimesResponse = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    match data.prayer_time {
        Some(times) if !times.is_empty() => Ok(times),
        _ => Err(format!("No prayer times found for zone {}", zone)),
    }
}
