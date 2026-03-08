# Islamic Community Platform Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add 11 Islamic/Malaysian skills and multi-community support to AmanClaw.

**Architecture:** 5 Rust built-in skills (solat, quran, qiblat, hijri, doa) + 6 Python script plugins (hadith, halal, zakat, masjid, khutbah, jakim) + community model in SQLite + notification scheduler.

**Tech Stack:** Rust (tokio, reqwest, sqlx, serde), Python (amanclaw_sdk, requests), SQLite, JAKIM e-Solat API, Quran.com API, sunnah.com API.

---

## Task 1: skill-solat (Rust) — Prayer Times

**Files:**
- Create: `rust/plugins/skill-solat/Cargo.toml`
- Create: `rust/plugins/skill-solat/src/lib.rs`
- Create: `rust/plugins/skill-solat/src/jakim.rs`
- Create: `rust/plugins/skill-solat/src/zones.rs`
- Modify: `rust/Cargo.toml` (add workspace member)
- Modify: `rust/crates/amanclaw-core/src/lib.rs` (register skill)
- Modify: `rust/crates/amanclaw-core/Cargo.toml` (add dependency)

**Step 1: Create zones.rs with all JAKIM prayer zones**

```rust
// rust/plugins/skill-solat/src/zones.rs
use std::collections::HashMap;

pub struct Zone {
    pub code: &'static str,
    pub state: &'static str,
    pub areas: &'static str,
}

pub fn get_all_zones() -> Vec<Zone> {
    vec![
        // Johor
        Zone { code: "JHR01", state: "Johor", areas: "Pulau Aur, Pemanggil" },
        Zone { code: "JHR02", state: "Johor", areas: "Johor Bahru, Kota Tinggi, Mersing, Kulai" },
        Zone { code: "JHR03", state: "Johor", areas: "Kluang, Pontian" },
        Zone { code: "JHR04", state: "Johor", areas: "Batu Pahat, Muar, Segamat, Gemas" },
        // Kedah
        Zone { code: "KDH01", state: "Kedah", areas: "Kota Setar, Kubang Pasu, Pokok Sena" },
        Zone { code: "KDH02", state: "Kedah", areas: "Kuala Muda, Yan, Pendang" },
        Zone { code: "KDH03", state: "Kedah", areas: "Padang Terap, Sik" },
        Zone { code: "KDH04", state: "Kedah", areas: "Baling" },
        Zone { code: "KDH05", state: "Kedah", areas: "Kulim, Bandar Baharu" },
        Zone { code: "KDH06", state: "Kedah", areas: "Langkawi" },
        Zone { code: "KDH07", state: "Kedah", areas: "Gunung Jerai" },
        // Kelantan
        Zone { code: "KTN01", state: "Kelantan", areas: "Kota Bharu, Bachok, Pasir Puteh, Tumpat, Pasir Mas, Tanah Merah, Machang, Kuala Krai, Mukim Chiku" },
        Zone { code: "KTN02", state: "Kelantan", areas: "Gua Musang (Daerah Galas, Bertam), Jeli" },
        // Melaka
        Zone { code: "MLK01", state: "Melaka", areas: "Seluruh Negeri Melaka" },
        // Negeri Sembilan
        Zone { code: "NGS01", state: "Negeri Sembilan", areas: "Tampin, Jempol" },
        Zone { code: "NGS02", state: "Negeri Sembilan", areas: "Port Dickson, Seremban, Kuala Pilah, Jelebu, Rembau" },
        // Pahang
        Zone { code: "PHG01", state: "Pahang", areas: "Pulau Tioman" },
        Zone { code: "PHG02", state: "Pahang", areas: "Kuantan, Pekan, Rompin, Muadzam Shah" },
        Zone { code: "PHG03", state: "Pahang", areas: "Jerantut, Temerloh, Maran, Bera, Chenor, Jengka" },
        Zone { code: "PHG04", state: "Pahang", areas: "Bentong, Lipis, Raub" },
        Zone { code: "PHG05", state: "Pahang", areas: "Genting Highlands, Cameron Highlands" },
        // Perlis
        Zone { code: "PLS01", state: "Perlis", areas: "Seluruh Negeri Perlis" },
        // Pulau Pinang
        Zone { code: "PNG01", state: "Pulau Pinang", areas: "Seluruh Negeri Pulau Pinang" },
        // Perak
        Zone { code: "PRK01", state: "Perak", areas: "Tapah, Slim River, Tanjung Malim" },
        Zone { code: "PRK02", state: "Perak", areas: "Kuala Kangsar, Sg. Siput, Ipoh, Kampar, Batu Gajah, Hulu Perak" },
        Zone { code: "PRK03", state: "Perak", areas: "Lenggong, Pengkalan Hulu, Grik" },
        Zone { code: "PRK04", state: "Perak", areas: "Temengor, Belum" },
        Zone { code: "PRK05", state: "Perak", areas: "Teluk Intan, Bagan Datoh, Kg. Gajah, Sri Iskandar, Beruas, Parit, Lumut, Sitiawan, Pulau Pangkor" },
        Zone { code: "PRK06", state: "Perak", areas: "Selama, Taiping, Bagan Serai, Parit Buntar" },
        Zone { code: "PRK07", state: "Perak", areas: "Bukit Larut" },
        // Sabah
        Zone { code: "SBH01", state: "Sabah", areas: "Sandakan, Tungku, Sungai Imam" },
        Zone { code: "SBH02", state: "Sabah", areas: "Pinangah, ## Keningau, Tambunan, Nabawan" },
        Zone { code: "SBH03", state: "Sabah", areas: "Lahad Datu, Silabukan, Kunak, Semporna, Tungku, Tawau" },
        Zone { code: "SBH04", state: "Sabah", areas: "Pensiangan, ## Sapulut" },
        Zone { code: "SBH05", state: "Sabah", areas: "Papar, Ranau, Kota Marudu, Tuaran, Penampang, Kota Belud" },
        Zone { code: "SBH06", state: "Sabah", areas: "Gunung Kinabalu" },
        Zone { code: "SBH07", state: "Sabah", areas: "Kota Kinabalu, Putatan, Menggatal" },
        Zone { code: "SBH08", state: "Sabah", areas: "Kudat, Pitas, Pulau Banggi" },
        Zone { code: "SBH09", state: "Sabah", areas: "Beaufort, Kuala Penyu, Sipitang, Tenom, Long Pasia" },
        // Sarawak
        Zone { code: "SWK01", state: "Sarawak", areas: "Limbang, Sundar, Trusan" },
        Zone { code: "SWK02", state: "Sarawak", areas: "Miri, Niah, Bekenu, Sibuti, Marudi" },
        Zone { code: "SWK03", state: "Sarawak", areas: "Tatau, Belaga, Kapit, Sebauh, Song, Bintulu" },
        Zone { code: "SWK04", state: "Sarawak", areas: "Sibu, Kanowit, Dalat, Mukah, Igan, Oya, Balingian, Julau, Daro, Sarikei" },
        Zone { code: "SWK05", state: "Sarawak", areas: "Samarahan, Sri Aman, Betong, Lubok Antu, Kabong, Lingga, Engkilili, Pusa" },
        Zone { code: "SWK06", state: "Sarawak", areas: "Kuching, Bau, Lundu, Sematan" },
        Zone { code: "SWK07", state: "Sarawak", areas: "Lawas, Merapok, Trusan" },
        Zone { code: "SWK08", state: "Sarawak", areas: "Saratok, Roban, Debak, Serembu" },
        Zone { code: "SWK09", state: "Sarawak", areas: "Serian" },
        // Selangor
        Zone { code: "SGR01", state: "Selangor", areas: "Gombak, Petaling, Sepang, Hulu Langat, Hulu Selangor, Shah Alam" },
        Zone { code: "SGR02", state: "Selangor", areas: "Kuala Selangor, Sabak Bernam" },
        Zone { code: "SGR03", state: "Selangor", areas: "Klang, Kuala Langat" },
        // Terengganu
        Zone { code: "TRG01", state: "Terengganu", areas: "Kuala Terengganu, Marang, Kuala Nerus" },
        Zone { code: "TRG02", state: "Terengganu", areas: "Besut, Setiu" },
        Zone { code: "TRG03", state: "Terengganu", areas: "Hulu Terengganu" },
        Zone { code: "TRG04", state: "Terengganu", areas: "Dungun, Kemaman" },
        // WP
        Zone { code: "WLY01", state: "WP Kuala Lumpur", areas: "Kuala Lumpur, Putrajaya" },
        Zone { code: "WLY02", state: "WP Labuan", areas: "Labuan" },
    ]
}

pub fn find_zone(code: &str) -> Option<Zone> {
    get_all_zones().into_iter().find(|z| z.code.eq_ignore_ascii_case(code))
}

pub fn zones_by_state(state: &str) -> Vec<Zone> {
    get_all_zones().into_iter().filter(|z| z.state.eq_ignore_ascii_case(state)).collect()
}

pub fn get_states() -> Vec<&'static str> {
    vec![
        "Johor", "Kedah", "Kelantan", "Melaka", "Negeri Sembilan",
        "Pahang", "Perak", "Perlis", "Pulau Pinang", "Sabah",
        "Sarawak", "Selangor", "Terengganu",
        "WP Kuala Lumpur", "WP Labuan",
    ]
}
```

**Step 2: Create jakim.rs with e-Solat API client**

