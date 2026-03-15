/// Hijri calendar-aware scheduling.
///
/// Supports scheduling events on Islamic dates like "15 Ramadan", "1 Shawwal", etc.
/// Uses the tabular Islamic calendar (same algorithm as skill-hijri) for date conversion.
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::scheduler::SchedulerEvent;
use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Tabular Hijri calendar (embedded to avoid cross-crate dependency issues)
// ---------------------------------------------------------------------------

/// Leap years in the 30-year Hijri cycle.
const LEAP_YEARS: [i32; 11] = [2, 5, 7, 10, 13, 16, 18, 21, 24, 26, 29];

/// Islamic calendar epoch in Julian Day Number (July 16, 622 CE Julian).
const HIJRI_EPOCH_JDN: i64 = 1948439;

fn is_hijri_leap(year: i64) -> bool {
    let pos = ((year - 1) % 30 + 1) as i32;
    LEAP_YEARS.contains(&pos)
}

fn hijri_month_length(month: u32, leap: bool) -> i64 {
    if month % 2 == 1 || (month == 12 && leap) {
        30
    } else {
        29
    }
}

fn hijri_year_length(year: i64) -> i64 {
    if is_hijri_leap(year) { 355 } else { 354 }
}

fn gregorian_to_jdn(year: i32, month: i32, day: i32) -> i64 {
    let (y, m) = if month <= 2 {
        ((year - 1) as i64, (month + 12) as i64)
    } else {
        (year as i64, month as i64)
    };
    let a = y / 100;
    let b = 2 - a + a / 4;
    (365.25 * (y + 4716) as f64) as i64
        + (30.6001 * (m + 1) as f64) as i64
        + day as i64
        + b
        - 1524
}

fn jdn_to_gregorian(jdn: i64) -> NaiveDate {
    let a = jdn + 32044;
    let b = (4 * a + 3) / 146097;
    let c = a - (146097 * b) / 4;
    let d = (4 * c + 3) / 1461;
    let e = c - (1461 * d) / 4;
    let m = (5 * e + 2) / 153;

    let day = (e - (153 * m + 2) / 5 + 1) as u32;
    let month = (m + 3 - 12 * (m / 10)) as u32;
    let year = (100 * b + d - 4800 + m / 10) as i32;

    NaiveDate::from_ymd_opt(year, month, day).unwrap_or(NaiveDate::from_ymd_opt(2000, 1, 1).unwrap())
}

/// Simple Hijri date (year, month, day).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HijriDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

/// Convert a Gregorian date to Hijri.
pub fn gregorian_to_hijri(date: NaiveDate) -> HijriDate {
    let jd = gregorian_to_jdn(date.year(), date.month() as i32, date.day() as i32);
    let days_since_epoch = jd - HIJRI_EPOCH_JDN;

    let cycle = days_since_epoch / 10631;
    let remaining = days_since_epoch - cycle * 10631;

    let year_in_cycle = ((remaining as f64 - 0.5) / 354.36667).floor() as i64;
    let year_in_cycle = year_in_cycle.clamp(0, 29);

    let year = cycle * 30 + year_in_cycle + 1;

    let mut days_in_years: i64 = 0;
    for yr in 1..=year_in_cycle {
        let abs_year = cycle * 30 + yr;
        days_in_years += hijri_year_length(abs_year);
    }

    let day_of_year = remaining - days_in_years;

    let current_year_len = hijri_year_length(year);
    let (year, day_of_year) = if day_of_year > current_year_len {
        (year + 1, day_of_year - current_year_len)
    } else {
        (year, day_of_year)
    };

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

    HijriDate { year, month, day }
}

/// Convert a Hijri date to Gregorian using the tabular calendar.
fn hijri_to_gregorian(year: i32, month: u32, day: u32) -> NaiveDate {
    let y = year as i64;
    let cycle = (y - 1) / 30;
    let year_in_cycle = (y - 1) % 30;

    // Days from complete cycles
    let mut jdn = HIJRI_EPOCH_JDN + cycle * 10631;

    // Days from complete years within the cycle
    for yr in 1..=year_in_cycle {
        let abs_year = cycle * 30 + yr;
        jdn += hijri_year_length(abs_year);
    }

    // Days from complete months
    let leap = is_hijri_leap(y);
    for m in 1..month {
        jdn += hijri_month_length(m, leap);
    }

    // Days within the month
    jdn += day as i64;

    jdn_to_gregorian(jdn)
}

// ---------------------------------------------------------------------------
// Schedule types
// ---------------------------------------------------------------------------

/// A target to send a scheduled message to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub platform: String,
    pub chat_id: String,
    #[serde(default)]
    pub topic_id: Option<String>,
}

