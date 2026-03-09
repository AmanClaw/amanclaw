use amanclaw_traits::config::AppConfig;

/// The number of built-in Rust skills compiled into the engine.
const BUILTIN_SKILL_COUNT: usize = 7; // sysinfo, shell, solat, qiblat, hijri, doa, quran

#[derive(Debug, Clone)]
pub struct DiagnosticResult {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

/// Run startup diagnostics against the given config and environment.
pub fn run_startup_diagnostics(config: &AppConfig) -> Vec<DiagnosticResult> {
    let mut results = Vec::new();

    // 1. Config loaded (always passes)
    results.push(DiagnosticResult {
        name: "Config loaded".into(),
        passed: true,
        detail: format!("LLM model: {}", config.llm.model),
    });

    // 2. LLM configured
    let llm_ok = !config.llm.base_url.is_empty();
    results.push(DiagnosticResult {
        name: "LLM configured".into(),
        passed: llm_ok,
        detail: if llm_ok {
            format!("{} ({})", config.llm.base_url, config.llm.model)
        } else {
            "base_url is empty".into()
        },
    });

    // 3. Telegram
    let telegram_ok = std::env::var("TELEGRAM_BOT_TOKEN").is_ok();
    results.push(DiagnosticResult {
        name: "Telegram".into(),
        passed: telegram_ok,
        detail: if telegram_ok {
            "TELEGRAM_BOT_TOKEN set".into()
        } else {
            "TELEGRAM_BOT_TOKEN not set".into()
        },
    });

    // 4. Discord
    let discord_ok = std::env::var("DISCORD_BOT_TOKEN").is_ok();
    results.push(DiagnosticResult {
        name: "Discord".into(),
        passed: discord_ok,
        detail: if discord_ok {
            "DISCORD_BOT_TOKEN set".into()
        } else {
            "DISCORD_BOT_TOKEN not set".into()
        },
    });

    // 5. WhatsApp
    let whatsapp_ok = std::env::var("WHATSAPP_TOKEN").is_ok()
        || std::env::var("WHATSAPP_PHONE_NUMBER_ID").is_ok();
    results.push(DiagnosticResult {
        name: "WhatsApp".into(),
        passed: whatsapp_ok,
        detail: if whatsapp_ok {
            "WhatsApp credentials set".into()
        } else {
            "WHATSAPP_TOKEN / WHATSAPP_PHONE_NUMBER_ID not set".into()
        },
    });

    // 6. Slack
    let slack_ok = std::env::var("SLACK_BOT_TOKEN").is_ok();
    results.push(DiagnosticResult {
        name: "Slack".into(),
        passed: slack_ok,
        detail: if slack_ok {
            "SLACK_BOT_TOKEN set".into()
        } else {
            "SLACK_BOT_TOKEN not set".into()
        },
    });

    // 7. Skills
    let disabled_count = config.skills.disabled.len();
    let active_count = BUILTIN_SKILL_COUNT.saturating_sub(disabled_count);
    results.push(DiagnosticResult {
        name: "Skills".into(),
        passed: active_count > 0,
        detail: format!("{active_count} built-in active, {disabled_count} disabled"),
    });

    // 8. Script plugins
    let script_count = config.script_plugins.len();
    results.push(DiagnosticResult {
        name: "Script plugins".into(),
        passed: true,
        detail: format!("{script_count} configured"),
    });

    results
}

/// Print diagnostics to stdout with checkmarks / crosses.
pub fn print_diagnostics(results: &[DiagnosticResult]) {
    println!();
    println!("=== AmanClaw Startup Diagnostics ===");
    println!();
    for r in results {
        let icon = if r.passed { "\u{2713}" } else { "\u{2717}" };
        println!("  [{}] {}: {}", icon, r.name, r.detail);
    }
    let passed = results.iter().filter(|r| r.passed).count();
    println!();
    println!("  {}/{} checks passed", passed, results.len());
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use amanclaw_traits::config::AppConfig;

    fn minimal_config() -> AppConfig {
        let yaml = r#"
llm:
  base_url: "http://localhost:8001/v1"
  model: "test-model"
"#;
        serde_yaml::from_str(yaml).unwrap()
    }

    #[test]
    fn test_diagnostics_always_includes_config_loaded() {
        let config = minimal_config();
        let results = run_startup_diagnostics(&config);
        let config_check = results.iter().find(|r| r.name == "Config loaded").unwrap();
        assert!(config_check.passed);
    }

    #[test]
    fn test_diagnostics_llm_configured() {
        let config = minimal_config();
        let results = run_startup_diagnostics(&config);
        let llm_check = results.iter().find(|r| r.name == "LLM configured").unwrap();
        assert!(llm_check.passed);
        assert!(llm_check.detail.contains("localhost:8001"));
    }

    #[test]
    fn test_diagnostics_llm_empty_base_url() {
        let yaml = r#"
llm:
  base_url: ""
  model: "test-model"
"#;
        let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
        let results = run_startup_diagnostics(&config);
        let llm_check = results.iter().find(|r| r.name == "LLM configured").unwrap();
        assert!(!llm_check.passed);
    }

    #[test]
    fn test_diagnostics_skills_count() {
        let config = minimal_config();
        let results = run_startup_diagnostics(&config);
        let skills_check = results.iter().find(|r| r.name == "Skills").unwrap();
        assert!(skills_check.passed);
        assert!(skills_check.detail.contains("7 built-in active"));
        assert!(skills_check.detail.contains("0 disabled"));
    }

    #[test]
    fn test_diagnostics_skills_with_disabled() {
        let yaml = r#"
llm:
  base_url: "http://localhost:8001/v1"
  model: "test-model"
skills:
  disabled:
    - shell
    - sysinfo
"#;
        let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
        let results = run_startup_diagnostics(&config);
        let skills_check = results.iter().find(|r| r.name == "Skills").unwrap();
        assert!(skills_check.passed);
        assert!(skills_check.detail.contains("5 built-in active"));
        assert!(skills_check.detail.contains("2 disabled"));
    }

    #[test]
    fn test_diagnostics_script_plugins() {
        let config = minimal_config();
        let results = run_startup_diagnostics(&config);
        let scripts = results.iter().find(|r| r.name == "Script plugins").unwrap();
        assert!(scripts.passed);
        assert!(scripts.detail.contains("0 configured"));
    }

    #[test]
    fn test_diagnostics_result_count() {
        let config = minimal_config();
        let results = run_startup_diagnostics(&config);
        assert_eq!(results.len(), 8);
    }

    #[test]
    fn test_print_diagnostics_does_not_panic() {
        let config = minimal_config();
        let results = run_startup_diagnostics(&config);
        // Just verify it doesn't panic
        print_diagnostics(&results);
    }
}