```rust
// rust/plugins/skill-solat/src/jakim.rs
use serde::Deserialize;

const ESOLAT_API: &str = "https://www.e-solat.gov.my/index.php";

#[derive(Debug, Deserialize)]
pub struct PrayerTimesResponse {
    #[serde(rename = "ppirayerTime")]
    pub prayer_time: Option<Vec<PrayerTime>>,
    #[serde(rename = "status")]
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
```

**Step 3: Create skill-solat/src/lib.rs**

```rust
// rust/plugins/skill-solat/src/lib.rs
mod jakim;
mod zones;

use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};

pub struct SolatSkill;

#[async_trait::async_trait]
impl Skill for SolatSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "solat".into(),
            description: "Get Malaysian prayer times (waktu solat) by JAKIM zone. Supports all zones in Malaysia.".into(),
            timeout_ms: 15000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "zone": {
                    "type": "string",
                    "description": "JAKIM zone code e.g. SGR01, WLY01, JHR02. If not provided, lists available states and zones."
                },
                "action": {
                    "type": "string",
                    "enum": ["today", "list_zones", "list_states"],
                    "description": "Action to perform. Default: today"
                }
            },
            "required": []
        })
    }

    async fn execute(&self, input: SkillInput) -> SkillResult {
        let args: serde_json::Value = match serde_json::from_str(&input.args) {
            Ok(v) => v,
            Err(e) => return SkillResult { success: false, output: String::new(), error: Some(format!("Invalid args: {}", e)) },
        };

        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("today");

        match action {
            "list_states" => {
                let states = zones::get_states();
                SkillResult {
                    success: true,
                    output: format!("Negeri-negeri di Malaysia:\n{}", states.join("\n")),
                    error: None,
                }
            }
            "list_zones" => {
                let state = args.get("zone").and_then(|v| v.as_str()).unwrap_or("");
                let zones = if state.is_empty() {
                    zones::get_all_zones()
                } else {
                    zones::zones_by_state(state)
                };
                let list: Vec<String> = zones.iter().map(|z| format!("{}: {} ({})", z.code, z.state, z.areas)).collect();
                SkillResult {
                    success: true,
                    output: if list.is_empty() {
                        format!("No zones found for '{}'. Use action=list_states to see available states.", state)
                    } else {
                        list.join("\n")
                    },
                    error: None,
                }
            }
            _ => {
                let zone = match args.get("zone").and_then(|v| v.as_str()) {
                    Some(z) => z,
                    None => return SkillResult {
                        success: false,
                        output: String::new(),
                        error: Some("Zone required. Example: SGR01, WLY01. Use action=list_zones to see all zones.".into()),
                    },
                };

                if zones::find_zone(zone).is_none() {
                    return SkillResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Unknown zone '{}'. Use action=list_zones to see valid zones.", zone)),
                    };
                }

                match jakim::fetch_prayer_times(zone).await {
                    Ok(times) => {
                        let t = &times[0];
                        let output = format!(
                            "Waktu Solat {} ({}):\n\nImsak: {}\nSubuh: {}\nSyuruk: {}\nZohor: {}\nAsar: {}\nMaghrib: {}\nIsyak: {}\n\nTarikh: {} | Hijri: {}",
                            zone, t.day, t.imsak, t.fajr, t.syuruk, t.dhuhr, t.asr, t.maghrib, t.isha, t.date, t.hijri
                        );
                        SkillResult { success: true, output, error: None }
                    }
                    Err(e) => SkillResult { success: false, output: String::new(), error: Some(e) },
                }
            }
        }
    }
}
```

**Step 4: Create Cargo.toml**

```toml
# rust/plugins/skill-solat/Cargo.toml
[package]
name = "amanclaw-skill-solat"
version = "0.1.0"
edition = "2021"

[dependencies]
amanclaw-traits = { path = "../../crates/amanclaw-traits" }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["rt"] }
```

**Step 5: Add to workspace and register**

In `rust/Cargo.toml`, add `"plugins/skill-solat"` to workspace members.

In `rust/crates/amanclaw-core/Cargo.toml`, add:
```toml
amanclaw-skill-solat = { path = "../../plugins/skill-solat" }
```

In `rust/crates/amanclaw-core/src/lib.rs`, after existing skill registrations:
```rust
registry.register(Arc::new(amanclaw_skill_solat::SolatSkill));
```

**Step 6: Build and verify**

Run: `cd rust && cargo build 2>&1 | tail -20`
Expected: Compiles successfully.

**Step 7: Commit**

```bash
git add rust/plugins/skill-solat/ rust/Cargo.toml rust/crates/amanclaw-core/
git commit -m "feat: add skill-solat with JAKIM e-Solat API integration"
```

---

## Task 2: skill-qiblat (Rust) — Qiblat Direction

**Files:**
- Create: `rust/plugins/skill-qiblat/Cargo.toml`
- Create: `rust/plugins/skill-qiblat/src/lib.rs`
- Modify: `rust/Cargo.toml`
- Modify: `rust/crates/amanclaw-core/src/lib.rs`
- Modify: `rust/crates/amanclaw-core/Cargo.toml`

**Step 1: Create lib.rs with Great Circle calculation**

```rust
// rust/plugins/skill-qiblat/src/lib.rs
use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};

pub struct QiblatSkill;

const KAABA_LAT: f64 = 21.4225;
const KAABA_LON: f64 = 39.8262;

fn calculate_qiblat(lat: f64, lon: f64) -> (f64, String) {
    let lat_rad = lat.to_radians();
    let lon_rad = lon.to_radians();
    let kaaba_lat_rad = KAABA_LAT.to_radians();
    let kaaba_lon_rad = KAABA_LON.to_radians();

    let delta_lon = kaaba_lon_rad - lon_rad;
    let x = delta_lon.sin() * kaaba_lat_rad.cos();
    let y = lat_rad.cos() * kaaba_lat_rad.sin()
        - lat_rad.sin() * kaaba_lat_rad.cos() * delta_lon.cos();

    let bearing = x.atan2(y).to_degrees();
    let bearing = (bearing + 360.0) % 360.0;

    let compass = match bearing as u32 {
        0..=22 | 338..=360 => "Utara (N)",
        23..=67 => "Timur Laut (NE)",
        68..=112 => "Timur (E)",
        113..=157 => "Tenggara (SE)",
        158..=202 => "Selatan (S)",
        203..=247 => "Barat Daya (SW)",
        248..=292 => "Barat (W)",
        293..=337 => "Barat Laut (NW)",
        _ => "Unknown",
    };

    (bearing, compass.to_string())
}

fn calculate_distance_km(lat: f64, lon: f64) -> f64 {
    let r = 6371.0;
    let dlat = (KAABA_LAT - lat).to_radians();
    let dlon = (KAABA_LON - lon).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat.to_radians().cos() * KAABA_LAT.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    r * c
}

#[async_trait::async_trait]
impl Skill for QiblatSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "qiblat".into(),
            description: "Calculate Qiblat direction (arah kiblat) from any location to Kaaba in Makkah.".into(),
            timeout_ms: 5000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "latitude": { "type": "number", "description": "Latitude of current location" },
                "longitude": { "type": "number", "description": "Longitude of current location" },
                "location": { "type": "string", "description": "Location name e.g. 'Kuala Lumpur', 'Shah Alam'. Used if lat/lon not provided." }
            },
            "required": []
        })
    }

    async fn execute(&self, input: SkillInput) -> SkillResult {
        let args: serde_json::Value = match serde_json::from_str(&input.args) {
            Ok(v) => v,
            Err(e) => return SkillResult { success: false, output: String::new(), error: Some(format!("Invalid args: {}", e)) },
        };

        // Default to KL if no coordinates given
        let lat = args.get("latitude").and_then(|v| v.as_f64()).unwrap_or(3.1390);
        let lon = args.get("longitude").and_then(|v| v.as_f64()).unwrap_or(101.6869);
        let location = args.get("location").and_then(|v| v.as_str()).unwrap_or("Kuala Lumpur");

        let (bearing, compass) = calculate_qiblat(lat, lon);
        let distance = calculate_distance_km(lat, lon);

        let output = format!(
            "Arah Kiblat dari {}:\n\nBearing: {:.1}°\nArah: {}\nJarak ke Kaabah: {:.0} km\n\nKoordinat: {:.4}°N, {:.4}°E",
            location, bearing, compass, distance, lat, lon
        );

        SkillResult { success: true, output, error: None }
    }
}
```

**Step 2: Create Cargo.toml**

```toml
[package]
name = "amanclaw-skill-qiblat"
version = "0.1.0"
edition = "2021"

[dependencies]
amanclaw-traits = { path = "../../crates/amanclaw-traits" }
async-trait = "0.1"
serde_json = "1"
```

**Step 3: Add to workspace, register, build, commit**

Same pattern as Task 1. Register with:
```rust
registry.register(Arc::new(amanclaw_skill_qiblat::QiblatSkill));
```

```bash
git commit -m "feat: add skill-qiblat with Great Circle bearing calculation"
```

---

## Task 3: skill-hijri (Rust) — Islamic Calendar

**Files:**
- Create: `rust/plugins/skill-hijri/Cargo.toml`
- Create: `rust/plugins/skill-hijri/src/lib.rs`
- Create: `rust/plugins/skill-hijri/src/converter.rs`
- Create: `rust/plugins/skill-hijri/src/events.rs`
- Modify: `rust/Cargo.toml`, `rust/crates/amanclaw-core/src/lib.rs`, `rust/crates/amanclaw-core/Cargo.toml`

