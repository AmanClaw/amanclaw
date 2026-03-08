pub mod converter;
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
                        upcoming.push(format!(
                            "{} {} - {} ({})",
                            e.day,
                            converter::month_name_ms(e.month),
                            e.name_ms,
                            e.name_en
                        ));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let skill = HijriSkill;
        let meta = skill.metadata();
        assert_eq!(meta.name, "hijri");
        assert_eq!(meta.version, "0.1.0");
    }

    #[tokio::test]
    async fn test_today_action() {
        let skill = HijriSkill;
        let input = SkillInput {
            name: "hijri".into(),
            args: r#"{"action": "today"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("Tarikh Hijri Hari Ini"));
    }

    #[tokio::test]
    async fn test_convert_action() {
        let skill = HijriSkill;
        let input = SkillInput {
            name: "hijri".into(),
            args: r#"{"action": "convert", "date": "2024-01-01"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("1445"), "Output should contain Hijri year: {}", result.output);
    }

    #[tokio::test]
    async fn test_convert_invalid_date() {
        let skill = HijriSkill;
        let input = SkillInput {
            name: "hijri".into(),
            args: r#"{"action": "convert", "date": "not-a-date"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_events_action() {
        let skill = HijriSkill;
        let input = SkillInput {
            name: "hijri".into(),
            args: r#"{"action": "events"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        // Should either list events or say none upcoming
        assert!(!result.output.is_empty());
    }

    #[tokio::test]
    async fn test_default_action() {
        let skill = HijriSkill;
        let input = SkillInput {
            name: "hijri".into(),
            args: "{}".into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("Tarikh Hijri Hari Ini"));
    }

    #[tokio::test]
    async fn test_unknown_action() {
        let skill = HijriSkill;
        let input = SkillInput {
            name: "hijri".into(),
            args: r#"{"action": "xyz"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Unknown action"));
    }
}