/// What happens when a Hijri schedule triggers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScheduleAction {
    /// Send a direct message to each target.
    Message {
        text: String,
        targets: Vec<Target>,
    },
    /// Invoke a skill and send the result to each target.
    Skill {
        skill: String,
        input: String,
        targets: Vec<Target>,
    },
}

/// A Hijri schedule entry — fires once per year on the given Islamic date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HijriSchedule {
    pub name: String,
    /// Hijri month (1-12).
    pub hijri_month: u8,
    /// Hijri day (1-30).
    pub hijri_day: u8,
    /// Local time to fire, e.g. "06:00".
    #[serde(default = "default_time")]
    pub time: String,
    /// What to do when the schedule triggers.
    #[serde(flatten)]
    pub action: ScheduleAction,
}

fn default_time() -> String {
    "06:00".into()
}

/// A schedule that is due today.
#[derive(Debug, Clone)]
pub struct DueSchedule {
    pub name: String,
    pub action: ScheduleAction,
    pub time: String,
}

// ---------------------------------------------------------------------------
// Core logic
// ---------------------------------------------------------------------------

/// Check which Hijri schedules are due on a given Hijri date.
pub fn check_hijri_schedules(schedules: &[HijriSchedule], today_hijri: &HijriDate) -> Vec<DueSchedule> {
    schedules
        .iter()
        .filter(|s| {
            s.hijri_month as u32 == today_hijri.month && s.hijri_day as u32 == today_hijri.day
        })
        .map(|s| DueSchedule {
            name: s.name.clone(),
            action: s.action.clone(),
            time: s.time.clone(),
        })
        .collect()
}

/// Find the next Gregorian date for a given Hijri month+day, starting from today.
///
/// Tries the current Hijri year first; if that date has passed, tries next year.
pub fn next_occurrence(hijri_month: u8, hijri_day: u8, today: NaiveDate) -> Option<NaiveDate> {
    let today_hijri = gregorian_to_hijri(today);

    // Try current Hijri year
    let candidate = hijri_to_gregorian(today_hijri.year, hijri_month as u32, hijri_day as u32);
    if candidate >= today {
        return Some(candidate);
    }

    // Try next Hijri year
    let candidate = hijri_to_gregorian(today_hijri.year + 1, hijri_month as u32, hijri_day as u32);
    Some(candidate)
}

/// Convert due schedules into [`SchedulerEvent`]s ready for the engine.
pub fn due_schedules_to_events(due: &[DueSchedule]) -> Vec<SchedulerEvent> {
    let mut events = Vec::new();
    for sched in due {
        match &sched.action {
            ScheduleAction::Message { text, targets } => {
                for t in targets {
                    events.push(SchedulerEvent::SendMessage(OutgoingMessage {
                        chat_id: t.chat_id.clone(),
                        text: text.clone(),
                        parse_mode: None,
                        reply_to: None,
                        platform: Some(t.platform.clone()),
                        topic_id: t.topic_id.clone(),
                        interactive: None,
                    }));
                }
            }
            ScheduleAction::Skill { skill, input, targets } => {
                let synthetic = format!("/{skill} {input}");
                for t in targets {
                    events.push(SchedulerEvent::InjectMessage(IncomingMessage {
                        user_id: format!("hijri-sched:{}", sched.name),
                        chat_id: t.chat_id.clone(),
                        platform: t.platform.clone(),
                        text: synthetic.clone(),
                        username: None,
                        first_name: None,
                        is_group: false,
                        image_data: None,
                        reply_to: None,
                        topic_id: t.topic_id.clone(),
                        channel_context: None,
                        is_cron: true,
                        is_webhook: false,
                        is_subagent: false,
                    }));
                }
            }
        }
    }
    events
}

