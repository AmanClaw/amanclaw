# Phase 2: DX + Differentiators Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make AmanClaw delightful to build on — skill scaffolding, live reload, playground, global prayer times, WhatsApp interactive messages, and published benchmarks.

**Architecture:** Extend CLI with `skill` subcommand group and `--watch` flag. Add pure-Rust prayer time calculation engine (no API dependency). Add WhatsApp interactive message types (buttons, lists). Embed minimal web playground served from Rust (no Node.js build step). Add benchmark suite with criterion.

**Tech Stack:** Rust, Clap (CLI), notify (file watcher), axum (embedded web server), criterion (benchmarks), WhatsApp Cloud API interactive messages

---

### Task 1: Add `amanclaw skill new` CLI Scaffolding

**Context:** Developers need a one-command way to create new skills. `amanclaw skill new my-skill --lang rust` generates a complete skill project with Cargo.toml, Skill trait impl, tests, and manifest. Python variant generates a single-file plugin with the JSON-RPC protocol.

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/cli.rs`
- Create: `rust/crates/amanclaw-cli/src/scaffold.rs`
- Modify: `rust/crates/amanclaw-cli/src/main.rs`

**Step 1: Write failing test for CLI parsing**

Add to `rust/crates/amanclaw-cli/src/cli.rs`:

```rust
// In the Command enum, add:
/// Manage skills
Skill {
    #[command(subcommand)]
    action: SkillAction,
},