**Step 1: Create converter.rs with Hijri date algorithm**

```rust
// rust/plugins/skill-hijri/src/converter.rs
use chrono::{Datelike, NaiveDate};

pub struct HijriDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub month_name_ar: &'static str,
    pub month_name_ms: &'static str,
}

const HIJRI_MONTHS_AR: [&str; 12] = [
    "Muharram", "Safar", "Rabi'ul Awal", "Rabi'ul Akhir",
    "Jamadil Awal", "Jamadil Akhir", "Rejab", "Sya'ban",
    "Ramadan", "Syawal", "Zulkaedah", "Zulhijjah",
];

const HIJRI_MONTHS_MS: [&str; 12] = [
    "Muharram", "Safar", "Rabiulawal", "Rabiulakhir",
    "Jamadilawal", "Jamadilakhir", "Rejab", "Syaaban",
    "Ramadan", "Syawal", "Zulkaedah", "Zulhijjah",
];

/// Gregorian to Hijri conversion (Kuwaiti algorithm)
pub fn gregorian_to_hijri(date: NaiveDate) -> HijriDate {
    let y = date.year();
    let m = date.month() as i32;
    let d = date.day() as i32;

    let jd = if m > 2 {
        let a = (y as f64 / 100.0).floor() as i32;
        let b = 2 - a + (a as f64 / 4.0).floor() as i32;
        (365.25 * (y + 4716) as f64).floor() as i32
            + (30.6001 * (m + 1) as f64).floor() as i32
            + d + b - 1524
    } else {
        let ny = y - 1;
        let nm = m + 12;
        let a = (ny as f64 / 100.0).floor() as i32;
        let b = 2 - a + (a as f64 / 4.0).floor() as i32;
        (365.25 * (ny + 4716) as f64).floor() as i32
            + (30.6001 * (nm + 1) as f64).floor() as i32
            + d + b - 1524
    };

    let l = jd - 1948440 + 10632;
    let n = ((l - 1) as f64 / 10631.0).floor() as i32;
    let l = l - 10631 * n + 354;
    let j = ((10985.0 - l as f64) / 5316.0).floor() as i32
        * ((50.0 * l as f64 / 17719.0).floor() as i32)
        + ((l as f64 / 5670.0).floor() as i32)
            * ((43.0 * l as f64 / 15238.0).floor() as i32);
    let l = l - ((30.0 - j as f64) / 15.0).floor() as i32
        * ((17719.0 + j as f64 * 15238.0) / 43.0).floor() as i32
        + (j as f64 / 16.0).floor() as i32
            * ((15238.0 - j as f64 * 5765.0) / 17719.0).floor() as i32;
    let month = ((100.0 * (l - 1) as f64 + 10985.0) / 5316.0).floor() as u32;
    let day = l - ((2959.0 * month as f64 - 29.0) / 100.0).floor() as i32;
    let year = 30 * n + j - 30;

    let mi = (month - 1).min(11) as usize;
    HijriDate {
        year,
        month,
        day: day as u32,
        month_name_ar: HIJRI_MONTHS_AR[mi],
        month_name_ms: HIJRI_MONTHS_MS[mi],
    }
}

impl std::fmt::Display for HijriDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.day, self.month_name_ms, self.year)
    }
}
```

**Step 2: Create events.rs with Islamic events**

```rust
// rust/plugins/skill-hijri/src/events.rs

pub struct IslamicEvent {
    pub month: u32,
    pub day: u32,
    pub name_ms: &'static str,
    pub name_en: &'static str,
}

pub fn get_events() -> Vec<IslamicEvent> {
    vec![
        IslamicEvent { month: 1, day: 1, name_ms: "Awal Muharram", name_en: "Islamic New Year" },
        IslamicEvent { month: 1, day: 10, name_ms: "Hari Asyura", name_en: "Day of Ashura" },
        IslamicEvent { month: 3, day: 12, name_ms: "Mawlidur Rasul", name_en: "Prophet's Birthday" },
        IslamicEvent { month: 7, day: 27, name_ms: "Israk & Mikraj", name_en: "Isra and Mi'raj" },
        IslamicEvent { month: 8, day: 15, name_ms: "Nisfu Sya'ban", name_en: "Mid-Sha'ban" },
        IslamicEvent { month: 9, day: 1, name_ms: "Ramadan Bermula", name_en: "Start of Ramadan" },
        IslamicEvent { month: 9, day: 17, name_ms: "Nuzul Al-Quran", name_en: "Revelation of Quran" },
        IslamicEvent { month: 9, day: 27, name_ms: "Lailatul Qadr", name_en: "Night of Power" },
        IslamicEvent { month: 10, day: 1, name_ms: "Hari Raya Aidilfitri", name_en: "Eid al-Fitr" },
        IslamicEvent { month: 12, day: 9, name_ms: "Hari Arafah", name_en: "Day of Arafah" },
        IslamicEvent { month: 12, day: 10, name_ms: "Hari Raya Haji", name_en: "Eid al-Adha" },
    ]
}
```

**Step 3: Create lib.rs**

```rust
// rust/plugins/skill-hijri/src/lib.rs
mod converter;
mod events;

use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};
use chrono::Local;

pub struct HijriSkill;

#[async_trait::async_trait]
impl Skill for HijriSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "hijri".into(),
            description: "Islamic (Hijri) calendar. Convert dates, check today's Hijri date, list upcoming Islamic events.".into(),
            timeout_ms: 5000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["today", "events", "convert"],
                    "description": "today = current Hijri date, events = upcoming Islamic events, convert = convert a Gregorian date"
                },
                "date": {
                    "type": "string",
                    "description": "Gregorian date to convert (YYYY-MM-DD). Only for action=convert."
                }
            },
            "required": []
        })
    }

    async fn execute(&self, input: SkillInput) -> SkillResult {
        let args: serde_json::Value = serde_json::from_str(&input.args).unwrap_or_default();
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("today");

        match action {
            "today" => {
                let today = Local::now().date_naive();
                let hijri = converter::gregorian_to_hijri(today);
                let output = format!(
                    "Tarikh Hijri Hari Ini:\n\n{}\n\nMasihi: {}",
                    hijri, today.format("%d %B %Y")
                );
                SkillResult { success: true, output, error: None }
            }
            "events" => {
                let today = Local::now().date_naive();
                let hijri = converter::gregorian_to_hijri(today);
                let all_events = events::get_events();
                let mut upcoming: Vec<String> = Vec::new();
                for e in &all_events {
                    let months_away = if e.month >= hijri.month {
                        e.month - hijri.month
                    } else {
                        12 - hijri.month + e.month
                    };
                    if months_away <= 3 {
                        upcoming.push(format!("{} {} - {} ({})", e.day, converter::gregorian_to_hijri(today).month_name_ms, e.name_ms, e.name_en));
                    }
                }
                let output = if upcoming.is_empty() {
                    "Tiada peristiwa Islam dalam 3 bulan akan datang.".into()
                } else {
                    format!("Peristiwa Islam akan datang:\n\n{}", upcoming.join("\n"))
                };
                SkillResult { success: true, output, error: None }
            }
            "convert" => {
                let date_str = args.get("date").and_then(|v| v.as_str()).unwrap_or("");
                match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    Ok(date) => {
                        let hijri = converter::gregorian_to_hijri(date);
                        SkillResult {
                            success: true,
                            output: format!("{} = {}", date.format("%d %B %Y"), hijri),
                            error: None,
                        }
                    }
                    Err(_) => SkillResult {
                        success: false,
                        output: String::new(),
                        error: Some("Invalid date format. Use YYYY-MM-DD.".into()),
                    },
                }
            }
            _ => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown action '{}'. Use: today, events, convert.", action)),
            },
        }
    }
}
```

**Step 4: Create Cargo.toml**

```toml
[package]
name = "amanclaw-skill-hijri"
version = "0.1.0"
edition = "2021"

[dependencies]
amanclaw-traits = { path = "../../crates/amanclaw-traits" }
async-trait = "0.1"
serde_json = "1"
chrono = "0.4"
```

**Step 5: Add to workspace, register, build, commit**

```bash
git commit -m "feat: add skill-hijri with Gregorian-Hijri conversion and Islamic events"
```

---

## Task 4: skill-doa (Rust) — Doa & Zikir Collection

**Files:**
- Create: `rust/plugins/skill-doa/Cargo.toml`
- Create: `rust/plugins/skill-doa/src/lib.rs`
- Create: `rust/plugins/skill-doa/src/collection.rs`
- Modify: `rust/Cargo.toml`, `rust/crates/amanclaw-core/src/lib.rs`, `rust/crates/amanclaw-core/Cargo.toml`

**Step 1: Create collection.rs with doa database**

