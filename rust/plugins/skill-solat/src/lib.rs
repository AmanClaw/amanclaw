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
            Err(e) => return SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("Invalid args: {}", e)),
            },
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