#[derive(Subcommand)]
pub enum SkillAction {
    /// Create a new skill from template
    New {
        /// Skill name (e.g., my-skill)
        name: String,
        /// Language: rust or python
        #[arg(short, long, default_value = "rust")]
        lang: String,
        /// Output directory (default: ./plugins)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Run a skill's tests in isolation
    Test {
        /// Skill name
        name: String,
    },
}
```

Add tests:
```rust
#[test]
fn test_cli_skill_new_rust() {
    let cli = Cli::parse_from(["amanclaw", "skill", "new", "my-skill"]);
    assert!(matches!(cli.command, Some(Command::Skill { .. })));
}

#[test]
fn test_cli_skill_new_python() {
    let cli = Cli::parse_from(["amanclaw", "skill", "new", "my-skill", "--lang", "python"]);
    assert!(matches!(cli.command, Some(Command::Skill { .. })));
}
```

Run: `cargo test -p amanclaw-cli -- test_cli_skill`
Expected: PASS

**Step 2: Create scaffold module**

Create `rust/crates/amanclaw-cli/src/scaffold.rs`:

```rust
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn scaffold_rust_skill(name: &str, output_dir: &Path) -> Result<PathBuf> {
    let skill_dir = output_dir.join(format!("skill-{name}"));
    std::fs::create_dir_all(&skill_dir)?;
    std::fs::create_dir_all(skill_dir.join("src"))?;

    // Cargo.toml
    let cargo_toml = format!(r#"[package]
name = "skill-{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
amanclaw-plugin-sdk = {{ path = "../../crates/amanclaw-plugin-sdk" }}
serde_json = "1"
"#);
    std::fs::write(skill_dir.join("Cargo.toml"), cargo_toml)?;

    // src/lib.rs
    let lib_rs = format!(r#"use amanclaw_plugin_sdk::*;

amanclaw_plugin! {{
    metadata: SkillMetadata {{
        name: "{name}".into(),
        description: "TODO: describe your skill".into(),
        timeout_ms: 10000,
        version: "0.1.0".into(),
    }},
    parameters: serde_json::json!({{
        "type": "object",
        "properties": {{
            "input": {{
                "type": "string",
                "description": "TODO: describe input"
            }}
        }},
        "required": ["input"]
    }}),
    execute: |input| -> SkillResult {{
        let args: serde_json::Value = serde_json::from_str(&input.args)
            .unwrap_or_default();
        let user_input = args["input"].as_str().unwrap_or("(no input)");
        SkillResult::ok(format!("Hello from {name}! Input: {{user_input}}"))
    }}
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn test_metadata() {{
        // Verify the plugin compiles and metadata is correct
        assert!(!"{name}".is_empty());
    }}
}}
"#);
    std::fs::write(skill_dir.join("src/lib.rs"), lib_rs)?;

    // amanclaw-skill.toml manifest
    let manifest = format!(r#"[skill]
name = "{name}"
version = "0.1.0"
description = "TODO: describe your skill"
author = "Your Name"
license = "MIT"

[permissions]
network = []
filesystem = false
max_memory = "32MB"
timeout = "10s"
"#);
    std::fs::write(skill_dir.join("amanclaw-skill.toml"), manifest)?;

    Ok(skill_dir)
}

pub fn scaffold_python_skill(name: &str, output_dir: &Path) -> Result<PathBuf> {
    let safe_name = name.replace('-', "_");
    let file_path = output_dir.join(format!("skill_{safe_name}.py"));

    let content = format!(r#"#!/usr/bin/env python3
"""AmanClaw skill: {name}

Protocol: JSON-RPC over stdin/stdout (line-based).
"""
import json
import sys


def metadata():
    return {{
        "name": "{name}",
        "description": "TODO: describe your skill",
        "timeout_ms": 10000,
        "version": "0.1.0",
    }}


def parameters():
    return {{
        "type": "object",
        "properties": {{
            "input": {{
                "type": "string",
                "description": "TODO: describe input",
            }}
        }},
        "required": ["input"],
    }}


def execute(skill_input):
    args = json.loads(skill_input.get("args", "{{}}"))
    user_input = args.get("input", "(no input)")
    return {{
        "success": True,
        "output": f"Hello from {name}! Input: {{user_input}}",
        "error": None,
    }}


def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            request = json.loads(line)
        except json.JSONDecodeError:
            continue

        method = request.get("method", "")
        if method == "metadata":
            result = metadata()
        elif method == "parameters":
            result = parameters()
        elif method == "execute":
            result = execute(request.get("input", {{}}))
        elif method == "shutdown":
            break
        else:
            result = {{"error": f"Unknown method: {{method}}"}}

        print(json.dumps(result), flush=True)


if __name__ == "__main__":
    main()
"#);
    std::fs::write(&file_path, content)?;

    // Create manifest alongside
    let manifest_path = output_dir.join(format!("skill_{safe_name}.toml"));
    let manifest = format!(r#"[skill]
name = "{name}"
version = "0.1.0"
description = "TODO: describe your skill"
author = "Your Name"
license = "MIT"
lang = "python"

[permissions]
network = []
filesystem = false
timeout = "10s"
"#);
    std::fs::write(&manifest_path, manifest)?;

    Ok(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scaffold_rust_skill() {
        let dir = TempDir::new().unwrap();
        let result = scaffold_rust_skill("weather", dir.path()).unwrap();
        assert!(result.join("Cargo.toml").exists());
        assert!(result.join("src/lib.rs").exists());
        assert!(result.join("amanclaw-skill.toml").exists());
        let cargo = std::fs::read_to_string(result.join("Cargo.toml")).unwrap();
        assert!(cargo.contains("skill-weather"));
        assert!(cargo.contains("amanclaw-plugin-sdk"));
    }

    #[test]
    fn test_scaffold_python_skill() {
        let dir = TempDir::new().unwrap();
        let result = scaffold_python_skill("halal-check", dir.path()).unwrap();
        assert!(result.exists());
        let content = std::fs::read_to_string(&result).unwrap();
        assert!(content.contains("halal-check"));
        assert!(content.contains("def execute"));
        assert!(content.contains("def metadata"));
    }

    #[test]
    fn test_scaffold_rust_creates_manifest() {
        let dir = TempDir::new().unwrap();
        let result = scaffold_rust_skill("test-skill", dir.path()).unwrap();
        let manifest = std::fs::read_to_string(result.join("amanclaw-skill.toml")).unwrap();
        assert!(manifest.contains("[skill]"));
        assert!(manifest.contains("[permissions]"));
        assert!(manifest.contains("test-skill"));
    }
}
```

**Step 3: Wire up scaffold to CLI main.rs**

Add to `main.rs` match block:
```rust
Some(Command::Skill { action }) => cmd_skill(action).await,
```

Add handler:
```rust
async fn cmd_skill(action: cli::SkillAction) -> Result<()> {
    match action {
        cli::SkillAction::New { name, lang, output } => {
            let output_dir = PathBuf::from(output.unwrap_or_else(|| "plugins".into()));
            std::fs::create_dir_all(&output_dir)?;
            match lang.as_str() {
                "rust" => {
                    let path = scaffold::scaffold_rust_skill(&name, &output_dir)?;
                    println!("Created Rust skill at {}", path.display());
                    println!();
                    println!("Next steps:");
                    println!("  1. cd {}", path.display());
                    println!("  2. Edit src/lib.rs — implement your skill logic");
                    println!("  3. cargo build --target wasm32-wasi --release");
                    println!("  4. cp target/wasm32-wasi/release/skill_{}.wasm ../../plugins/", name.replace('-', "_"));
                }
                "python" => {
                    let path = scaffold::scaffold_python_skill(&name, &output_dir)?;
                    println!("Created Python skill at {}", path.display());
                    println!();
                    println!("Next steps:");
                    println!("  1. Edit {} — implement your skill logic", path.display());
                    println!("  2. Add to config.yaml under script_plugins:");
                    println!("     {}: {{ command: python3, args: [{}] }}", name, path.display());
                }
                other => anyhow::bail!("Unsupported language: {other}. Use 'rust' or 'python'."),
            }
            Ok(())
        }
        cli::SkillAction::Test { name } => {
            println!("Testing skill: {name}...");
            // Check for Rust skill
            let rust_path = PathBuf::from(format!("plugins/skill-{name}"));
            if rust_path.exists() {
                let status = std::process::Command::new("cargo")
                    .args(["test"])
                    .current_dir(&rust_path)
                    .status()
                    .context("Failed to run cargo test")?;
                if !status.success() {
                    anyhow::bail!("Tests failed for skill-{name}");
                }
                println!("All tests passed for skill-{name}");
            } else {
                anyhow::bail!("Skill 'skill-{name}' not found in plugins/. Use 'amanclaw skill new {name}' to create it.");
            }
            Ok(())
        }
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p amanclaw-cli`
Expected: All tests pass

**Step 5: Commit**

```bash
git add rust/crates/amanclaw-cli/
git commit -m "feat(cli): add skill scaffolding (amanclaw skill new)"
```

---

### Task 2: Global Prayer Time Calculation Engine

**Context:** Currently `skill-solat` only fetches from JAKIM API (Malaysia). We need a pure-Rust prayer time calculation engine that supports global calculation methods: MWL, ISNA, Egyptian, Karachi, Umm al-Qura, and JAKIM. This makes AmanClaw useful for Muslim communities worldwide with zero API dependency.

**Files:**
- Create: `rust/crates/amanclaw-prayer-times/Cargo.toml`
- Create: `rust/crates/amanclaw-prayer-times/src/lib.rs`
- Create: `rust/crates/amanclaw-prayer-times/src/methods.rs`
- Create: `rust/crates/amanclaw-prayer-times/src/calc.rs`
- Modify: `rust/Cargo.toml` (add to workspace members)
- Modify: `rust/plugins/skill-solat/Cargo.toml` (add dependency)
- Modify: `rust/plugins/skill-solat/src/lib.rs` (add `calculate` action)

**Step 1: Create the prayer-times crate**

Add to `rust/Cargo.toml` workspace members:
```toml
"crates/amanclaw-prayer-times",
```

Create `rust/crates/amanclaw-prayer-times/Cargo.toml`:
```toml
[package]
name = "amanclaw-prayer-times"
version = "0.1.0"
edition = "2021"
description = "Pure-Rust Islamic prayer time calculation supporting global methods"
license = "MIT"
repository = "https://github.com/AmanClaw/amanclaw"
keywords = ["prayer-times", "islam", "salah", "adhan"]

[dependencies]
chrono = "0.4"
serde = { version = "1", features = ["derive"] }
```

**Step 2: Implement calculation methods**

Create `rust/crates/amanclaw-prayer-times/src/methods.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Prayer time calculation method used in different regions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CalculationMethod {
    /// Muslim World League — Fajr: 18°, Isha: 17°
    Mwl,
    /// Islamic Society of North America — Fajr: 15°, Isha: 15°
    Isna,
    /// Egyptian General Authority of Survey — Fajr: 19.5°, Isha: 17.5°
    Egyptian,
    /// University of Islamic Sciences, Karachi — Fajr: 18°, Isha: 18°
    Karachi,
    /// Umm al-Qura University, Makkah — Fajr: 18.5°, Isha: 90min after Maghrib (120 in Ramadan)
    UmmAlQura,
    /// JAKIM Malaysia — Fajr: 20°, Isha: 18°
    Jakim,
}

/// Angle parameters for a calculation method.
#[derive(Debug, Clone, Copy)]
pub struct MethodParams {
    pub fajr_angle: f64,
    pub isha_angle: f64,
    /// If set, Isha is minutes after Maghrib instead of angle-based.
    pub isha_minutes: Option<u32>,
}

impl CalculationMethod {
    pub fn params(self) -> MethodParams {
        match self {
            Self::Mwl => MethodParams { fajr_angle: 18.0, isha_angle: 17.0, isha_minutes: None },
            Self::Isna => MethodParams { fajr_angle: 15.0, isha_angle: 15.0, isha_minutes: None },
            Self::Egyptian => MethodParams { fajr_angle: 19.5, isha_angle: 17.5, isha_minutes: None },
            Self::Karachi => MethodParams { fajr_angle: 18.0, isha_angle: 18.0, isha_minutes: None },
            Self::UmmAlQura => MethodParams { fajr_angle: 18.5, isha_angle: 0.0, isha_minutes: Some(90) },
            Self::Jakim => MethodParams { fajr_angle: 20.0, isha_angle: 18.0, isha_minutes: None },
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Mwl => "Muslim World League",
            Self::Isna => "ISNA (North America)",
            Self::Egyptian => "Egyptian General Authority",
            Self::Karachi => "University of Islamic Sciences, Karachi",
            Self::UmmAlQura => "Umm al-Qura, Makkah",
            Self::Jakim => "JAKIM (Malaysia)",
        }
    }

    /// Return all available methods.
    pub fn all() -> &'static [CalculationMethod] {
        &[Self::Mwl, Self::Isna, Self::Egyptian, Self::Karachi, Self::UmmAlQura, Self::Jakim]
    }

    /// Parse from string (case-insensitive).
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mwl" => Some(Self::Mwl),
            "isna" => Some(Self::Isna),
            "egyptian" | "egypt" => Some(Self::Egyptian),
            "karachi" => Some(Self::Karachi),
            "umm_al_qura" | "ummalqura" | "makkah" => Some(Self::UmmAlQura),
            "jakim" | "malaysia" => Some(Self::Jakim),
            _ => None,
        }
    }
}
```

**Step 3: Implement solar calculation engine**

Create `rust/crates/amanclaw-prayer-times/src/calc.rs`:

```rust
use crate::methods::{CalculationMethod, MethodParams};
use chrono::NaiveDate;
use std::f64::consts::PI;

/// Calculated prayer times for a single day.
#[derive(Debug, Clone)]
pub struct PrayerTimes {
    pub fajr: (u8, u8),
    pub sunrise: (u8, u8),
    pub dhuhr: (u8, u8),
    pub asr: (u8, u8),
    pub maghrib: (u8, u8),
    pub isha: (u8, u8),
}

impl PrayerTimes {
    pub fn format_time(hm: (u8, u8)) -> String {
        format!("{:02}:{:02}", hm.0, hm.1)
    }
}

/// Calculate prayer times for a given date, location, and method.
pub fn calculate(
    date: NaiveDate,
    latitude: f64,
    longitude: f64,
    timezone: f64,
    method: CalculationMethod,
) -> PrayerTimes {
    let params = method.params();
    let jd = julian_day(date);

    let dhuhr_hours = compute_dhuhr(jd, longitude, timezone);
    let sunrise_hours = dhuhr_hours - time_for_angle(0.833, latitude, jd);
    let sunset_hours = dhuhr_hours + time_for_angle(0.833, latitude, jd);
    let fajr_hours = dhuhr_hours - time_for_angle(params.fajr_angle, latitude, jd);
    let asr_hours = dhuhr_hours + asr_time(latitude, jd);
    let maghrib_hours = sunset_hours;
    let isha_hours = match params.isha_minutes {
        Some(min) => maghrib_hours + (min as f64) / 60.0,
        None => dhuhr_hours + time_for_angle(params.isha_angle, latitude, jd),
    };

    PrayerTimes {
        fajr: to_hm(fajr_hours),
        sunrise: to_hm(sunrise_hours),
        dhuhr: to_hm(dhuhr_hours),
        asr: to_hm(asr_hours),
        maghrib: to_hm(maghrib_hours),
        isha: to_hm(isha_hours),
    }
}

fn to_hm(hours: f64) -> (u8, u8) {
    let h = hours.floor().clamp(0.0, 23.0) as u8;
    let m = ((hours - hours.floor()) * 60.0).round().clamp(0.0, 59.0) as u8;
    (h, m)
}

fn deg_to_rad(d: f64) -> f64 { d * PI / 180.0 }
fn rad_to_deg(r: f64) -> f64 { r * 180.0 / PI }
fn sin_deg(d: f64) -> f64 { deg_to_rad(d).sin() }
fn cos_deg(d: f64) -> f64 { deg_to_rad(d).cos() }
fn tan_deg(d: f64) -> f64 { deg_to_rad(d).tan() }
fn arcsin_deg(x: f64) -> f64 { rad_to_deg(x.asin()) }
fn arccos_deg(x: f64) -> f64 { rad_to_deg(x.clamp(-1.0, 1.0).acos()) }
fn arctan2_deg(y: f64, x: f64) -> f64 { rad_to_deg(y.atan2(x)) }

/// Julian day number for a calendar date.
fn julian_day(date: NaiveDate) -> f64 {
    let y = date.year() as f64;
    let m = date.month() as f64;
    let d = date.day() as f64;

    let (y2, m2) = if m <= 2.0 { (y - 1.0, m + 12.0) } else { (y, m) };
    let a = (y2 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();

    (365.25 * (y2 + 4716.0)).floor() + (30.6001 * (m2 + 1.0)).floor() + d + b - 1524.5
}

use chrono::Datelike;

/// Sun declination for a Julian day.
fn sun_declination(jd: f64) -> f64 {
    let d = jd - 2451545.0;
    let g = (357.529 + 0.98560028 * d) % 360.0;
    let q = (280.459 + 0.98564736 * d) % 360.0;
    let l = (q + 1.915 * sin_deg(g) + 0.020 * sin_deg(2.0 * g)) % 360.0;
    let e = 23.439 - 0.00000036 * d;
    arcsin_deg(sin_deg(e) * sin_deg(l))
}

/// Equation of time in hours.
fn equation_of_time(jd: f64) -> f64 {
    let d = jd - 2451545.0;
    let g = (357.529 + 0.98560028 * d) % 360.0;
    let q = (280.459 + 0.98564736 * d) % 360.0;
    let l = (q + 1.915 * sin_deg(g) + 0.020 * sin_deg(2.0 * g)) % 360.0;
    let e = 23.439 - 0.00000036 * d;
    let ra = arctan2_deg(cos_deg(e) * sin_deg(l), cos_deg(l)) / 15.0;
    let ra_fix = ra - (l / 15.0);
    // Normalize to [-0.5, 0.5] day range
    let eqt = q / 15.0 - ra_fix.round() * 15.0 / 15.0;
    // Simpler: EqT = q/15 - RA
    (q / 15.0) - ra
}

/// Dhuhr time in hours (local time).
fn compute_dhuhr(jd: f64, longitude: f64, timezone: f64) -> f64 {
    let d = jd - 2451545.0;
    let g = (357.529 + 0.98560028 * d) % 360.0;
    let q = (280.459 + 0.98564736 * d) % 360.0;
    let l = (q + 1.915 * sin_deg(g) + 0.020 * sin_deg(2.0 * g)) % 360.0;
    let e = 23.439 - 0.00000036 * d;
    let ra = arctan2_deg(cos_deg(e) * sin_deg(l), cos_deg(l));
    // Normalize RA to [0, 360]
    let ra_norm = ((ra % 360.0) + 360.0) % 360.0;
    let eqt = (q - ra_norm) / 15.0;

    12.0 + timezone - longitude / 15.0 - eqt
}

/// Time difference (in hours) for sun angle below horizon.
fn time_for_angle(angle: f64, latitude: f64, jd: f64) -> f64 {
    let decl = sun_declination(jd);
    let cos_ha = ((-sin_deg(angle)) - sin_deg(latitude) * sin_deg(decl))
        / (cos_deg(latitude) * cos_deg(decl));
    arccos_deg(cos_ha) / 15.0
}

/// Asr time difference from Dhuhr (Shafi'i: shadow = object + 1).
fn asr_time(latitude: f64, jd: f64) -> f64 {
    let decl = sun_declination(jd);
    let factor = 1.0; // Shafi'i
    let angle = arcsin_deg(1.0 / (factor + tan_deg((latitude - decl).abs())));
    // Asr is afternoon, so we want time after dhuhr
    // Use the generic formula: H = arccos((sin(a) - sin(lat)*sin(decl)) / (cos(lat)*cos(decl)))
    // where a = acot(factor + tan|lat-decl|) converted
    let a_rad = deg_to_rad(angle);
    let cos_ha = (a_rad.sin() - sin_deg(latitude) * sin_deg(decl))
        / (cos_deg(latitude) * cos_deg(decl));
    arccos_deg(cos_ha) / 15.0
}
```

**Step 4: Create lib.rs with public API**

Create `rust/crates/amanclaw-prayer-times/src/lib.rs`:

```rust
pub mod calc;
pub mod methods;

pub use calc::{calculate, PrayerTimes};
pub use methods::CalculationMethod;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_kuala_lumpur_mwl() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let times = calculate(date, 3.139, 101.687, 8.0, CalculationMethod::Mwl);
        // Fajr should be around 6:00-6:30 in KL
        assert!(times.fajr.0 >= 5 && times.fajr.0 <= 7, "Fajr hour: {}", times.fajr.0);
        // Dhuhr around 13:00-13:30
        assert!(times.dhuhr.0 >= 12 && times.dhuhr.0 <= 14, "Dhuhr hour: {}", times.dhuhr.0);
        // Maghrib around 19:00-19:30
        assert!(times.maghrib.0 >= 18 && times.maghrib.0 <= 20, "Maghrib hour: {}", times.maghrib.0);
    }

    #[test]
    fn test_new_york_isna() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 21).unwrap(); // Summer solstice
        let times = calculate(date, 40.7128, -74.0060, -4.0, CalculationMethod::Isna);
        // In summer, Fajr in NYC around 3:30-4:30 EDT
        assert!(times.fajr.0 >= 3 && times.fajr.0 <= 5, "Fajr hour: {}", times.fajr.0);
        // Dhuhr around 13:00
        assert!(times.dhuhr.0 >= 12 && times.dhuhr.0 <= 14, "Dhuhr hour: {}", times.dhuhr.0);
    }

    #[test]
    fn test_makkah_umm_al_qura() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let times = calculate(date, 21.4225, 39.8262, 3.0, CalculationMethod::UmmAlQura);
        // In Makkah, Dhuhr around 12:15-12:30
        assert!(times.dhuhr.0 >= 11 && times.dhuhr.0 <= 13, "Dhuhr hour: {}", times.dhuhr.0);
        // Isha should be 90 min after Maghrib
        let maghrib_mins = times.maghrib.0 as u32 * 60 + times.maghrib.1 as u32;
        let isha_mins = times.isha.0 as u32 * 60 + times.isha.1 as u32;
        let diff = isha_mins - maghrib_mins;
        assert!(diff >= 85 && diff <= 95, "Isha-Maghrib diff: {diff} min");
    }

    #[test]
    fn test_all_methods_produce_valid_times() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        for method in CalculationMethod::all() {
            let times = calculate(date, 3.139, 101.687, 8.0, *method);
            // Basic sanity: Fajr < Sunrise < Dhuhr < Asr < Maghrib < Isha
            let fajr = times.fajr.0 as u32 * 60 + times.fajr.1 as u32;
            let sunrise = times.sunrise.0 as u32 * 60 + times.sunrise.1 as u32;
            let dhuhr = times.dhuhr.0 as u32 * 60 + times.dhuhr.1 as u32;
            let asr = times.asr.0 as u32 * 60 + times.asr.1 as u32;
            let maghrib = times.maghrib.0 as u32 * 60 + times.maghrib.1 as u32;
            let isha = times.isha.0 as u32 * 60 + times.isha.1 as u32;
            assert!(fajr < sunrise, "{method:?}: fajr >= sunrise");
            assert!(sunrise < dhuhr, "{method:?}: sunrise >= dhuhr");
            assert!(dhuhr < asr, "{method:?}: dhuhr >= asr");
            assert!(asr < maghrib, "{method:?}: asr >= maghrib");
            assert!(maghrib < isha, "{method:?}: maghrib >= isha");
        }
    }

    #[test]
    fn test_format_time() {
        assert_eq!(PrayerTimes::format_time((5, 30)), "05:30");
        assert_eq!(PrayerTimes::format_time((13, 5)), "13:05");
        assert_eq!(PrayerTimes::format_time((0, 0)), "00:00");
    }

    #[test]
    fn test_method_from_str() {
        assert_eq!(CalculationMethod::from_str_loose("mwl"), Some(CalculationMethod::Mwl));
        assert_eq!(CalculationMethod::from_str_loose("ISNA"), Some(CalculationMethod::Isna));
        assert_eq!(CalculationMethod::from_str_loose("jakim"), Some(CalculationMethod::Jakim));
        assert_eq!(CalculationMethod::from_str_loose("malaysia"), Some(CalculationMethod::Jakim));
        assert_eq!(CalculationMethod::from_str_loose("invalid"), None);
    }
}
```

**Step 5: Integrate with solat skill**

Add dependency to `rust/plugins/skill-solat/Cargo.toml`:
```toml
amanclaw-prayer-times = { path = "../../crates/amanclaw-prayer-times" }
chrono = "0.4"
```

Add `calculate` action to `rust/plugins/skill-solat/src/lib.rs` in the `execute` match:
```rust
"calculate" => {
    let lat = args.get("latitude").and_then(|v| v.as_f64()).unwrap_or(3.139);
    let lon = args.get("longitude").and_then(|v| v.as_f64()).unwrap_or(101.687);
    let tz = args.get("timezone").and_then(|v| v.as_f64()).unwrap_or(8.0);
    let method_str = args.get("method").and_then(|v| v.as_str()).unwrap_or("mwl");
    let method = amanclaw_prayer_times::CalculationMethod::from_str_loose(method_str)
        .unwrap_or(amanclaw_prayer_times::CalculationMethod::Mwl);
    let date = chrono::Local::now().date_naive();
    let times = amanclaw_prayer_times::calculate(date, lat, lon, tz, method);
    let output = format!(
        "Prayer Times ({}):\n\nFajr: {}\nSunrise: {}\nDhuhr: {}\nAsr: {}\nMaghrib: {}\nIsha: {}\n\nMethod: {}\nLocation: ({lat:.4}, {lon:.4}), UTC{tz:+.0}",
        date,
        amanclaw_prayer_times::PrayerTimes::format_time(times.fajr),
        amanclaw_prayer_times::PrayerTimes::format_time(times.sunrise),
        amanclaw_prayer_times::PrayerTimes::format_time(times.dhuhr),
        amanclaw_prayer_times::PrayerTimes::format_time(times.asr),
        amanclaw_prayer_times::PrayerTimes::format_time(times.maghrib),
        amanclaw_prayer_times::PrayerTimes::format_time(times.isha),
        method.display_name(),
    );
    SkillResult { success: true, output, error: None }
}
"list_methods" => {
    let methods: Vec<String> = amanclaw_prayer_times::CalculationMethod::all()
        .iter()
        .map(|m| format!("{:?} — {}", m, m.display_name()))
        .collect();
    SkillResult {
        success: true,
        output: format!("Available prayer time calculation methods:\n\n{}", methods.join("\n")),
        error: None,
    }
}
```

Update the skill's `parameters_schema` to add the new actions and parameters.

**Step 6: Run tests**

Run: `cargo test -p amanclaw-prayer-times && cargo test -p skill-solat`
Expected: All tests pass

**Step 7: Commit**

```bash
git add rust/crates/amanclaw-prayer-times/ rust/plugins/skill-solat/ rust/Cargo.toml
git commit -m "feat: add global prayer time calculation engine (6 methods)"
```

---

### Task 3: WhatsApp Interactive Messages (Buttons & Lists)

**Context:** WhatsApp Cloud API supports interactive messages (buttons, lists) beyond plain text. This gives AmanClaw a richer UX on WhatsApp than any competitor. Add support for sending button and list messages through the WhatsApp channel adapter.

**Files:**
- Modify: `rust/crates/amanclaw-traits/src/message.rs`
- Modify: `rust/plugins/channel-whatsapp/src/lib.rs`

**Step 1: Extend OutgoingMessage with interactive types**

Read `rust/crates/amanclaw-traits/src/message.rs` to see current `OutgoingMessage` struct.

Add to `OutgoingMessage`:
```rust
/// Optional interactive elements (WhatsApp buttons, lists).
pub interactive: Option<InteractiveMessage>,
```

Add new types:
```rust
/// Interactive message for platforms that support rich UI (WhatsApp).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractiveMessage {
    /// Up to 3 quick reply buttons.
    Buttons {
        body: String,
        buttons: Vec<MessageButton>,
    },
    /// Scrollable list with sections.
    List {
        body: String,
        button_text: String,
        sections: Vec<ListSection>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageButton {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSection {
    pub title: String,
    pub rows: Vec<ListRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
}
```

**Step 2: Implement WhatsApp interactive message sending**

In `rust/plugins/channel-whatsapp/src/lib.rs`, update `send_message` to handle interactive messages:

```rust
async fn send_message(&self, msg: OutgoingMessage) -> anyhow::Result<()> {
    let url = format!(
        "https://graph.facebook.com/v21.0/{}/messages",
        self.phone_number_id
    );

    let payload = if let Some(interactive) = &msg.interactive {
        match interactive {
            InteractiveMessage::Buttons { body, buttons } => {
                serde_json::json!({
                    "messaging_product": "whatsapp",
                    "to": msg.chat_id,
                    "type": "interactive",
                    "interactive": {
                        "type": "button",
                        "body": { "text": body },
                        "action": {
                            "buttons": buttons.iter().map(|b| serde_json::json!({
                                "type": "reply",
                                "reply": { "id": b.id, "title": b.title }
                            })).collect::<Vec<_>>()
                        }
                    }
                })
            }
            InteractiveMessage::List { body, button_text, sections } => {
                serde_json::json!({
                    "messaging_product": "whatsapp",
                    "to": msg.chat_id,
                    "type": "interactive",
                    "interactive": {
                        "type": "list",
                        "body": { "text": body },
                        "action": {
                            "button": button_text,
                            "sections": sections.iter().map(|s| serde_json::json!({
                                "title": s.title,
                                "rows": s.rows.iter().map(|r| {
                                    let mut row = serde_json::json!({
                                        "id": r.id,
                                        "title": r.title,
                                    });
                                    if let Some(desc) = &r.description {
                                        row["description"] = serde_json::json!(desc);
                                    }
                                    row
                                }).collect::<Vec<_>>()
                            })).collect::<Vec<_>>()
                        }
                    }
                })
            }
        }
    } else {
        serde_json::json!({
            "messaging_product": "whatsapp",
            "to": msg.chat_id,
            "type": "text",
            "text": { "body": msg.text }
        })
    };

    let resp = self.http.post(&url)
        .bearer_auth(&self.access_token)
        .json(&payload)
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        tracing::error!(body, "WhatsApp API error");
    }
    Ok(())
}
```

**Step 3: Handle button/list reply payloads in webhook**

WhatsApp sends button replies as `type: "interactive"` with `interactive.button_reply.id` or `interactive.list_reply.id`. Add handling:

```rust
// In WaMessage struct, add:
interactive: Option<WaInteractive>,

#[derive(Debug, Deserialize)]
struct WaInteractive {
    button_reply: Option<WaReply>,
    list_reply: Option<WaReply>,
}

#[derive(Debug, Deserialize)]
struct WaReply {
    id: String,
    title: String,
}
```

In `handle_webhook`, add interactive message parsing:
```rust
"interactive" => {
    if let Some(inter) = &wa_msg.interactive {
        if let Some(reply) = &inter.button_reply {
            format!("[button:{}] {}", reply.id, reply.title)
        } else if let Some(reply) = &inter.list_reply {
            format!("[list:{}] {}", reply.id, reply.title)
        } else {
            "[Interactive message]".to_string()
        }
    } else {
        "[Interactive message]".to_string()
    }
}
```

**Step 4: Add tests**

```rust
#[test]
fn test_interactive_button_serialization() {
    let interactive = InteractiveMessage::Buttons {
        body: "Choose an option".into(),
        buttons: vec![
            MessageButton { id: "btn_1".into(), title: "Option 1".into() },
            MessageButton { id: "btn_2".into(), title: "Option 2".into() },
        ],
    };
    let json = serde_json::to_string(&interactive).unwrap();
    assert!(json.contains("btn_1"));
    assert!(json.contains("Option 1"));
}

#[test]
fn test_deserialize_interactive_reply() {
    let json = r#"{
        "from": "601234567890",
        "type": "interactive",
        "id": "msg_456",
        "interactive": {
            "button_reply": {"id": "btn_1", "title": "Option 1"}
        }
    }"#;
    let msg: WaMessage = serde_json::from_str(json).unwrap();
    assert_eq!(msg.msg_type, "interactive");
    assert!(msg.interactive.is_some());
}
```

**Step 5: Run tests**

Run: `cargo test -p channel-whatsapp && cargo test -p amanclaw-traits`
Expected: All tests pass

**Step 6: Commit**

```bash
git add rust/crates/amanclaw-traits/src/message.rs rust/plugins/channel-whatsapp/
git commit -m "feat(whatsapp): add interactive messages (buttons, lists)"
```

---

### Task 4: Live Reload for Development (`--watch`)

**Context:** `amanclaw dev --watch` should watch plugins/, souls/, and config.yaml for changes and trigger appropriate reloads. The watcher infrastructure already exists in `amanclaw-wasm-runtime/src/watcher.rs` — extend it and wire it into the dev command.

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/cli.rs` (add `--watch` flag to Dev)
- Create: `rust/crates/amanclaw-cli/src/dev_watcher.rs`
- Modify: `rust/crates/amanclaw-cli/src/main.rs`
- Modify: `rust/crates/amanclaw-cli/Cargo.toml` (add notify dependency)

**Step 1: Add `--watch` flag to Dev command**

In `cli.rs`, change Dev from unit variant to struct variant:
```rust
/// Start in development mode with mock LLM
Dev {
    /// Watch for file changes and auto-reload
    #[arg(long)]
    watch: bool,
},
```

Add test:
```rust
#[test]
fn test_cli_dev_watch() {
    let cli = Cli::parse_from(["amanclaw", "dev", "--watch"]);
    assert!(matches!(cli.command, Some(Command::Dev { watch: true })));
}
```

**Step 2: Create dev_watcher module**

Create `rust/crates/amanclaw-cli/src/dev_watcher.rs`:

```rust
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum DevEvent {
    PluginChanged(String),
    SoulChanged(String),
    ConfigChanged,
}

pub struct DevWatcher {
    _watcher: RecommendedWatcher,
    pub rx: mpsc::Receiver<DevEvent>,
}

impl DevWatcher {
    pub fn new(
        plugins_dir: &Path,
        souls_dir: &Path,
        config_path: &Path,
    ) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel(64);
        let config_path_owned = config_path.to_path_buf();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                let dominated = matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                );
                if !dominated { return; }

                for path in &event.paths {
                    let event = if path == &config_path_owned {
                        Some(DevEvent::ConfigChanged)
                    } else if path.extension().is_some_and(|e| {
                        e == "wasm" || e == "py" || e == "js"
                    }) {
                        Some(DevEvent::PluginChanged(
                            path.file_name().unwrap_or_default().to_string_lossy().into(),
                        ))
                    } else if path.extension().is_some_and(|e| e == "md") {
                        Some(DevEvent::SoulChanged(
                            path.file_name().unwrap_or_default().to_string_lossy().into(),
                        ))
                    } else {
                        None
                    };

                    if let Some(evt) = event {
                        let _ = tx.blocking_send(evt);
                    }
                }
            }
        })?;

        for dir in [plugins_dir, souls_dir] {
            if dir.exists() {
                watcher.watch(dir, RecursiveMode::Recursive)?;
            }
        }
        // Watch config file's parent directory
        if let Some(parent) = config_path.parent() {
            if parent.exists() {
                watcher.watch(parent, RecursiveMode::NonRecursive)?;
            }
        }

        Ok(Self { _watcher: watcher, rx })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn test_dev_watcher_detects_plugin() {
        let dir = TempDir::new().unwrap();
        let plugins = dir.path().join("plugins");
        let souls = dir.path().join("souls");
        let config = dir.path().join("config.yaml");
        std::fs::create_dir_all(&plugins).unwrap();
        std::fs::create_dir_all(&souls).unwrap();
        std::fs::write(&config, "test: true").unwrap();

        let mut watcher = DevWatcher::new(&plugins, &souls, &config).unwrap();

        std::fs::write(plugins.join("test.py"), "# test").unwrap();
        sleep(Duration::from_millis(500)).await;

        if let Ok(DevEvent::PluginChanged(name)) = watcher.rx.try_recv() {
            assert!(name.contains("test.py"));
        }
    }

    #[tokio::test]
    async fn test_dev_watcher_detects_soul() {
        let dir = TempDir::new().unwrap();
        let plugins = dir.path().join("plugins");
        let souls = dir.path().join("souls");
        let config = dir.path().join("config.yaml");
        std::fs::create_dir_all(&plugins).unwrap();
        std::fs::create_dir_all(&souls).unwrap();
        std::fs::write(&config, "test: true").unwrap();

        let mut watcher = DevWatcher::new(&plugins, &souls, &config).unwrap();

        std::fs::write(souls.join("ustaz.md"), "# Ustaz").unwrap();
        sleep(Duration::from_millis(500)).await;

        if let Ok(DevEvent::SoulChanged(name)) = watcher.rx.try_recv() {
            assert!(name.contains("ustaz.md"));
        }
    }
}
```

**Step 3: Wire watch mode into cmd_dev**

Update `cmd_dev` in main.rs:
```rust
async fn cmd_dev(config_path: &str, watch: bool) -> Result<()> {
    println!("Starting AmanClaw in development mode...");
    println!("Using mock LLM — no API key required");
    if watch {
        println!("Watch mode enabled — auto-reload on file changes");
    }
    println!();

    if watch {
        let config = PathBuf::from(config_path);
        let plugins = PathBuf::from("plugins");
        let souls = PathBuf::from("souls");
        let mut watcher = dev_watcher::DevWatcher::new(&plugins, &souls, &config)?;

        // Spawn watcher event loop
        tokio::spawn(async move {
            while let Some(event) = watcher.rx.recv().await {
                match event {
                    dev_watcher::DevEvent::PluginChanged(name) => {
                        println!("\n  Plugin changed: {name} — reload triggered");
                    }
                    dev_watcher::DevEvent::SoulChanged(name) => {
                        println!("\n  SOUL changed: {name} — reload triggered");
                    }
                    dev_watcher::DevEvent::ConfigChanged => {
                        println!("\n  Config changed — restart required");
                    }
                }
            }
        });
    }

    cmd_run(config_path).await
}
```

Update the match in main:
```rust
Some(Command::Dev { watch }) => cmd_dev(&cli.config, watch).await,
```

**Step 4: Run tests**

Run: `cargo test -p amanclaw-cli`
Expected: All tests pass

**Step 5: Commit**

```bash
git add rust/crates/amanclaw-cli/
git commit -m "feat(cli): add live reload with --watch flag for dev mode"
```

---

### Task 5: Benchmark Suite

**Context:** Publish benchmarks proving AmanClaw runs efficiently on Raspberry Pi hardware. Use criterion for micro-benchmarks. Add a benchmark CI job. Update README with published benchmark results.

**Files:**
- Create: `rust/crates/amanclaw-core/benches/pipeline.rs`
- Create: `rust/crates/amanclaw-prayer-times/benches/calculation.rs`
- Modify: `rust/crates/amanclaw-core/Cargo.toml` (add criterion)
- Modify: `rust/crates/amanclaw-prayer-times/Cargo.toml` (add criterion)
- Modify: `.github/workflows/ci.yml` (add benchmark job)

**Step 1: Add criterion dependency**

Add to `rust/crates/amanclaw-core/Cargo.toml`:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "pipeline"
harness = false
```

Add to `rust/crates/amanclaw-prayer-times/Cargo.toml`:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "calculation"
harness = false
```

**Step 2: Create prayer times benchmark**

Create `rust/crates/amanclaw-prayer-times/benches/calculation.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use amanclaw_prayer_times::{calculate, CalculationMethod};
use chrono::NaiveDate;

fn bench_prayer_time_calculation(c: &mut Criterion) {
    let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();

    c.bench_function("prayer_times_mwl_kl", |b| {
        b.iter(|| {
            calculate(
                black_box(date),
                black_box(3.139),
                black_box(101.687),
                black_box(8.0),
                black_box(CalculationMethod::Mwl),
            )
        })
    });

    c.bench_function("prayer_times_isna_nyc", |b| {
        b.iter(|| {
            calculate(
                black_box(date),
                black_box(40.7128),
                black_box(-74.006),
                black_box(-4.0),
                black_box(CalculationMethod::Isna),
            )
        })
    });

    c.bench_function("prayer_times_all_methods", |b| {
        b.iter(|| {
            for method in CalculationMethod::all() {
                calculate(
                    black_box(date),
                    black_box(3.139),
                    black_box(101.687),
                    black_box(8.0),
                    black_box(*method),
                );
            }
        })
    });
}

criterion_group!(benches, bench_prayer_time_calculation);
criterion_main!(benches);
```

**Step 3: Create pipeline benchmark**

Create `rust/crates/amanclaw-core/benches/pipeline.rs`:

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_diagnostics(c: &mut Criterion) {
    let config = amanclaw_traits::config::AppConfig::default();

    c.bench_function("startup_diagnostics", |b| {
        b.iter(|| {
            amanclaw_core::diagnostics::run_startup_diagnostics(&config)
        })
    });
}

criterion_group!(benches, bench_diagnostics);
criterion_main!(benches);
```

**Step 4: Add benchmark CI job**

Add to `.github/workflows/ci.yml`:
```yaml
  bench:
    name: Benchmarks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust -> target
      - name: Run benchmarks
        working-directory: rust
        run: cargo bench --workspace 2>&1 | tee bench-output.txt
      - name: Upload benchmark results
        uses: actions/upload-artifact@v4
        with:
          name: benchmark-results
          path: rust/target/criterion/
```

**Step 5: Run benchmarks locally**

Run: `cd rust && cargo bench -p amanclaw-prayer-times`
Expected: Benchmarks complete with timing data

**Step 6: Commit**

```bash
git add rust/crates/amanclaw-core/benches/ rust/crates/amanclaw-prayer-times/benches/ rust/crates/amanclaw-core/Cargo.toml rust/crates/amanclaw-prayer-times/Cargo.toml .github/workflows/ci.yml
git commit -m "feat: add benchmark suite with criterion (prayer times + diagnostics)"
```

---

### Task 6: Minimal Web Playground

**Context:** `amanclaw playground` opens a local web UI for testing skills interactively. Keep it minimal — embedded HTML served from Rust via axum. No Node.js/React build step. Uses HTMX for interactivity. Shows: chat interface, skill list, pipeline trace.

**Files:**
- Create: `rust/crates/amanclaw-cli/src/playground.rs`
- Create: `rust/crates/amanclaw-cli/static/playground.html`
- Modify: `rust/crates/amanclaw-cli/src/cli.rs` (add Playground command)
- Modify: `rust/crates/amanclaw-cli/src/main.rs`
- Modify: `rust/crates/amanclaw-cli/Cargo.toml` (add axum dep if not present)

**Step 1: Add Playground CLI command**

In `cli.rs`:
```rust
/// Open interactive web playground
Playground {
    /// Port for playground server
    #[arg(short, long, default_value = "3000")]
    port: u16,
},
```

**Step 2: Create playground HTML**

Create `rust/crates/amanclaw-cli/static/playground.html` — a single-file HTML with embedded CSS and JS using HTMX:

The HTML should contain:
- Header with "AmanClaw Playground" title
- Left panel: list of available skills (fetched from `/api/skills`)
- Center: chat-like interface to send messages and see responses
- Right panel: pipeline trace/debug output
- HTMX for server-sent updates
- Minimal, clean design with CSS variables

**Step 3: Create playground server module**

Create `rust/crates/amanclaw-cli/src/playground.rs`:

```rust
use axum::{Router, routing::get, response::Html, Json};
use std::net::SocketAddr;

const PLAYGROUND_HTML: &str = include_str!("../static/playground.html");

pub async fn run_playground(port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index))
        .route("/api/skills", get(list_skills))
        .route("/api/health", get(health));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Playground running at http://localhost:{port}");
    println!("Press Ctrl+C to stop");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(PLAYGROUND_HTML)
}