```rust
// rust/plugins/skill-doa/src/collection.rs

pub struct Doa {
    pub id: u32,
    pub category: &'static str,
    pub title_ms: &'static str,
    pub title_en: &'static str,
    pub arabic: &'static str,
    pub transliteration: &'static str,
    pub translation_ms: &'static str,
    pub translation_en: &'static str,
    pub source: &'static str,
}

pub fn get_all_doa() -> Vec<Doa> {
    vec![
        // Doa Harian
        Doa {
            id: 1, category: "harian",
            title_ms: "Doa Sebelum Makan", title_en: "Before Eating",
            arabic: "بِسْمِ اللهِ وَعَلَى بَرَكَةِ اللهِ",
            transliteration: "Bismillahi wa 'ala barakatillah",
            translation_ms: "Dengan nama Allah dan dengan berkat Allah",
            translation_en: "In the name of Allah and with the blessings of Allah",
            source: "Abu Dawud",
        },
        Doa {
            id: 2, category: "harian",
            title_ms: "Doa Selepas Makan", title_en: "After Eating",
            arabic: "اَلْحَمْدُ لِلَّهِ الَّذِي أَطْعَمَنَا وَسَقَانَا وَجَعَلَنَا مُسْلِمِينَ",
            transliteration: "Alhamdulillahilladzi at'amana wa saqana wa ja'alana muslimin",
            translation_ms: "Segala puji bagi Allah yang memberi kami makan dan minum serta menjadikan kami orang Islam",
            translation_en: "Praise be to Allah who gave us food and drink and made us Muslims",
            source: "Abu Dawud, Tirmizi",
        },
        Doa {
            id: 3, category: "harian",
            title_ms: "Doa Sebelum Tidur", title_en: "Before Sleeping",
            arabic: "بِاسْمِكَ اللَّهُمَّ أَمُوتُ وَأَحْيَا",
            transliteration: "Bismika Allahumma amutu wa ahya",
            translation_ms: "Dengan nama-Mu ya Allah aku mati dan aku hidup",
            translation_en: "In Your name O Allah, I die and I live",
            source: "Bukhari",
        },
        Doa {
            id: 4, category: "harian",
            title_ms: "Doa Bangun Tidur", title_en: "Upon Waking Up",
            arabic: "اَلْحَمْدُ لِلَّهِ الَّذِي أَحْيَانَا بَعْدَ مَا أَمَاتَنَا وَإِلَيْهِ النُّشُورُ",
            transliteration: "Alhamdulillahilladzi ahyana ba'da ma amatana wa ilaihin nushur",
            translation_ms: "Segala puji bagi Allah yang menghidupkan kami setelah mematikan kami dan kepada-Nya kami dikembalikan",
            translation_en: "Praise be to Allah who gave us life after death and to Him is the return",
            source: "Bukhari",
        },
        Doa {
            id: 5, category: "harian",
            title_ms: "Doa Masuk Rumah", title_en: "Entering Home",
            arabic: "بِسْمِ اللهِ وَلَجْنَا وَبِسْمِ اللهِ خَرَجْنَا وَعَلَى اللهِ رَبِّنَا تَوَكَّلْنَا",
            transliteration: "Bismillahi walajna wa bismillahi kharajna wa 'alallahi rabbina tawakkalna",
            translation_ms: "Dengan nama Allah kami masuk, dengan nama Allah kami keluar, dan kepada Allah Tuhan kami, kami bertawakkal",
            translation_en: "In the name of Allah we enter, in the name of Allah we leave, and upon Allah our Lord we rely",
            source: "Abu Dawud",
        },
        Doa {
            id: 6, category: "harian",
            title_ms: "Doa Keluar Rumah", title_en: "Leaving Home",
            arabic: "بِسْمِ اللهِ تَوَكَّلْتُ عَلَى اللهِ لَا حَوْلَ وَلَا قُوَّةَ إِلَّا بِاللهِ",
            transliteration: "Bismillahi tawakkaltu 'alallahi la hawla wa la quwwata illa billah",
            translation_ms: "Dengan nama Allah, aku bertawakkal kepada Allah. Tiada daya dan kekuatan melainkan dengan Allah",
            translation_en: "In the name of Allah, I rely on Allah. There is no power nor strength except with Allah",
            source: "Abu Dawud, Tirmizi",
        },
        // Zikir Pagi
        Doa {
            id: 7, category: "pagi",
            title_ms: "Zikir Pagi - Ayat Kursi", title_en: "Morning - Ayatul Kursi",
            arabic: "اللَّهُ لَا إِلَهَ إِلَّا هُوَ الْحَيُّ الْقَيُّومُ",
            transliteration: "Allahu la ilaha illa huwal hayyul qayyum...",
            translation_ms: "Allah, tiada tuhan selain Dia, Yang Hidup, Yang Berdiri Sendiri",
            translation_en: "Allah, there is no deity except Him, the Ever-Living, the Sustainer",
            source: "Al-Baqarah 2:255",
        },
        Doa {
            id: 8, category: "pagi",
            title_ms: "Zikir Pagi - Sayyidul Istighfar", title_en: "Morning - Master Supplication for Forgiveness",
            arabic: "اَللَّهُمَّ أَنْتَ رَبِّي لَا إِلَهَ إِلَّا أَنْتَ خَلَقْتَنِي وَأَنَا عَبْدُكَ",
            transliteration: "Allahumma anta rabbi la ilaha illa anta khalaqtani wa ana 'abduk...",
            translation_ms: "Ya Allah, Engkau Tuhanku, tiada tuhan selain Engkau, Engkau menciptakan aku dan aku hamba-Mu",
            translation_en: "O Allah, You are my Lord, there is no deity except You, You created me and I am Your servant",
            source: "Bukhari",
        },
        // Musafir
        Doa {
            id: 9, category: "musafir",
            title_ms: "Doa Musafir / Naik Kenderaan", title_en: "Traveller's Prayer",
            arabic: "سُبْحَانَ الَّذِي سَخَّرَ لَنَا هَذَا وَمَا كُنَّا لَهُ مُقْرِنِينَ وَإِنَّا إِلَى رَبِّنَا لَمُنْقَلِبُونَ",
            transliteration: "Subhanalladzi sakhkhara lana hadha wa ma kunna lahu muqrinin wa inna ila rabbina lamunqalibun",
            translation_ms: "Maha Suci Tuhan yang memudahkan ini untuk kami sedangkan kami tidak mampu menguasainya dan sesungguhnya kami akan kembali kepada Tuhan kami",
            translation_en: "Glory to Him who has subjected this to us, we could never have it by our efforts, and to our Lord we shall return",
            source: "Az-Zukhruf 43:13-14",
        },
        // Solat
        Doa {
            id: 10, category: "solat",
            title_ms: "Doa Selepas Azan", title_en: "After Adhan",
            arabic: "اَللَّهُمَّ رَبَّ هَذِهِ الدَّعْوَةِ التَّامَّةِ وَالصَّلَاةِ الْقَائِمَةِ",
            transliteration: "Allahumma rabba hadhihid da'watit tammah was solatil qo'imah...",
            translation_ms: "Ya Allah, Tuhan yang memiliki seruan yang sempurna ini dan solat yang didirikan",
            translation_en: "O Allah, Lord of this perfect call and established prayer",
            source: "Bukhari",
        },
    ]
}

pub fn get_categories() -> Vec<(&'static str, &'static str)> {
    vec![
        ("harian", "Doa Harian / Daily"),
        ("pagi", "Zikir Pagi / Morning"),
        ("petang", "Zikir Petang / Evening"),
        ("solat", "Doa Solat / Prayer"),
        ("musafir", "Doa Musafir / Travel"),
        ("makan", "Doa Makan / Food"),
        ("tidur", "Doa Tidur / Sleep"),
        ("wudhu", "Doa Wudhu / Ablution"),
        ("masjid", "Doa Masjid / Mosque"),
        ("umum", "Doa Umum / General"),
    ]
}

pub fn search_doa(query: &str) -> Vec<&Doa> {
    let q = query.to_lowercase();
    get_all_doa().iter().filter(|d| {
        d.title_ms.to_lowercase().contains(&q)
            || d.title_en.to_lowercase().contains(&q)
            || d.category.to_lowercase().contains(&q)
            || d.transliteration.to_lowercase().contains(&q)
    }).collect()
    // Note: This won't work because get_all_doa() returns owned Vec
    // Will need to use a static or lazy_static in actual implementation
}
```

**Step 2: Create lib.rs implementing Skill trait**

Same pattern as previous skills. Parameters: `category` (string), `search` (string), `action` (enum: list_categories, search, random, by_category).

**Step 3: Cargo.toml, workspace, register, build, commit**

```bash
git commit -m "feat: add skill-doa with doa and zikir collection"
```

---

## Task 5: skill-quran (Rust) — Quran Search & Lookup

**Files:**
- Create: `rust/plugins/skill-quran/Cargo.toml`
- Create: `rust/plugins/skill-quran/src/lib.rs`
- Create: `rust/plugins/skill-quran/src/api.rs`
- Modify: `rust/Cargo.toml`, `rust/crates/amanclaw-core/src/lib.rs`, `rust/crates/amanclaw-core/Cargo.toml`

**Step 1: Create api.rs with Quran.com API client**

