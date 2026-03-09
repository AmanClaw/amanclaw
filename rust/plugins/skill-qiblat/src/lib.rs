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
    let y =
        lat_rad.cos() * kaaba_lat_rad.sin() - lat_rad.sin() * kaaba_lat_rad.cos() * delta_lon.cos();

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
            description:
                "Calculate Qiblat direction (arah kiblat) from any location to Kaaba in Makkah."
                    .into(),
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
            Err(e) => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Invalid args: {e}")),
                };
            }
        };

        // Default to KL if no coordinates given
        let lat = args
            .get("latitude")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.1390);
        let lon = args
            .get("longitude")
            .and_then(|v| v.as_f64())
            .unwrap_or(101.6869);
        let location = args
            .get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("Kuala Lumpur");

        let (bearing, compass) = calculate_qiblat(lat, lon);
        let distance = calculate_distance_km(lat, lon);

        let output = format!(
            "Arah Kiblat dari {location}:\n\nBearing: {bearing:.1}\u{00b0}\nArah: {compass}\nJarak ke Kaabah: {distance:.0} km\n\nKoordinat: {lat:.4}\u{00b0}N, {lon:.4}\u{00b0}E"
        );

        SkillResult {
            success: true,
            output,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let skill = QiblatSkill;
        let meta = skill.metadata();
        assert_eq!(meta.name, "qiblat");
        assert_eq!(meta.timeout_ms, 5000);
    }

    #[test]
    fn test_qiblat_from_kl() {
        let (bearing, compass) = calculate_qiblat(3.1390, 101.6869);
        // From KL, Qiblat should be roughly W (~292 degrees)
        assert!(
            bearing > 270.0 && bearing < 300.0,
            "Bearing from KL should be ~292, got {bearing}"
        );
        assert_eq!(compass, "Barat (W)");
    }

    #[test]
    fn test_qiblat_from_london() {
        let (bearing, _compass) = calculate_qiblat(51.5074, -0.1278);
        // From London, Qiblat should be roughly SE (~119 degrees)
        assert!(
            bearing > 100.0 && bearing < 140.0,
            "Bearing from London should be ~119, got {bearing}"
        );
    }

    #[test]
    fn test_distance_from_kl() {
        let dist = calculate_distance_km(3.1390, 101.6869);
        // KL to Makkah is roughly 6900-7100 km
        assert!(
            dist > 6800.0 && dist < 7200.0,
            "Distance from KL should be ~6974km, got {dist}"
        );
    }

    #[test]
    fn test_distance_from_makkah() {
        let dist = calculate_distance_km(KAABA_LAT, KAABA_LON);
        // Distance from Kaaba to itself should be ~0
        assert!(
            dist < 1.0,
            "Distance from Kaaba to itself should be ~0, got {dist}"
        );
    }

    #[tokio::test]
    async fn test_execute_default() {
        let skill = QiblatSkill;
        let input = SkillInput {
            name: "qiblat".into(),
            args: "{}".into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("Kuala Lumpur"));
        assert!(result.output.contains("Bearing"));
    }

    #[tokio::test]
    async fn test_execute_invalid_args() {
        let skill = QiblatSkill;
        let input = SkillInput {
            name: "qiblat".into(),
            args: "not json".into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
