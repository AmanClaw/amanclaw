mod jakim;
mod zones;

use amanclaw_prayer_times::{CalculationMethod, PrayerTimes, calculate};
use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};
use chrono::NaiveDate;

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
                    "description": "JAKIM zone code e.g. SGR01, WLY01, JHR02. Required for 'today' action."
                },
                "action": {
                    "type": "string",
                    "enum": ["today", "list_zones", "list_states", "calculate", "list_methods"],
                    "description": "Action to perform. Default: today"
                },
                "latitude": {
                    "type": "number",
                    "description": "Latitude for 'calculate' action (positive = North)"
                },
                "longitude": {
                    "type": "number",
                    "description": "Longitude for 'calculate' action (positive = East)"
                },
                "timezone": {
                    "type": "number",
                    "description": "UTC offset in hours for 'calculate' action (e.g. 8 for Malaysia, -5 for US East)"
                },
                "method": {
                    "type": "string",
                    "description": "Calculation method for 'calculate' action: MWL, ISNA, Egyptian, Karachi, UmmAlQura, JAKIM"
                },
                "date": {
                    "type": "string",
                    "description": "Date in YYYY-MM-DD format for 'calculate' action. Defaults to today."
                }
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

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("today");

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
                let all_zones = if state.is_empty() {
                    zones::get_all_zones()
                } else {
                    zones::zones_by_state(state)
                };
                let list: Vec<String> = all_zones
                    .iter()
                    .map(|z| format!("{}: {} ({})", z.code, z.state, z.areas))
                    .collect();
                SkillResult {
                    success: true,
                    output: if list.is_empty() {
                        format!(
                            "No zones found for '{state}'. Use action=list_states to see available states."
                        )
                    } else {
                        list.join("\n")
                    },
                    error: None,
                }
            }
            "list_methods" => {
                let methods: Vec<String> = CalculationMethod::all()
                    .iter()
                    .map(|m| format!("{m} — {}", m.display_name()))
                    .collect();
                SkillResult {
                    success: true,
                    output: format!(
                        "Available prayer time calculation methods:\n{}",
                        methods.join("\n")
                    ),
                    error: None,
                }
            }
            "calculate" => {
                let lat = match args.get("latitude").and_then(|v| v.as_f64()) {
                    Some(v) => v,
                    None => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some(
                                "latitude is required for calculate action".into(),
                            ),
                        };
                    }
                };
                let lon = match args.get("longitude").and_then(|v| v.as_f64()) {
                    Some(v) => v,
                    None => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some(
                                "longitude is required for calculate action".into(),
                            ),
                        };
                    }
                };
                let tz = args
                    .get("timezone")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                let method_str = args
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("MWL");
                let method = match CalculationMethod::from_str_loose(method_str) {
                    Some(m) => m,
                    None => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some(format!(
                                "Unknown method '{method_str}'. Use action=list_methods to see available methods."
                            )),
                        };
                    }
                };

                let date = if let Some(ds) = args.get("date").and_then(|v| v.as_str()) {
                    match NaiveDate::parse_from_str(ds, "%Y-%m-%d") {
                        Ok(d) => d,
                        Err(e) => {
                            return SkillResult {
                                success: false,
                                output: String::new(),
                                error: Some(format!(
                                    "Invalid date '{ds}': {e}. Use YYYY-MM-DD format."
                                )),
                            };
                        }
                    }
                } else {
                    chrono::Local::now().date_naive()
                };

                let times = calculate(date, lat, lon, tz, method);
                let output = format!(
                    "Prayer Times ({}, {}):\nDate: {}\nCoordinates: {:.4}, {:.4} (UTC{:+.1})\n\nFajr:    {}\nSunrise: {}\nDhuhr:   {}\nAsr:     {}\nMaghrib: {}\nIsha:    {}",
                    method,
                    method.display_name(),
                    date,
                    lat,
                    lon,
                    tz,
                    PrayerTimes::format_time(times.fajr),
                    PrayerTimes::format_time(times.sunrise),
                    PrayerTimes::format_time(times.dhuhr),
                    PrayerTimes::format_time(times.asr),
                    PrayerTimes::format_time(times.maghrib),
                    PrayerTimes::format_time(times.isha),
                );
                SkillResult {
                    success: true,
                    output,
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
                        error: Some(format!(
                            "Unknown zone '{zone}'. Use action=list_zones to see valid zones."
                        )),
                    };
                }

                match jakim::fetch_prayer_times(zone).await {
                    Ok(times) => {
                        let t = &times[0];
                        let output = format!(
                            "Waktu Solat {} ({}):\n\nImsak: {}\nSubuh: {}\nSyuruk: {}\nZohor: {}\nAsar: {}\nMaghrib: {}\nIsyak: {}\n\nTarikh: {} | Hijri: {}",
                            zone,
                            t.day,
                            t.imsak,
                            t.fajr,
                            t.syuruk,
                            t.dhuhr,
                            t.asr,
                            t.maghrib,
                            t.isha,
                            t.date,
                            t.hijri
                        );
                        SkillResult {
                            success: true,
                            output,
                            error: None,
                        }
                    }
                    Err(e) => SkillResult {
                        success: false,
                        output: String::new(),
                        error: Some(e),
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let skill = SolatSkill;
        let meta = skill.metadata();
        assert_eq!(meta.name, "solat");
        assert_eq!(meta.version, "0.1.0");
    }

    #[test]
    fn test_parameters_schema() {
        let skill = SolatSkill;
        let schema = skill.parameters_schema();
        assert!(schema["properties"]["zone"].is_object());
        assert!(schema["properties"]["action"].is_object());
    }

    #[test]
    fn test_zones_find() {
        assert!(zones::find_zone("WLY01").is_some());
        assert!(zones::find_zone("SGR01").is_some());
        assert!(zones::find_zone("INVALID").is_none());
    }

    #[test]
    fn test_zones_by_state() {
        let johor = zones::zones_by_state("Johor");
        assert_eq!(johor.len(), 4);
    }

    #[test]
    fn test_get_states() {
        let states = zones::get_states();
        assert_eq!(states.len(), 15);
        assert!(states.contains(&"Selangor"));
    }

    #[tokio::test]
    async fn test_missing_zone() {
        let skill = SolatSkill;
        let input = SkillInput {
            name: "solat".into(),
            args: r#"{"action": "today"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Zone required"));
    }

    #[tokio::test]
    async fn test_invalid_zone() {
        let skill = SolatSkill;
        let input = SkillInput {
            name: "solat".into(),
            args: r#"{"zone": "INVALID"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Unknown zone"));
    }

    #[tokio::test]
    async fn test_list_states() {
        let skill = SolatSkill;
        let input = SkillInput {
            name: "solat".into(),
            args: r#"{"action": "list_states"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("Selangor"));
    }

    #[tokio::test]
    async fn test_list_zones() {
        let skill = SolatSkill;
        let input = SkillInput {
            name: "solat".into(),
            args: r#"{"action": "list_zones", "zone": "Johor"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("JHR01"));
    }
}