```rust
// rust/plugins/skill-quran/src/api.rs
use serde::Deserialize;

const QURAN_API: &str = "https://api.quran.com/api/v4";

#[derive(Debug, Deserialize)]
pub struct VerseResponse {
    pub verses: Vec<Verse>,
}

#[derive(Debug, Deserialize)]
pub struct Verse {
    pub id: u32,
    pub verse_key: String,
    pub text_uthmani: String,
    pub translations: Option<Vec<Translation>>,
}

#[derive(Debug, Deserialize)]
pub struct Translation {
    pub text: String,
    pub resource_name: String,
    pub language_name: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub search: SearchResult,
}

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub query: String,
    pub total_results: u32,
    pub results: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
pub struct SearchHit {
    pub verse_key: String,
    pub text: String,
    pub translations: Vec<Translation>,
}

/// Fetch a specific verse with Malay translation (ID 39 = Malay)
pub async fn get_verse(surah: u32, ayat: u32) -> Result<Verse, String> {
    let url = format!(
        "{}/verses/by_key/{}:{}?language=ms&translations=39,131&fields=text_uthmani",
        QURAN_API, surah, ayat
    );
    let resp = reqwest::get(&url).await.map_err(|e| format!("HTTP error: {}", e))?;
    let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse error: {}", e))?;

    let verse = data.get("verse").ok_or("No verse found")?;
    serde_json::from_value(verse.clone()).map_err(|e| format!("Deserialize error: {}", e))
}

/// Search Quran by keyword
pub async fn search(query: &str, language: &str) -> Result<SearchResult, String> {
    let url = format!(
        "{}/search?q={}&size=5&language={}",
        QURAN_API,
        urlencoding::encode(query),
        language
    );
    let resp = reqwest::get(&url).await.map_err(|e| format!("HTTP error: {}", e))?;
    let data: SearchResponse = resp.json().await.map_err(|e| format!("Parse error: {}", e))?;
    Ok(data.search)
}

/// Get a random verse (for daily verse feature)
pub async fn random_verse() -> Result<Verse, String> {
    let surah = (rand::random::<u32>() % 114) + 1;
    // Simple approach: get first ayat of random surah
    get_verse(surah, 1).await
}
```

**Step 2: Create lib.rs**

Skill with parameters: `surah` (number), `ayat` (number), `search` (string), `action` (enum: verse, search, random, surah_list).

**Step 3: Cargo.toml with deps: reqwest, urlencoding, rand, serde**

**Step 4: Workspace, register, build, commit**

```bash
git commit -m "feat: add skill-quran with Quran.com API for verse lookup and search"
```

---

## Task 6: skill-hadith (Python) — Hadith Search

**Files:**
- Create: `plugins/skill_hadith.py`

**Step 1: Create Python plugin**

```python
# plugins/skill_hadith.py
import json
import urllib.request
from amanclaw_sdk import plugin, SkillInput, SkillResult

SUNNAH_API = "https://api.sunnah.com/v1"

COLLECTIONS = {
    "bukhari": {"name": "Sahih al-Bukhari", "id": "bukhari"},
    "muslim": {"name": "Sahih Muslim", "id": "muslim"},
    "abudawud": {"name": "Sunan Abu Dawud", "id": "abudawud"},
    "tirmidhi": {"name": "Jami` at-Tirmidhi", "id": "tirmidhi"},
    "nasai": {"name": "Sunan an-Nasa'i", "id": "nasai"},
    "ibnmajah": {"name": "Sunan Ibn Majah", "id": "ibnmajah"},
}

def api_get(path, api_key=""):
    url = f"{SUNNAH_API}{path}"
    req = urllib.request.Request(url)
    if api_key:
        req.add_header("x-api-key", api_key)
    req.add_header("Accept", "application/json")
    with urllib.request.urlopen(req, timeout=15) as resp:
        return json.loads(resp.read().decode())

@plugin(
    name="hadith",
    description="Search and lookup hadith from major collections (Bukhari, Muslim, Abu Dawud, Tirmidhi, Nasai, Ibn Majah).",
    parameters={
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["search", "lookup", "random", "collections"],
                "description": "search = search by keyword, lookup = specific hadith by number, random = random hadith, collections = list available collections"
            },
            "query": {"type": "string", "description": "Search keyword"},
            "collection": {"type": "string", "description": "Collection name: bukhari, muslim, abudawud, tirmidhi, nasai, ibnmajah"},
            "hadith_number": {"type": "string", "description": "Hadith number for lookup"},
        },
        "required": [],
    },
)
def execute(input: SkillInput) -> SkillResult:
    args = input.parse_args()
    action = args.get("action", "search")
    api_key = input.env.get("SUNNAH_API_KEY", "")

    if action == "collections":
        lines = [f"- {k}: {v['name']}" for k, v in COLLECTIONS.items()]
        return SkillResult.ok("Koleksi Hadis:\n" + "\n".join(lines))

    if action == "lookup":
        collection = args.get("collection", "bukhari")
        number = args.get("hadith_number", "1")
        try:
            data = api_get(f"/collections/{collection}/hadiths/{number}", api_key)
            hadith = data.get("hadith", [data])[0] if isinstance(data, dict) else data
            text_ar = hadith.get("arabicText", hadith.get("body", ""))
            text_en = hadith.get("englishText", hadith.get("text", ""))
            return SkillResult.ok(
                f"📖 {COLLECTIONS.get(collection, {}).get('name', collection)} #{number}\n\n"
                f"Arab:\n{text_ar}\n\n"
                f"English:\n{text_en}"
            )
        except Exception as e:
            return SkillResult.err(f"Failed to lookup hadith: {e}")

    if action == "search":
        query = args.get("query", "")
        if not query:
            return SkillResult.err("Please provide a search query.")
        try:
            data = api_get(f"/hadiths?q={query}&limit=3", api_key)
            hadiths = data.get("data", [])
            if not hadiths:
                return SkillResult.ok(f"Tiada hadis ditemui untuk '{query}'.")
            results = []
            for h in hadiths[:3]:
                results.append(f"[{h.get('collection', '?')} #{h.get('hadithNumber', '?')}]\n{h.get('englishText', h.get('body', ''))[:200]}...")
            return SkillResult.ok(f"Hasil carian '{query}':\n\n" + "\n\n".join(results))
        except Exception as e:
            return SkillResult.err(f"Search failed: {e}")

    return SkillResult.err(f"Unknown action: {action}")

if __name__ == "__main__":
    execute.run()
```

**Step 2: Add to config.yaml**

```yaml
script_plugins:
  hadith:
    command: "python3"
    args: ["./plugins/skill_hadith.py"]
    env:
      SUNNAH_API_KEY: "${SUNNAH_API_KEY}"
```

**Step 3: Commit**

```bash
git add plugins/skill_hadith.py
git commit -m "feat: add skill-hadith Python plugin with sunnah.com API"
```

---

## Task 7: skill-halal (Python) — JAKIM Halal Verification

**Files:**
- Create: `plugins/skill_halal.py`

**Step 1: Create Python plugin**

```python
# plugins/skill_halal.py
import json
import urllib.request
import urllib.parse
from amanclaw_sdk import plugin, SkillInput, SkillResult

JAKIM_HALAL_URL = "https://www.halal.gov.my/v4/api"

def search_halal(query):
    """Search JAKIM halal directory"""
    encoded = urllib.parse.quote(query)
    url = f"{JAKIM_HALAL_URL}/search?keyword={encoded}&type=all"
    req = urllib.request.Request(url)
    req.add_header("Accept", "application/json")
    req.add_header("User-Agent", "AmanClaw/1.0")
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return json.loads(resp.read().decode())
    except Exception:
        # Fallback: scrape or use alternative endpoint
        return None

def verify_cert(cert_number):
    """Verify halal certificate by number"""
    url = f"{JAKIM_HALAL_URL}/verify?cert={urllib.parse.quote(cert_number)}"
    req = urllib.request.Request(url)
    req.add_header("Accept", "application/json")
    req.add_header("User-Agent", "AmanClaw/1.0")
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return json.loads(resp.read().decode())
    except Exception:
        return None

@plugin(
    name="halal",
    description="Check halal status of products, restaurants, and premises using JAKIM Malaysia halal database. Verify halal certificates.",
    parameters={
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["search", "verify"],
                "description": "search = search by product/restaurant name, verify = verify by certificate number"
            },
            "query": {"type": "string", "description": "Product, restaurant, or company name to search"},
            "cert_number": {"type": "string", "description": "JAKIM halal certificate number to verify"},
        },
        "required": [],
    },
)
def execute(input: SkillInput) -> SkillResult:
    args = input.parse_args()
    action = args.get("action", "search")

    if action == "verify":
        cert = args.get("cert_number", "")
        if not cert:
            return SkillResult.err("Sila berikan nombor sijil halal / Please provide certificate number.")
        data = verify_cert(cert)
        if data is None:
            return SkillResult.err("Gagal menghubungi pangkalan data JAKIM. Sila cuba lagi.")
        if data.get("valid"):
            return SkillResult.ok(
                f"Sijil Halal JAKIM: {cert}\n"
                f"Status: SAH / VALID\n"
                f"Syarikat: {data.get('company', 'N/A')}\n"
                f"Tamat: {data.get('expiry', 'N/A')}"
            )
        else:
            return SkillResult.ok(f"Sijil {cert}: TIDAK SAH / NOT VALID atau tidak ditemui.")

    # Default: search
    query = args.get("query", "")
    if not query:
        return SkillResult.err("Sila berikan nama produk/restoran untuk dicari.")
    data = search_halal(query)
    if data is None:
        return SkillResult.err("Gagal menghubungi pangkalan data JAKIM. Sila cuba lagi.")
    results = data.get("results", data.get("data", []))
    if not results:
        return SkillResult.ok(f"Tiada keputusan halal ditemui untuk '{query}'. Cuba nama lain atau semak di halal.gov.my.")
    lines = []
    for r in results[:5]:
        name = r.get("name", r.get("company", "N/A"))
        status = r.get("status", "N/A")
        cert = r.get("cert_number", r.get("certificate", "N/A"))
        expiry = r.get("expiry", r.get("valid_until", "N/A"))
        lines.append(f"- {name}\n  Sijil: {cert} | Status: {status} | Tamat: {expiry}")
    return SkillResult.ok(f"Keputusan carian halal untuk '{query}':\n\n" + "\n".join(lines))