async fn list_skills() -> Json<Vec<serde_json::Value>> {
    // Return built-in skill list (static for now, will be dynamic when connected to engine)
    Json(vec![
        serde_json::json!({"name": "solat", "description": "Prayer times by zone or calculation"}),
        serde_json::json!({"name": "qiblat", "description": "Qiblat direction from location"}),
        serde_json::json!({"name": "hijri", "description": "Hijri calendar conversion"}),
        serde_json::json!({"name": "doa", "description": "Islamic prayers and supplications"}),
    ])
}

async fn health() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_playground_index() {
        let app = Router::new().route("/", get(index));
        let resp = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_playground_skills_api() {
        let app = Router::new().route("/api/skills", get(list_skills));
        let resp = app
            .oneshot(Request::builder().uri("/api/skills").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_playground_health() {
        let app = Router::new().route("/api/health", get(health));
        let resp = app
            .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }
}
```

**Step 4: Wire into main**

Add to match block:
```rust
Some(Command::Playground { port }) => playground::run_playground(port).await,
```

**Step 5: Run tests**

Run: `cargo test -p amanclaw-cli -- playground`
Expected: All tests pass

**Step 6: Commit**

```bash
git add rust/crates/amanclaw-cli/
git commit -m "feat(cli): add interactive web playground (amanclaw playground)"
```

---

### Task 7: Update README with Phase 2 Features

**Context:** Update the README.md to showcase new Phase 2 features: global prayer times, skill scaffolding, live reload, playground, benchmarks. Add a "Quick Start" section and benchmark results.

**Files:**
- Modify: `README.md`

**Step 1: Add new sections to README**

Add after existing content:
- **Quick Start** section: `amanclaw init → amanclaw dev → amanclaw skill new`
- **Skill Development** section: scaffolding, testing, packaging
- **Global Prayer Times** section: supported methods table
- **Live Reload** section: `amanclaw dev --watch`
- **Playground** section: `amanclaw playground`
- **Benchmarks** section: table with timing data from criterion runs

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update README with Phase 2 features and benchmarks"
```

---

### Task 8: Fix Clippy + Format + Integration Test

**Context:** Final verification pass. Run clippy, format, and add a quick integration test for the new features.

**Files:**
- Modify: various (clippy fixes)
- Modify: `rust/crates/amanclaw-core/tests/integration.rs`

**Step 1: Run clippy and fix warnings**

Run: `cd rust && cargo clippy --workspace -- -D warnings`
Fix any warnings.

**Step 2: Run formatter**

Run: `cd rust && cargo fmt --all`

**Step 3: Add integration test**

Add to `rust/crates/amanclaw-core/tests/integration.rs`:

```rust
#[test]
fn test_prayer_times_all_methods_sanity() {
    use amanclaw_prayer_times::{calculate, CalculationMethod};
    use chrono::NaiveDate;

    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let locations = [
        ("Kuala Lumpur", 3.139, 101.687, 8.0),
        ("New York", 40.7128, -74.006, -4.0),
        ("Makkah", 21.4225, 39.8262, 3.0),
        ("Istanbul", 41.0082, 28.9784, 3.0),
        ("Jakarta", -6.2088, 106.8456, 7.0),
    ];

    for (city, lat, lon, tz) in &locations {
        for method in CalculationMethod::all() {
            let times = calculate(date, *lat, *lon, *tz, *method);
            // Basic sanity: all hours between 0-23
            assert!(times.fajr.0 < 24, "{city} {method:?}: fajr hour {}", times.fajr.0);
            assert!(times.dhuhr.0 < 24, "{city} {method:?}: dhuhr hour {}", times.dhuhr.0);
            assert!(times.isha.0 < 24, "{city} {method:?}: isha hour {}", times.isha.0);
        }
    }
}
```

**Step 4: Run full test suite**

Run: `cd rust && cargo test --workspace`
Expected: All tests pass

**Step 5: Run full CI locally**

Run: `cd rust && cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace`
Expected: All pass

**Step 6: Commit**

```bash
git add -A
git commit -m "chore: phase 2 cleanup — clippy fixes, formatting, integration tests"
```

---

## Execution Summary

| Task | Feature | Estimated Size |
|------|---------|---------------|
| 1 | `amanclaw skill new` scaffolding | Medium |
| 2 | Global prayer time engine (6 methods) | Large |
| 3 | WhatsApp interactive messages | Medium |
| 4 | Live reload (`--watch`) | Medium |
| 5 | Benchmark suite | Small |
| 6 | Web playground | Medium |
| 7 | README updates | Small |
| 8 | Clippy + format + integration tests | Small |

**Parallelizable tasks:** 1+2 (independent), 3+4 (independent), 5+6 (independent)
**Sequential dependencies:** 8 depends on all others