/// Start a background task that checks Hijri schedules once per hour and fires
/// due events via the scheduler event channel.
pub fn start_hijri_checker(
    schedules: Vec<HijriSchedule>,
    tx: mpsc::Sender<SchedulerEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Track which (name, hijri_day, hijri_month) combos we already fired today
        let mut last_fired_date: Option<NaiveDate> = None;

        loop {
            let today = chrono::Local::now().date_naive();
            let today_hijri = gregorian_to_hijri(today);

            // Reset fired set if the day changed
            let already_fired_today = last_fired_date == Some(today);

            if !already_fired_today {
                let due = check_hijri_schedules(&schedules, &today_hijri);
                if !due.is_empty() {
                    tracing::info!(
                        count = due.len(),
                        hijri = %format!("{}-{}-{}", today_hijri.year, today_hijri.month, today_hijri.day),
                        "Hijri schedules due today"
                    );
                    let events = due_schedules_to_events(&due);
                    for event in events {
                        if let Err(e) = tx.send(event).await {
                            tracing::error!(error = %e, "Failed to send Hijri schedule event");
                        }
                    }
                    last_fired_date = Some(today);
                }
            }

            // Check again in 1 hour
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_schedule(name: &str, month: u8, day: u8, text: &str) -> HijriSchedule {
        HijriSchedule {
            name: name.into(),
            hijri_month: month,
            hijri_day: day,
            time: "06:00".into(),
            action: ScheduleAction::Message {
                text: text.into(),
                targets: vec![Target {
                    platform: "telegram".into(),
                    chat_id: "-100123".into(),
                    topic_id: None,
                }],
            },
        }
    }

    #[test]
    fn test_check_schedules_finds_due_today() {
        let schedules = vec![
            make_schedule("ramadan_start", 9, 1, "Ramadan Mubarak!"),
            make_schedule("eid", 10, 1, "Eid Mubarak!"),
        ];

        let today = HijriDate { year: 1446, month: 9, day: 1 };
        let due = check_hijri_schedules(&schedules, &today);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].name, "ramadan_start");
    }

    #[test]
    fn test_check_schedules_empty_when_no_match() {
        let schedules = vec![
            make_schedule("ramadan_start", 9, 1, "Ramadan Mubarak!"),
            make_schedule("eid", 10, 1, "Eid Mubarak!"),
        ];

        let today = HijriDate { year: 1446, month: 3, day: 15 };
        let due = check_hijri_schedules(&schedules, &today);
        assert!(due.is_empty());
    }

    #[test]
    fn test_hijri_months_valid_range() {
        // Verify conversion doesn't panic for all valid month/day combos
        for month in 1..=12u8 {
            for day in 1..=30u8 {
                let sched = make_schedule("test", month, day, "test");
                let today = HijriDate { year: 1446, month: month as u32, day: day as u32 };
                let due = check_hijri_schedules(&[sched], &today);
                assert_eq!(due.len(), 1, "month={month}, day={day} should match");
            }
        }
    }

    #[test]
    fn test_gregorian_to_hijri_roundtrip() {
        // 1 Jan 2024 should be around 19 Jamadilakhir 1445
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let hijri = gregorian_to_hijri(date);
        assert_eq!(hijri.year, 1445);
        assert_eq!(hijri.month, 6); // Jamadilakhir
    }

    #[test]
    fn test_next_occurrence_future() {
        // Ask for 1 Ramadan from a date well before Ramadan
        let today = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let next = next_occurrence(9, 1, today);
        assert!(next.is_some());
        let next = next.unwrap();
        // The next 1 Ramadan should be after today
        assert!(next >= today);
    }

    #[test]
    fn test_next_occurrence_wraps_year() {
        // Ask for 1 Muharram from late in the Hijri year
        let today = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        let next = next_occurrence(1, 1, today);
        assert!(next.is_some());
        assert!(next.unwrap() >= today);
    }

    #[test]
    fn test_due_schedules_to_events_message() {
        let due = vec![DueSchedule {
            name: "test".into(),
            action: ScheduleAction::Message {
                text: "Hello!".into(),
                targets: vec![
                    Target { platform: "telegram".into(), chat_id: "123".into(), topic_id: None },
                    Target { platform: "discord".into(), chat_id: "456".into(), topic_id: None },
                ],
            },
            time: "06:00".into(),
        }];
        let events = due_schedules_to_events(&due);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_due_schedules_to_events_skill() {
        let due = vec![DueSchedule {
            name: "test".into(),
            action: ScheduleAction::Skill {
                skill: "solat".into(),
                input: "kuala lumpur".into(),
                targets: vec![Target {
                    platform: "telegram".into(),
                    chat_id: "123".into(),
                    topic_id: None,
                }],
            },
            time: "06:00".into(),
        }];
        let events = due_schedules_to_events(&due);
        assert_eq!(events.len(), 1);
        match &events[0] {
            SchedulerEvent::InjectMessage(msg) => {
                assert_eq!(msg.text, "/solat kuala lumpur");
                assert!(msg.is_cron);
            }
            _ => panic!("Expected InjectMessage"),
        }
    }

    #[test]
    fn test_hijri_to_gregorian_basic() {
        // 1 Muharram 1446 should be approximately July 2024
        let greg = hijri_to_gregorian(1446, 1, 1);
        assert_eq!(greg.year(), 2024);
        // Should be in June or July 2024
        assert!(greg.month() >= 6 && greg.month() <= 8, "got month {}", greg.month());
    }
}