if __name__ == "__main__":
    execute.run()
```

**Step 2: Add to config.yaml, commit**

```bash
git commit -m "feat: add skill-halal Python plugin with JAKIM halal database"
```

---

## Task 8: skill-zakat (Python) — Zakat Calculator

**Files:**
- Create: `plugins/skill_zakat.py`

**Step 1: Create Python plugin**

```python
# plugins/skill_zakat.py
from amanclaw_sdk import plugin, SkillInput, SkillResult

# 2024/2025 rates - update yearly from JAKIM/state zakat authorities
ZAKAT_FITRAH_RATES = {
    "default": 7.00,
    "WLY": 7.00, "SGR": 7.00, "JHR": 7.00, "PHG": 7.00,
    "PRK": 7.00, "KDH": 7.00, "KTN": 7.00, "TRG": 7.00,
    "PNG": 7.00, "PLS": 7.00, "MLK": 7.00, "NGS": 7.00,
    "SBH": 7.00, "SWK": 7.00,
}

NISAB_GOLD_GRAMS = 85.0  # 85 grams of gold

@plugin(
    name="zakat",
    description="Calculate zakat (Islamic tax). Supports zakat fitrah, zakat pendapatan (income), zakat simpanan (savings), and zakat emas (gold).",
    parameters={
        "type": "object",
        "properties": {
            "type": {
                "type": "string",
                "enum": ["fitrah", "pendapatan", "simpanan", "emas", "info"],
                "description": "Type of zakat to calculate"
            },
            "state": {"type": "string", "description": "State code for fitrah rate (e.g., WLY, SGR, JHR)"},
            "dependents": {"type": "integer", "description": "Number of dependents for fitrah"},
            "annual_income": {"type": "number", "description": "Annual gross income (RM) for zakat pendapatan"},
            "annual_expenses": {"type": "number", "description": "Annual allowable expenses/deductions (RM)"},
            "savings_balance": {"type": "number", "description": "Lowest savings balance in the year (RM)"},
            "gold_grams": {"type": "number", "description": "Gold weight in grams for zakat emas"},
            "gold_price_per_gram": {"type": "number", "description": "Current gold price per gram (RM)"},
        },
        "required": [],
    },
)
def execute(input: SkillInput) -> SkillResult:
    args = input.parse_args()
    zakat_type = args.get("type", "info")

    if zakat_type == "info":
        return SkillResult.ok(
            "Jenis-jenis Zakat:\n\n"
            "1. Zakat Fitrah - wajib setiap Muslim bulan Ramadan\n"
            "2. Zakat Pendapatan - 2.5% dari pendapatan bersih tahunan\n"
            "3. Zakat Simpanan - 2.5% dari simpanan yang cukup nisab & haul\n"
            "4. Zakat Emas - 2.5% dari emas yang cukup nisab (85g)\n\n"
            "Guna type=fitrah/pendapatan/simpanan/emas untuk pengiraan."
        )

    if zakat_type == "fitrah":
        state = args.get("state", "WLY").upper()
        dependents = args.get("dependents", 1)
        rate = ZAKAT_FITRAH_RATES.get(state, ZAKAT_FITRAH_RATES["default"])
        total = rate * dependents
        return SkillResult.ok(
            f"Zakat Fitrah ({state}):\n\n"
            f"Kadar: RM {rate:.2f} seorang\n"
            f"Bilangan tanggungan: {dependents}\n"
            f"Jumlah: RM {total:.2f}"
        )

    if zakat_type == "pendapatan":
        income = args.get("annual_income", 0)
        expenses = args.get("annual_expenses", 0)
        if income <= 0:
            return SkillResult.err("Sila masukkan pendapatan tahunan (annual_income).")
        net = income - expenses
        zakat = max(0, net * 0.025)
        monthly = zakat / 12
        return SkillResult.ok(
            f"Zakat Pendapatan:\n\n"
            f"Pendapatan tahunan: RM {income:,.2f}\n"
            f"Tolakan: RM {expenses:,.2f}\n"
            f"Pendapatan bersih: RM {net:,.2f}\n"
            f"Zakat (2.5%): RM {zakat:,.2f}\n"
            f"Anggaran bulanan: RM {monthly:,.2f}"
        )

    if zakat_type == "simpanan":
        balance = args.get("savings_balance", 0)
        if balance <= 0:
            return SkillResult.err("Sila masukkan baki simpanan terendah dalam setahun (savings_balance).")
        gold_price = args.get("gold_price_per_gram", 400.0)
        nisab = NISAB_GOLD_GRAMS * gold_price
        if balance < nisab:
            return SkillResult.ok(
                f"Zakat Simpanan:\n\n"
                f"Baki terendah: RM {balance:,.2f}\n"
                f"Nisab (85g emas @ RM{gold_price:.2f}/g): RM {nisab:,.2f}\n"
                f"Status: TIDAK WAJIB (baki < nisab)"
            )
        zakat = balance * 0.025
        return SkillResult.ok(
            f"Zakat Simpanan:\n\n"
            f"Baki terendah: RM {balance:,.2f}\n"
            f"Nisab: RM {nisab:,.2f}\n"
            f"Status: WAJIB\n"
            f"Zakat (2.5%): RM {zakat:,.2f}"
        )

    if zakat_type == "emas":
        grams = args.get("gold_grams", 0)
        price = args.get("gold_price_per_gram", 400.0)
        if grams <= 0:
            return SkillResult.err("Sila masukkan berat emas dalam gram (gold_grams).")
        value = grams * price
        if grams < NISAB_GOLD_GRAMS:
            return SkillResult.ok(
                f"Zakat Emas:\n\n"
                f"Berat: {grams:.1f}g\n"
                f"Nisab: {NISAB_GOLD_GRAMS:.0f}g\n"
                f"Status: TIDAK WAJIB (berat < nisab)"
            )
        zakat = value * 0.025
        return SkillResult.ok(
            f"Zakat Emas:\n\n"
            f"Berat: {grams:.1f}g @ RM{price:.2f}/g\n"
            f"Nilai: RM {value:,.2f}\n"
            f"Zakat (2.5%): RM {zakat:,.2f}"
        )

    return SkillResult.err(f"Jenis zakat tidak dikenali: {zakat_type}")

if __name__ == "__main__":
    execute.run()
```

**Step 2: Add to config.yaml, commit**

```bash
git commit -m "feat: add skill-zakat Python plugin with fitrah, pendapatan, simpanan, emas calculators"
```

---

## Task 9: skill-masjid (Python) — Mosque Finder

**Files:**
- Create: `plugins/skill_masjid.py`

**Step 1: Create Python plugin using Google Places API**

```python
# plugins/skill_masjid.py
import json
import urllib.request
import urllib.parse
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="masjid",
    description="Find nearest masjid (mosque) or surau from a location in Malaysia using Google Places API.",
    parameters={
        "type": "object",
        "properties": {
            "latitude": {"type": "number", "description": "Latitude of current location"},
            "longitude": {"type": "number", "description": "Longitude of current location"},
            "location": {"type": "string", "description": "Location name e.g. 'Shah Alam', 'KLCC'. Used if lat/lon not provided."},
            "radius": {"type": "integer", "description": "Search radius in meters (default: 2000)"},
        },
        "required": [],
    },
)
def execute(input: SkillInput) -> SkillResult:
    args = input.parse_args()
    api_key = input.env.get("GOOGLE_PLACES_API_KEY", "")

    if not api_key:
        return SkillResult.err("GOOGLE_PLACES_API_KEY not configured.")

    lat = args.get("latitude")
    lon = args.get("longitude")
    location = args.get("location", "")
    radius = args.get("radius", 2000)

    # Geocode location name if no coordinates
    if (lat is None or lon is None) and location:
        geo_url = f"https://maps.googleapis.com/maps/api/geocode/json?address={urllib.parse.quote(location + ', Malaysia')}&key={api_key}"
        req = urllib.request.Request(geo_url)
        try:
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = json.loads(resp.read().decode())
                if data.get("results"):
                    loc = data["results"][0]["geometry"]["location"]
                    lat, lon = loc["lat"], loc["lng"]
        except Exception as e:
            return SkillResult.err(f"Gagal geocode lokasi: {e}")

    if lat is None or lon is None:
        return SkillResult.err("Sila berikan lokasi (latitude/longitude atau nama tempat).")

    # Search for mosques nearby
    places_url = (
        f"https://maps.googleapis.com/maps/api/place/nearbysearch/json"
        f"?location={lat},{lon}&radius={radius}&type=mosque&key={api_key}&language=ms"
    )
    req = urllib.request.Request(places_url)
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = json.loads(resp.read().decode())
    except Exception as e:
        return SkillResult.err(f"Gagal mencari masjid: {e}")

    results = data.get("results", [])
    if not results:
        return SkillResult.ok(f"Tiada masjid/surau ditemui dalam radius {radius}m dari lokasi anda. Cuba tingkatkan radius.")

    lines = []
    for r in results[:5]:
        name = r.get("name", "N/A")
        addr = r.get("vicinity", "N/A")
        rating = r.get("rating", "N/A")
        status = "Buka" if r.get("opening_hours", {}).get("open_now") else "Tutup/Tidak pasti"
        rlat = r["geometry"]["location"]["lat"]
        rlon = r["geometry"]["location"]["lng"]
        maps_link = f"https://maps.google.com/?q={rlat},{rlon}"
        lines.append(f"- {name}\n  Alamat: {addr}\n  Rating: {rating} | Status: {status}\n  Maps: {maps_link}")

    return SkillResult.ok(
        f"Masjid/Surau berhampiran ({len(results)} ditemui):\n\n" + "\n\n".join(lines)
    )

if __name__ == "__main__":
    execute.run()
```

**Step 2: Add to config.yaml with GOOGLE_PLACES_API_KEY env, commit**

```bash
git commit -m "feat: add skill-masjid Python plugin with Google Places mosque finder"
```

---

## Task 10: skill-khutbah (Python) — Weekly Khutbah

**Files:**
- Create: `plugins/skill_khutbah.py`

**Step 1: Create Python plugin**

```python
# plugins/skill_khutbah.py
import json
import urllib.request
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="khutbah",
    description="Get latest weekly Friday khutbah (sermon) from JAKIM Malaysia. Search khutbah archive.",
    parameters={
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["latest", "search"],
                "description": "latest = this week's khutbah, search = search archive by keyword"
            },
            "query": {"type": "string", "description": "Search keyword for khutbah archive"},
        },
        "required": [],
    },
)
def execute(input: SkillInput) -> SkillResult:
    args = input.parse_args()
    action = args.get("action", "latest")

    if action == "latest":
        # JAKIM publishes weekly khutbah - scrape or use API
        url = "https://www.islam.gov.my/api/khutbah/latest"
        try:
            req = urllib.request.Request(url)
            req.add_header("User-Agent", "AmanClaw/1.0")
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = json.loads(resp.read().decode())
            title = data.get("title", "N/A")
            date = data.get("date", "N/A")
            content = data.get("content", data.get("summary", "Tidak dapat dimuatkan."))
            # Truncate if too long
            if len(content) > 1500:
                content = content[:1500] + "...\n\n[Baca penuh di portal JAKIM]"
            return SkillResult.ok(
                f"Khutbah Jumaat Minggu Ini:\n\n"
                f"Tajuk: {title}\n"
                f"Tarikh: {date}\n\n"
                f"{content}"
            )
        except Exception as e:
            return SkillResult.ok(
                "Maaf, tidak dapat memuat khutbah terkini dari JAKIM.\n"
                "Sila layari: https://www.islam.gov.my/e-jakim/teks-khutbah-jumaat"
            )

    if action == "search":
        query = args.get("query", "")
        if not query:
            return SkillResult.err("Sila berikan kata kunci carian.")
        return SkillResult.ok(
            f"Carian khutbah untuk '{query}':\n"
            f"Sila layari: https://www.islam.gov.my/e-jakim/teks-khutbah-jumaat\n"
            f"dan cari menggunakan kata kunci tersebut."
        )

    return SkillResult.err(f"Action tidak dikenali: {action}")

if __name__ == "__main__":
    execute.run()
```

**Step 2: Add to config.yaml, commit**

```bash
git commit -m "feat: add skill-khutbah Python plugin for weekly JAKIM khutbah"
```

---

## Task 11: skill-jakim (Python) — JAKIM Services & Fatwa

**Files:**
- Create: `plugins/skill_jakim.py`

**Step 1: Create Python plugin**

```python
# plugins/skill_jakim.py
import json
import urllib.request
import urllib.parse
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="jakim",
    description="Access JAKIM Malaysia services: fatwa search, Islamic events calendar, e-JAKIM services directory.",
    parameters={
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["fatwa", "events", "services"],
                "description": "fatwa = search fatwa database, events = Islamic events calendar, services = JAKIM services directory"
            },
            "query": {"type": "string", "description": "Search query for fatwa"},
        },
        "required": [],
    },
)
def execute(input: SkillInput) -> SkillResult:
    args = input.parse_args()
    action = args.get("action", "services")

    if action == "services":
        return SkillResult.ok(
            "Perkhidmatan e-JAKIM:\n\n"
            "1. e-Solat - Waktu solat seluruh Malaysia\n"
            "   https://www.e-solat.gov.my\n\n"
            "2. Portal Halal - Semakan status halal\n"
            "   https://www.halal.gov.my\n\n"
            "3. e-Fatwa - Pangkalan data fatwa Malaysia\n"
            "   https://e-muamalat.islam.gov.my\n\n"
            "4. Teks Khutbah - Khutbah Jumaat mingguan\n"
            "   https://www.islam.gov.my/e-jakim/teks-khutbah-jumaat\n\n"
            "5. e-Quran - Al-Quran digital JAKIM\n"
            "   https://quran.jakim.gov.my\n\n"
            "6. SPMJ - Sistem Pengurusan Masjid\n"
            "   https://spmj.jawhar.gov.my"
        )

    if action == "fatwa":
        query = args.get("query", "")
        if not query:
            return SkillResult.err("Sila berikan kata kunci untuk carian fatwa.")
        try:
            url = f"https://e-muamalat.islam.gov.my/api/fatwa/search?q={urllib.parse.quote(query)}"
            req = urllib.request.Request(url)
            req.add_header("User-Agent", "AmanClaw/1.0")
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = json.loads(resp.read().decode())
            results = data.get("results", data.get("data", []))
            if not results:
                return SkillResult.ok(
                    f"Tiada fatwa ditemui untuk '{query}'.\n"
                    f"Cuba cari di: https://e-muamalat.islam.gov.my"
                )
            lines = []
            for r in results[:3]:
                title = r.get("title", "N/A")
                status = r.get("status", "N/A")
                date = r.get("date", "N/A")
                lines.append(f"- {title}\n  Status: {status} | Tarikh: {date}")
            return SkillResult.ok(f"Fatwa berkaitan '{query}':\n\n" + "\n\n".join(lines))
        except Exception:
            return SkillResult.ok(
                f"Tidak dapat mencari fatwa secara automatik.\n"
                f"Sila layari: https://e-muamalat.islam.gov.my dan cari '{query}'."
            )

    if action == "events":
        return SkillResult.ok(
            "Peristiwa Islam Utama 2026:\n\n"
            "- Awal Muharram (Tahun Baru Islam)\n"
            "- Mawlidur Rasul (Hari Keputeraan Nabi)\n"
            "- Israk & Mikraj\n"
            "- Nisfu Sya'ban\n"
            "- Ramadan\n"
            "- Nuzul Al-Quran\n"
            "- Hari Raya Aidilfitri\n"
            "- Hari Raya Haji\n\n"
            "Tarikh tepat bergantung pada rukyah. Semak di: https://www.islam.gov.my"
        )

    return SkillResult.err(f"Action tidak dikenali: {action}")

if __name__ == "__main__":
    execute.run()
```

**Step 2: Add to config.yaml, commit**

```bash
git commit -m "feat: add skill-jakim Python plugin for JAKIM services and fatwa search"
```

---

## Task 12: Community Model — Database Schema & CRUD

**Files:**
- Modify: `rust/crates/amanclaw-memory/src/schema.rs` (add community tables)
- Create: `rust/crates/amanclaw-memory/src/community.rs` (community CRUD)
- Modify: `rust/crates/amanclaw-memory/src/lib.rs` (export community module)

**Step 1: Add community tables to schema.rs**

Add these SQL statements to the schema initialization:

```sql
CREATE TABLE IF NOT EXISTS communities (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    zone TEXT NOT NULL DEFAULT 'WLY01',
    language TEXT NOT NULL DEFAULT 'rojak',
    platform TEXT NOT NULL,
    platform_group_id TEXT NOT NULL UNIQUE,
    enabled_skills TEXT NOT NULL DEFAULT '[]',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS community_notifications (
    community_id TEXT NOT NULL REFERENCES communities(id),
    notification_type TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (community_id, notification_type)
);

CREATE TABLE IF NOT EXISTS community_admins (
    community_id TEXT NOT NULL REFERENCES communities(id),
    user_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    added_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (community_id, user_id)
);
```

**Step 2: Create community.rs with CRUD operations**

```rust
// rust/crates/amanclaw-memory/src/community.rs
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    pub id: String,
    pub name: String,
    pub zone: String,
    pub language: String,
    pub platform: String,
    pub platform_group_id: String,
    pub enabled_skills: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityNotifications {
    pub solat_reminder: bool,
    pub daily_doa: bool,
    pub daily_quran: bool,
    pub weekly_khutbah: bool,
}

impl Default for CommunityNotifications {
    fn default() -> Self {
        Self {
            solat_reminder: true,
            daily_doa: true,
            daily_quran: true,
            weekly_khutbah: true,
        }
    }
}

pub struct CommunityStore {
    pool: SqlitePool,
}

impl CommunityStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, name: &str, zone: &str, language: &str, platform: &str, group_id: &str) -> Result<Community, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let skills = serde_json::to_string(&vec![
            "solat", "quran", "qiblat", "hijri", "doa",
            "hadith", "halal", "zakat", "masjid", "khutbah", "jakim"
        ]).unwrap();

        sqlx::query("INSERT INTO communities (id, name, zone, language, platform, platform_group_id, enabled_skills) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(&id).bind(name).bind(zone).bind(language).bind(platform).bind(group_id).bind(&skills)
            .execute(&self.pool).await?;

        // Set default notifications
        for ntype in &["solat_reminder", "daily_doa", "daily_quran", "weekly_khutbah"] {
            sqlx::query("INSERT INTO community_notifications (community_id, notification_type, enabled) VALUES (?, ?, 1)")
                .bind(&id).bind(ntype)
                .execute(&self.pool).await?;
        }

        self.get_by_id(&id).await
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Community, sqlx::Error> {
        let row = sqlx::query_as::<_, (String, String, String, String, String, String, String, String)>(
            "SELECT id, name, zone, language, platform, platform_group_id, enabled_skills, created_at FROM communities WHERE id = ?"
        ).bind(id).fetch_one(&self.pool).await?;

        Ok(Community {
            id: row.0, name: row.1, zone: row.2, language: row.3,
            platform: row.4, platform_group_id: row.5,
            enabled_skills: serde_json::from_str(&row.6).unwrap_or_default(),
            created_at: row.7,
        })
    }

    pub async fn get_by_group_id(&self, platform: &str, group_id: &str) -> Result<Option<Community>, sqlx::Error> {
        let row = sqlx::query_as::<_, (String, String, String, String, String, String, String, String)>(
            "SELECT id, name, zone, language, platform, platform_group_id, enabled_skills, created_at FROM communities WHERE platform = ? AND platform_group_id = ?"
        ).bind(platform).bind(group_id).fetch_optional(&self.pool).await?;

        Ok(row.map(|r| Community {
            id: r.0, name: r.1, zone: r.2, language: r.3,
            platform: r.4, platform_group_id: r.5,
            enabled_skills: serde_json::from_str(&r.6).unwrap_or_default(),
            created_at: r.7,
        }))
    }

    pub async fn update_zone(&self, id: &str, zone: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE communities SET zone = ? WHERE id = ?")
            .bind(zone).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_language(&self, id: &str, language: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE communities SET language = ? WHERE id = ?")
            .bind(language).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_skills(&self, id: &str, skills: &[String]) -> Result<(), sqlx::Error> {
        let json = serde_json::to_string(skills).unwrap();
        sqlx::query("UPDATE communities SET enabled_skills = ? WHERE id = ?")
            .bind(&json).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn add_admin(&self, community_id: &str, user_id: &str, platform: &str) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT OR IGNORE INTO community_admins (community_id, user_id, platform) VALUES (?, ?, ?)")
            .bind(community_id).bind(user_id).bind(platform).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn is_admin(&self, community_id: &str, user_id: &str) -> Result<bool, sqlx::Error> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM community_admins WHERE community_id = ? AND user_id = ?")
            .bind(community_id).bind(user_id).fetch_one(&self.pool).await?;
        Ok(count.0 > 0)
    }

    pub async fn get_all_communities(&self) -> Result<Vec<Community>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, String, String, String)>(
            "SELECT id, name, zone, language, platform, platform_group_id, enabled_skills, created_at FROM communities"
        ).fetch_all(&self.pool).await?;

        Ok(rows.into_iter().map(|r| Community {
            id: r.0, name: r.1, zone: r.2, language: r.3,
            platform: r.4, platform_group_id: r.5,
            enabled_skills: serde_json::from_str(&r.6).unwrap_or_default(),
            created_at: r.7,
        }).collect())
    }
}
```

**Step 3: Add uuid dependency to amanclaw-memory Cargo.toml**

```toml
uuid = { version = "1", features = ["v4"] }
```

**Step 4: Export module, build, commit**

```bash
git commit -m "feat: add community model with SQLite schema and CRUD operations"
```

---

## Task 13: Update config.yaml with all Python plugins

**Files:**
- Modify: `config.example.yaml`

**Step 1: Add all script_plugins entries**

```yaml
script_plugins:
  hadith:
    command: "python3"
    args: ["./plugins/skill_hadith.py"]
    env:
      SUNNAH_API_KEY: "${SUNNAH_API_KEY}"
  halal:
    command: "python3"
    args: ["./plugins/skill_halal.py"]
  zakat:
    command: "python3"
    args: ["./plugins/skill_zakat.py"]
  masjid:
    command: "python3"
    args: ["./plugins/skill_masjid.py"]
    env:
      GOOGLE_PLACES_API_KEY: "${GOOGLE_PLACES_API_KEY}"
  khutbah:
    command: "python3"
    args: ["./plugins/skill_khutbah.py"]
  jakim:
    command: "python3"
    args: ["./plugins/skill_jakim.py"]
```

**Step 2: Update .env.example with new API keys**

```bash
# Islamic Skills API Keys
SUNNAH_API_KEY=your_sunnah_api_key
GOOGLE_PLACES_API_KEY=your_google_places_api_key
```

**Step 3: Commit**

```bash
git commit -m "feat: add Islamic skills configuration to config.example.yaml"
```

---

## Task 14: Integration Test — Verify All Skills Register

**Files:**
- Create: `rust/tests/islamic_skills_integration.rs`

**Step 1: Write integration test**

```rust
// rust/tests/islamic_skills_integration.rs
use amanclaw_traits::skill::Skill;

#[test]
fn test_solat_metadata() {
    let skill = amanclaw_skill_solat::SolatSkill;
    let meta = skill.metadata();
    assert_eq!(meta.name, "solat");
    assert!(meta.description.contains("prayer") || meta.description.contains("solat"));
}

#[test]
fn test_qiblat_metadata() {
    let skill = amanclaw_skill_qiblat::QiblatSkill;
    let meta = skill.metadata();
    assert_eq!(meta.name, "qiblat");
}

#[test]
fn test_hijri_metadata() {
    let skill = amanclaw_skill_hijri::HijriSkill;
    let meta = skill.metadata();
    assert_eq!(meta.name, "hijri");
}

#[test]
fn test_doa_metadata() {
    let skill = amanclaw_skill_doa::DoaSkill;
    let meta = skill.metadata();
    assert_eq!(meta.name, "doa");
}

#[test]
fn test_quran_metadata() {
    let skill = amanclaw_skill_quran::QuranSkill;
    let meta = skill.metadata();
    assert_eq!(meta.name, "quran");
}

#[tokio::test]
async fn test_qiblat_from_kl() {
    let skill = amanclaw_skill_qiblat::QiblatSkill;
    let input = amanclaw_traits::skill::SkillInput {
        name: "qiblat".into(),
        args: r#"{"latitude": 3.139, "longitude": 101.6869}"#.into(),
        user_id: "test".into(),
        platform: "test".into(),
    };
    let result = skill.execute(input).await;
    assert!(result.success);
    assert!(result.output.contains("292") || result.output.contains("293")); // KL qiblat ~292-293 degrees
}

#[tokio::test]
async fn test_hijri_today() {
    let skill = amanclaw_skill_hijri::HijriSkill;
    let input = amanclaw_traits::skill::SkillInput {
        name: "hijri".into(),
        args: r#"{"action": "today"}"#.into(),
        user_id: "test".into(),
        platform: "test".into(),
    };
    let result = skill.execute(input).await;
    assert!(result.success);
    assert!(result.output.contains("Hijri"));
}
```

**Step 2: Run tests**

Run: `cd rust && cargo test --test islamic_skills_integration`
Expected: All tests PASS.

**Step 3: Commit**

```bash
git commit -m "test: add integration tests for Islamic skills"
```

---

## Task 15: Full Build & Smoke Test

**Step 1: Build entire workspace**

Run: `cd rust && cargo build`
Expected: Compiles successfully with all 5 new Rust skills.

**Step 2: Verify Python plugins load**

Run: `python3 plugins/skill_hadith.py <<< '{"method": "metadata"}'`
Expected: JSON output with name "hadith".

Run: `python3 plugins/skill_zakat.py <<< '{"method": "metadata"}'`
Expected: JSON output with name "zakat".

**Step 3: Run full test suite**

Run: `cd rust && cargo test`
Expected: All tests pass.

**Step 4: Final commit**

```bash
git commit -m "chore: verify full build with all 11 Islamic skills"
```

---

## Summary

| Task | Type | Skill | Est. Complexity |
|------|------|-------|----------------|
| 1 | Rust | skill-solat | Medium (API + zones) |
| 2 | Rust | skill-qiblat | Low (pure math) |
| 3 | Rust | skill-hijri | Medium (calendar algorithm) |
| 4 | Rust | skill-doa | Low (static data) |
| 5 | Rust | skill-quran | Medium (API integration) |
| 6 | Python | skill-hadith | Low (API wrapper) |
| 7 | Python | skill-halal | Medium (JAKIM scraping) |
| 8 | Python | skill-zakat | Low (calculator) |
| 9 | Python | skill-masjid | Medium (Google Places) |
| 10 | Python | skill-khutbah | Low (API/scrape) |
| 11 | Python | skill-jakim | Low (directory + API) |
| 12 | Rust | Community model | Medium (schema + CRUD) |
| 13 | Config | config.yaml | Low |
| 14 | Test | Integration tests | Low |
| 15 | Build | Full verification | Low |

**Total: 15 tasks covering Phase 1 of the Islamic Community Platform.**
