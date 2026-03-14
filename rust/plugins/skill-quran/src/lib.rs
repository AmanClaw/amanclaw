mod api;

use std::sync::Arc;

use amanclaw_islamic_db::IslamicDb;
use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};

pub struct QuranSkill {
    db: Option<Arc<IslamicDb>>,
}

impl QuranSkill {
    /// Create a QuranSkill backed by the local IslamicDb.
    pub fn new(db: Arc<IslamicDb>) -> Self {
        Self { db: Some(db) }
    }
}

impl Default for QuranSkill {
    /// Create a QuranSkill without a local DB (API-only fallback).
    fn default() -> Self {
        Self { db: None }
    }
}

/// All 114 surahs: (number, Arabic name, transliteration, verse count).
const SURAHS: [(u32, &str, &str, u32); 114] = [
    (
        1,
        "\u{0627}\u{0644}\u{0641}\u{0627}\u{062A}\u{062D}\u{0629}",
        "Al-Fatihah",
        7,
    ),
    (
        2,
        "\u{0627}\u{0644}\u{0628}\u{0642}\u{0631}\u{0629}",
        "Al-Baqarah",
        286,
    ),
    (
        3,
        "\u{0622}\u{0644} \u{0639}\u{0645}\u{0631}\u{0627}\u{0646}",
        "Ali 'Imran",
        200,
    ),
    (
        4,
        "\u{0627}\u{0644}\u{0646}\u{0633}\u{0627}\u{0621}",
        "An-Nisa",
        176,
    ),
    (
        5,
        "\u{0627}\u{0644}\u{0645}\u{0627}\u{0626}\u{062F}\u{0629}",
        "Al-Ma'idah",
        120,
    ),
    (
        6,
        "\u{0627}\u{0644}\u{0623}\u{0646}\u{0639}\u{0627}\u{0645}",
        "Al-An'am",
        165,
    ),
    (
        7,
        "\u{0627}\u{0644}\u{0623}\u{0639}\u{0631}\u{0627}\u{0641}",
        "Al-A'raf",
        206,
    ),
    (
        8,
        "\u{0627}\u{0644}\u{0623}\u{0646}\u{0641}\u{0627}\u{0644}",
        "Al-Anfal",
        75,
    ),
    (
        9,
        "\u{0627}\u{0644}\u{062A}\u{0648}\u{0628}\u{0629}",
        "At-Tawbah",
        129,
    ),
    (10, "\u{064A}\u{0648}\u{0646}\u{0633}", "Yunus", 109),
    (11, "\u{0647}\u{0648}\u{062F}", "Hud", 123),
    (12, "\u{064A}\u{0648}\u{0633}\u{0641}", "Yusuf", 111),
    (
        13,
        "\u{0627}\u{0644}\u{0631}\u{0639}\u{062F}",
        "Ar-Ra'd",
        43,
    ),
    (
        14,
        "\u{0625}\u{0628}\u{0631}\u{0627}\u{0647}\u{064A}\u{0645}",
        "Ibrahim",
        52,
    ),
    (
        15,
        "\u{0627}\u{0644}\u{062D}\u{062C}\u{0631}",
        "Al-Hijr",
        99,
    ),
    (
        16,
        "\u{0627}\u{0644}\u{0646}\u{062D}\u{0644}",
        "An-Nahl",
        128,
    ),
    (
        17,
        "\u{0627}\u{0644}\u{0625}\u{0633}\u{0631}\u{0627}\u{0621}",
        "Al-Isra",
        111,
    ),
    (
        18,
        "\u{0627}\u{0644}\u{0643}\u{0647}\u{0641}",
        "Al-Kahf",
        110,
    ),
    (19, "\u{0645}\u{0631}\u{064A}\u{0645}", "Maryam", 98),
    (20, "\u{0637}\u{0647}", "Taha", 135),
    (
        21,
        "\u{0627}\u{0644}\u{0623}\u{0646}\u{0628}\u{064A}\u{0627}\u{0621}",
        "Al-Anbiya",
        112,
    ),
    (22, "\u{0627}\u{0644}\u{062D}\u{062C}", "Al-Hajj", 78),
    (
        23,
        "\u{0627}\u{0644}\u{0645}\u{0624}\u{0645}\u{0646}\u{0648}\u{0646}",
        "Al-Mu'minun",
        118,
    ),
    (24, "\u{0627}\u{0644}\u{0646}\u{0648}\u{0631}", "An-Nur", 64),
    (
        25,
        "\u{0627}\u{0644}\u{0641}\u{0631}\u{0642}\u{0627}\u{0646}",
        "Al-Furqan",
        77,
    ),
    (
        26,
        "\u{0627}\u{0644}\u{0634}\u{0639}\u{0631}\u{0627}\u{0621}",
        "Ash-Shu'ara",
        227,
    ),
    (
        27,
        "\u{0627}\u{0644}\u{0646}\u{0645}\u{0644}",
        "An-Naml",
        93,
    ),
    (
        28,
        "\u{0627}\u{0644}\u{0642}\u{0635}\u{0635}",
        "Al-Qasas",
        88,
    ),
    (
        29,
        "\u{0627}\u{0644}\u{0639}\u{0646}\u{0643}\u{0628}\u{0648}\u{062A}",
        "Al-Ankabut",
        69,
    ),
    (30, "\u{0627}\u{0644}\u{0631}\u{0648}\u{0645}", "Ar-Rum", 60),
    (31, "\u{0644}\u{0642}\u{0645}\u{0627}\u{0646}", "Luqman", 34),
    (
        32,
        "\u{0627}\u{0644}\u{0633}\u{062C}\u{062F}\u{0629}",
        "As-Sajdah",
        30,
    ),
    (
        33,
        "\u{0627}\u{0644}\u{0623}\u{062D}\u{0632}\u{0627}\u{0628}",
        "Al-Ahzab",
        73,
    ),
    (34, "\u{0633}\u{0628}\u{0623}", "Saba", 54),
    (35, "\u{0641}\u{0627}\u{0637}\u{0631}", "Fatir", 45),
    (36, "\u{064A}\u{0633}", "Ya-Sin", 83),
    (
        37,
        "\u{0627}\u{0644}\u{0635}\u{0627}\u{0641}\u{0627}\u{062A}",
        "As-Saffat",
        182,
    ),
    (38, "\u{0635}", "Sad", 88),
    (
        39,
        "\u{0627}\u{0644}\u{0632}\u{0645}\u{0631}",
        "Az-Zumar",
        75,
    ),
    (40, "\u{063A}\u{0627}\u{0641}\u{0631}", "Ghafir", 85),
    (41, "\u{0641}\u{0635}\u{0644}\u{062A}", "Fussilat", 54),
    (
        42,
        "\u{0627}\u{0644}\u{0634}\u{0648}\u{0631}\u{0649}",
        "Ash-Shura",
        53,
    ),
    (
        43,
        "\u{0627}\u{0644}\u{0632}\u{062E}\u{0631}\u{0641}",
        "Az-Zukhruf",
        89,
    ),
    (
        44,
        "\u{0627}\u{0644}\u{062F}\u{062E}\u{0627}\u{0646}",
        "Ad-Dukhan",
        59,
    ),
    (
        45,
        "\u{0627}\u{0644}\u{062C}\u{0627}\u{062B}\u{064A}\u{0629}",
        "Al-Jathiyah",
        37,
    ),
    (
        46,
        "\u{0627}\u{0644}\u{0623}\u{062D}\u{0642}\u{0627}\u{0641}",
        "Al-Ahqaf",
        35,
    ),
    (47, "\u{0645}\u{062D}\u{0645}\u{062F}", "Muhammad", 38),
    (
        48,
        "\u{0627}\u{0644}\u{0641}\u{062A}\u{062D}",
        "Al-Fath",
        29,
    ),
    (
        49,
        "\u{0627}\u{0644}\u{062D}\u{062C}\u{0631}\u{0627}\u{062A}",
        "Al-Hujurat",
        18,
    ),
    (50, "\u{0642}", "Qaf", 45),
    (
        51,
        "\u{0627}\u{0644}\u{0630}\u{0627}\u{0631}\u{064A}\u{0627}\u{062A}",
        "Adh-Dhariyat",
        60,
    ),
    (52, "\u{0627}\u{0644}\u{0637}\u{0648}\u{0631}", "At-Tur", 49),
    (
        53,
        "\u{0627}\u{0644}\u{0646}\u{062C}\u{0645}",
        "An-Najm",
        62,
    ),
    (
        54,
        "\u{0627}\u{0644}\u{0642}\u{0645}\u{0631}",
        "Al-Qamar",
        55,
    ),
    (
        55,
        "\u{0627}\u{0644}\u{0631}\u{062D}\u{0645}\u{0646}",
        "Ar-Rahman",
        78,
    ),
    (
        56,
        "\u{0627}\u{0644}\u{0648}\u{0627}\u{0642}\u{0639}\u{0629}",
        "Al-Waqi'ah",
        96,
    ),
    (
        57,
        "\u{0627}\u{0644}\u{062D}\u{062F}\u{064A}\u{062F}",
        "Al-Hadid",
        29,
    ),
    (
        58,
        "\u{0627}\u{0644}\u{0645}\u{062C}\u{0627}\u{062F}\u{0644}\u{0629}",
        "Al-Mujadila",
        22,
    ),
    (
        59,
        "\u{0627}\u{0644}\u{062D}\u{0634}\u{0631}",
        "Al-Hashr",
        24,
    ),
    (
        60,
        "\u{0627}\u{0644}\u{0645}\u{0645}\u{062A}\u{062D}\u{0646}\u{0629}",
        "Al-Mumtahanah",
        13,
    ),
    (61, "\u{0627}\u{0644}\u{0635}\u{0641}", "As-Saf", 14),
    (
        62,
        "\u{0627}\u{0644}\u{062C}\u{0645}\u{0639}\u{0629}",
        "Al-Jumu'ah",
        11,
    ),
    (
        63,
        "\u{0627}\u{0644}\u{0645}\u{0646}\u{0627}\u{0641}\u{0642}\u{0648}\u{0646}",
        "Al-Munafiqun",
        11,
    ),
    (
        64,
        "\u{0627}\u{0644}\u{062A}\u{063A}\u{0627}\u{0628}\u{0646}",
        "At-Taghabun",
        18,
    ),
    (
        65,
        "\u{0627}\u{0644}\u{0637}\u{0644}\u{0627}\u{0642}",
        "At-Talaq",
        12,
    ),
    (
        66,
        "\u{0627}\u{0644}\u{062A}\u{062D}\u{0631}\u{064A}\u{0645}",
        "At-Tahrim",
        12,
    ),
    (
        67,
        "\u{0627}\u{0644}\u{0645}\u{0644}\u{0643}",
        "Al-Mulk",
        30,
    ),
    (
        68,
        "\u{0627}\u{0644}\u{0642}\u{0644}\u{0645}",
        "Al-Qalam",
        52,
    ),
    (
        69,
        "\u{0627}\u{0644}\u{062D}\u{0627}\u{0642}\u{0629}",
        "Al-Haqqah",
        52,
    ),
    (
        70,
        "\u{0627}\u{0644}\u{0645}\u{0639}\u{0627}\u{0631}\u{062C}",
        "Al-Ma'arij",
        44,
    ),
    (71, "\u{0646}\u{0648}\u{062D}", "Nuh", 28),
    (72, "\u{0627}\u{0644}\u{062C}\u{0646}", "Al-Jinn", 28),
    (
        73,
        "\u{0627}\u{0644}\u{0645}\u{0632}\u{0645}\u{0644}",
        "Al-Muzzammil",
        20,
    ),
    (
        74,
        "\u{0627}\u{0644}\u{0645}\u{062F}\u{062B}\u{0631}",
        "Al-Muddaththir",
        56,
    ),
    (
        75,
        "\u{0627}\u{0644}\u{0642}\u{064A}\u{0627}\u{0645}\u{0629}",
        "Al-Qiyamah",
        40,
    ),
    (
        76,
        "\u{0627}\u{0644}\u{0625}\u{0646}\u{0633}\u{0627}\u{0646}",
        "Al-Insan",
        31,
    ),
    (
        77,
        "\u{0627}\u{0644}\u{0645}\u{0631}\u{0633}\u{0644}\u{0627}\u{062A}",
        "Al-Mursalat",
        50,
    ),
    (
        78,
        "\u{0627}\u{0644}\u{0646}\u{0628}\u{0623}",
        "An-Naba",
        40,
    ),
    (
        79,
        "\u{0627}\u{0644}\u{0646}\u{0627}\u{0632}\u{0639}\u{0627}\u{062A}",
        "An-Nazi'at",
        46,
    ),
    (80, "\u{0639}\u{0628}\u{0633}", "'Abasa", 42),
    (
        81,
        "\u{0627}\u{0644}\u{062A}\u{0643}\u{0648}\u{064A}\u{0631}",
        "At-Takwir",
        29,
    ),
    (
        82,
        "\u{0627}\u{0644}\u{0627}\u{0646}\u{0641}\u{0637}\u{0627}\u{0631}",
        "Al-Infitar",
        19,
    ),
    (
        83,
        "\u{0627}\u{0644}\u{0645}\u{0637}\u{0641}\u{0641}\u{064A}\u{0646}",
        "Al-Mutaffifin",
        36,
    ),
    (
        84,
        "\u{0627}\u{0644}\u{0627}\u{0646}\u{0634}\u{0642}\u{0627}\u{0642}",
        "Al-Inshiqaq",
        25,
    ),
    (
        85,
        "\u{0627}\u{0644}\u{0628}\u{0631}\u{0648}\u{062C}",
        "Al-Buruj",
        22,
    ),
    (
        86,
        "\u{0627}\u{0644}\u{0637}\u{0627}\u{0631}\u{0642}",
        "At-Tariq",
        17,
    ),
    (
        87,
        "\u{0627}\u{0644}\u{0623}\u{0639}\u{0644}\u{0649}",
        "Al-A'la",
        19,
    ),
    (
        88,
        "\u{0627}\u{0644}\u{063A}\u{0627}\u{0634}\u{064A}\u{0629}",
        "Al-Ghashiyah",
        26,
    ),
    (
        89,
        "\u{0627}\u{0644}\u{0641}\u{062C}\u{0631}",
        "Al-Fajr",
        30,
    ),
    (
        90,
        "\u{0627}\u{0644}\u{0628}\u{0644}\u{062F}",
        "Al-Balad",
        20,
    ),
    (
        91,
        "\u{0627}\u{0644}\u{0634}\u{0645}\u{0633}",
        "Ash-Shams",
        15,
    ),
    (
        92,
        "\u{0627}\u{0644}\u{0644}\u{064A}\u{0644}",
        "Al-Layl",
        21,
    ),
    (
        93,
        "\u{0627}\u{0644}\u{0636}\u{062D}\u{0649}",
        "Ad-Duhaa",
        11,
    ),
    (
        94,
        "\u{0627}\u{0644}\u{0634}\u{0631}\u{062D}",
        "Ash-Sharh",
        8,
    ),
    (95, "\u{0627}\u{0644}\u{062A}\u{064A}\u{0646}", "At-Tin", 8),
    (
        96,
        "\u{0627}\u{0644}\u{0639}\u{0644}\u{0642}",
        "Al-'Alaq",
        19,
    ),
    (97, "\u{0627}\u{0644}\u{0642}\u{062F}\u{0631}", "Al-Qadr", 5),
    (
        98,
        "\u{0627}\u{0644}\u{0628}\u{064A}\u{0646}\u{0629}",
        "Al-Bayyinah",
        8,
    ),
    (
        99,
        "\u{0627}\u{0644}\u{0632}\u{0644}\u{0632}\u{0644}\u{0629}",
        "Az-Zalzalah",
        8,
    ),
    (
        100,
        "\u{0627}\u{0644}\u{0639}\u{0627}\u{062F}\u{064A}\u{0627}\u{062A}",
        "Al-'Adiyat",
        11,
    ),
    (
        101,
        "\u{0627}\u{0644}\u{0642}\u{0627}\u{0631}\u{0639}\u{0629}",
        "Al-Qari'ah",
        11,
    ),
    (
        102,
        "\u{0627}\u{0644}\u{062A}\u{0643}\u{0627}\u{062B}\u{0631}",
        "At-Takathur",
        8,
    ),
    (
        103,
        "\u{0627}\u{0644}\u{0639}\u{0635}\u{0631}",
        "Al-'Asr",
        3,
    ),
    (
        104,
        "\u{0627}\u{0644}\u{0647}\u{0645}\u{0632}\u{0629}",
        "Al-Humazah",
        9,
    ),
    (105, "\u{0627}\u{0644}\u{0641}\u{064A}\u{0644}", "Al-Fil", 5),
    (106, "\u{0642}\u{0631}\u{064A}\u{0634}", "Quraysh", 4),
    (
        107,
        "\u{0627}\u{0644}\u{0645}\u{0627}\u{0639}\u{0648}\u{0646}",
        "Al-Ma'un",
        7,
    ),
    (
        108,
        "\u{0627}\u{0644}\u{0643}\u{0648}\u{062B}\u{0631}",
        "Al-Kawthar",
        3,
    ),
    (
        109,
        "\u{0627}\u{0644}\u{0643}\u{0627}\u{0641}\u{0631}\u{0648}\u{0646}",
        "Al-Kafirun",
        6,
    ),
    (
        110,
        "\u{0627}\u{0644}\u{0646}\u{0635}\u{0631}",
        "An-Nasr",
        3,
    ),
    (
        111,
        "\u{0627}\u{0644}\u{0645}\u{0633}\u{062F}",
        "Al-Masad",
        5,
    ),
    (
        112,
        "\u{0627}\u{0644}\u{0625}\u{062E}\u{0644}\u{0627}\u{0635}",
        "Al-Ikhlas",
        4,
    ),
    (
        113,
        "\u{0627}\u{0644}\u{0641}\u{0644}\u{0642}",
        "Al-Falaq",
        5,
    ),
    (114, "\u{0627}\u{0644}\u{0646}\u{0627}\u{0633}", "An-Nas", 6),
];

fn surah_name(number: u32) -> Option<(&'static str, &'static str)> {
    SURAHS.iter().find(|s| s.0 == number).map(|s| (s.1, s.2))
}

#[async_trait::async_trait]
impl Skill for QuranSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "quran".into(),
            description: "Quran verse lookup, search, and tafsir using local IslamicDb with API fallback. Supports verse lookup, keyword/thematic search, tafsir, and surah listing.".into(),
            timeout_ms: 15000,
            version: "0.2.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["verse", "search", "surah_list", "tafsir", "thematic"],
                    "description": "Action to perform. verse = lookup by surah:ayat, search = keyword search, surah_list = list all 114 surahs, tafsir = get tafsir for a verse (local DB only), thematic = semantic/thematic search (local DB only). Default: verse"
                },
                "surah": {
                    "type": "integer",
                    "description": "Surah number (1-114) for verse lookup or tafsir"
                },
                "ayat": {
                    "type": "integer",
                    "description": "Ayat (verse) number within the surah"
                },
                "query": {
                    "type": "string",
                    "description": "Search keyword for action=search or action=thematic"
                },
                "language": {
                    "type": "string",
                    "enum": ["ms", "en", "ar"],
                    "description": "Language for results. ms = Malay, en = English, ar = Arabic. Default: ms"
                },
                "tafsir": {
                    "type": "string",
                    "enum": ["ibn_kathir", "jalalayn"],
                    "description": "Tafsir source for action=tafsir. Default: ibn_kathir"
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
            .unwrap_or("verse");

        match action {
            "surah_list" => {
                let lines: Vec<String> = SURAHS
                    .iter()
                    .map(|(num, arabic, translit, verses)| {
                        format!("{num}. {translit} ({arabic}) - {verses} ayat")
                    })
                    .collect();
                SkillResult {
                    success: true,
                    output: format!(
                        "Senarai Surah Al-Quran (114 Surah):\n\n{}",
                        lines.join("\n")
                    ),
                    error: None,
                }
            }
            "search" => self.handle_search(&args).await,
            "tafsir" => self.handle_tafsir(&args).await,
            "thematic" => self.handle_thematic(&args).await,
            _ => self.handle_verse(&args).await,
        }
    }
}

impl QuranSkill {
    /// Handle verse lookup: try local DB first, fall back to API.
    async fn handle_verse(&self, args: &serde_json::Value) -> SkillResult {
        let surah = match args.get("surah").and_then(|v| v.as_u64()) {
            Some(s) if (1..=114).contains(&s) => s as u32,
            Some(s) => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Invalid surah number: {s}. Must be 1-114.")),
                };
            }
            None => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(
                        "Surah number required for verse lookup. Provide 'surah' (1-114) and 'ayat' parameters."
                            .into(),
                    ),
                };
            }
        };

        let ayat = match args.get("ayat").and_then(|v| v.as_u64()) {
            Some(a) if a >= 1 => a as u32,
            Some(_) => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some("Ayat number must be at least 1.".into()),
                };
            }
            None => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some("Ayat number required. Provide 'ayat' parameter.".into()),
                };
            }
        };

        let language = args
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("ms");
        let (arabic_name, translit) = surah_name(surah).unwrap_or(("", "Unknown"));

        // Try local DB first
        if let Some(db) = &self.db {
            match amanclaw_islamic_db::quran::get_verse(db.pool(), surah as i64, ayat as i64).await
            {
                Ok(Some(v)) => {
                    let translation = match language {
                        "en" => &v.translation_en,
                        "ar" => &v.text_uthmani,
                        _ => &v.translation_ms,
                    };
                    let output = format!(
                        "Surah {} ({}) - Ayat {}\n\n{}:{}\n{}\n\n[{}]\n{}\n",
                        translit,
                        arabic_name,
                        ayat,
                        v.surah,
                        v.ayat,
                        v.text_uthmani,
                        lang_label(language),
                        translation
                    );
                    tracing::debug!("Verse {}:{} served from local DB", surah, ayat);
                    return SkillResult {
                        success: true,
                        output,
                        error: None,
                    };
                }
                Ok(None) => {
                    tracing::debug!(
                        "Verse {}:{} not in local DB, falling back to API",
                        surah,
                        ayat
                    );
                }
                Err(e) => {
                    tracing::warn!("Local DB error for verse {}:{}: {}", surah, ayat, e);
                }
            }
        }

        // Fall back to Quran.com API
        match api::get_verse(surah, ayat).await {
            Ok(verse) => {
                let mut output = format!(
                    "Surah {} ({}) - Ayat {}\n\n{}\n{}\n",
                    translit, arabic_name, ayat, verse.verse_key, verse.text_uthmani
                );
                if let Some(translations) = &verse.translations {
                    for tr in translations {
                        output.push_str(&format!(
                            "\n[{}]\n{}\n",
                            tr.resource_name,
                            strip_html(&tr.text)
                        ));
                    }
                }
                SkillResult {
                    success: true,
                    output,
                    error: None,
                }
            }
            Err(e) => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to fetch verse: {e}")),
            },
        }
    }

    /// Handle search: try local DB first, fall back to API.
    async fn handle_search(&self, args: &serde_json::Value) -> SkillResult {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) if !q.is_empty() => q,
            _ => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some("Search query is required. Provide 'query' parameter.".into()),
                };
            }
        };
        let language = args
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("ms");

        // Try local DB first
        if let Some(db) = &self.db {
            match amanclaw_islamic_db::quran::search(db.pool(), query, 10).await {
                Ok(results) if !results.is_empty() => {
                    let mut output =
                        format!("Hasil carian '{}' ({} hasil):\n", query, results.len());
                    for v in &results {
                        let name = surah_name(v.surah as u32)
                            .map(|(_, t)| t)
                            .unwrap_or("Unknown");
                        let translation = match language {
                            "en" => &v.translation_en,
                            "ar" => &v.text_uthmani,
                            _ => &v.translation_ms,
                        };
                        output.push_str(&format!(
                            "\n[{}:{} - {}]\n{}\n({}) {}\n",
                            v.surah,
                            v.ayat,
                            name,
                            v.text_uthmani,
                            lang_label(language),
                            translation
                        ));
                    }
                    tracing::debug!("Search '{}' served from local DB", query);
                    return SkillResult {
                        success: true,
                        output,
                        error: None,
                    };
                }
                Ok(_) => {
                    tracing::debug!(
                        "Search '{}' returned no results from local DB, falling back to API",
                        query
                    );
                }
                Err(e) => {
                    tracing::warn!("Local DB search error for '{}': {}", query, e);
                }
            }
        }

        // Fall back to Quran.com API
        match api::search(query, language).await {
            Ok(result) => {
                if result.results.is_empty() {
                    return SkillResult {
                        success: true,
                        output: format!("Tiada hasil ditemui untuk '{query}'."),
                        error: None,
                    };
                }
                let mut output = format!(
                    "Hasil carian '{}' ({} hasil):\n",
                    result.query, result.total_results
                );
                for hit in &result.results {
                    let surah_num: u32 = hit
                        .verse_key
                        .split(':')
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    let name = surah_name(surah_num).map(|(_, t)| t).unwrap_or("Unknown");
                    output.push_str(&format!(
                        "\n[{} - {}]\n{}\n",
                        hit.verse_key, name, hit.text
                    ));
                    for tr in &hit.translations {
                        output.push_str(&format!(
                            "({}) {}\n",
                            tr.resource_name,
                            strip_html(&tr.text)
                        ));
                    }
                }
                SkillResult {
                    success: true,
                    output,
                    error: None,
                }
            }
            Err(e) => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("Search error: {e}")),
            },
        }
    }

    /// Handle tafsir lookup (local DB only).
    async fn handle_tafsir(&self, args: &serde_json::Value) -> SkillResult {
        let db = match &self.db {
            Some(db) => db,
            None => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(
                        "Tafsir requires local IslamicDb. No database configured.".into(),
                    ),
                };
            }
        };

        let surah = match args.get("surah").and_then(|v| v.as_u64()) {
            Some(s) if (1..=114).contains(&s) => s as i64,
            Some(s) => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Invalid surah number: {s}. Must be 1-114.")),
                };
            }
            None => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(
                        "Surah number required for tafsir. Provide 'surah' (1-114) and 'ayat' parameters."
                            .into(),
                    ),
                };
            }
        };

        let ayat = match args.get("ayat").and_then(|v| v.as_u64()) {
            Some(a) if a >= 1 => a as i64,
            Some(_) => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some("Ayat number must be at least 1.".into()),
                };
            }
            None => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some("Ayat number required. Provide 'ayat' parameter.".into()),
                };
            }
        };

        let tafsir_name = args
            .get("tafsir")
            .and_then(|v| v.as_str())
            .unwrap_or("ibn_kathir");

        let (arabic_name, translit) =
            surah_name(surah as u32).unwrap_or(("", "Unknown"));

        match amanclaw_islamic_db::quran::get_tafsir(db.pool(), surah, ayat, tafsir_name).await {
            Ok(entries) if !entries.is_empty() => {
                let mut output = format!(
                    "Tafsir {} - Surah {} ({}) Ayat {}\n\n",
                    tafsir_name, translit, arabic_name, ayat
                );
                for entry in &entries {
                    output.push_str(&format!("[{} - {}]\n{}\n\n", entry.tafsir_name, entry.language, entry.text));
                }
                SkillResult {
                    success: true,
                    output,
                    error: None,
                }
            }
            Ok(_) => SkillResult {
                success: true,
                output: format!(
                    "Tiada tafsir '{}' ditemui untuk Surah {} Ayat {}.",
                    tafsir_name, translit, ayat
                ),
                error: None,
            },
            Err(e) => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("Tafsir lookup error: {e}")),
            },
        }
    }

    /// Handle thematic/semantic search (local DB only, uses FTS).
    async fn handle_thematic(&self, args: &serde_json::Value) -> SkillResult {
        let db = match &self.db {
            Some(db) => db,
            None => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(
                        "Thematic search requires local IslamicDb. No database configured.".into(),
                    ),
                };
            }
        };

        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) if !q.is_empty() => q,
            _ => {
                return SkillResult {
                    success: false,
                    output: String::new(),
                    error: Some(
                        "Search query is required for thematic search. Provide 'query' parameter."
                            .into(),
                    ),
                };
            }
        };

        let language = args
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("ms");

        match amanclaw_islamic_db::quran::search(db.pool(), query, 15).await {
            Ok(results) if !results.is_empty() => {
                let mut output = format!(
                    "Carian tematik '{}' ({} hasil):\n",
                    query,
                    results.len()
                );
                for v in &results {
                    let name = surah_name(v.surah as u32)
                        .map(|(_, t)| t)
                        .unwrap_or("Unknown");
                    let translation = match language {
                        "en" => &v.translation_en,
                        "ar" => &v.text_uthmani,
                        _ => &v.translation_ms,
                    };
                    output.push_str(&format!(
                        "\n[{}:{} - {}]\n{}\n({}) {}\n",
                        v.surah,
                        v.ayat,
                        name,
                        v.text_uthmani,
                        lang_label(language),
                        translation
                    ));
                }
                SkillResult {
                    success: true,
                    output,
                    error: None,
                }
            }
            Ok(_) => SkillResult {
                success: true,
                output: format!("Tiada hasil ditemui untuk carian tematik '{query}'."),
                error: None,
            },
            Err(e) => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("Thematic search error: {e}")),
            },
        }
    }
}

/// Map language code to a display label.
fn lang_label(lang: &str) -> &'static str {
    match lang {
        "en" => "English",
        "ar" => "Arabic",
        _ => "Bahasa Melayu",
    }
}

/// Strip basic HTML tags from translation text.
fn strip_html(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut inside_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => result.push(ch),
            _ => {}
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_creates_without_db() {
        let skill = QuranSkill::default();
        assert!(skill.db.is_none());
    }

    #[test]
    fn test_metadata() {
        let skill = QuranSkill::default();
        let meta = skill.metadata();
        assert_eq!(meta.name, "quran");
        assert_eq!(meta.version, "0.2.0");
    }

    #[test]
    fn test_parameters_schema_includes_new_actions() {
        let skill = QuranSkill::default();
        let schema = skill.parameters_schema();
        let actions = &schema["properties"]["action"]["enum"];
        let action_list: Vec<&str> = actions
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(action_list.contains(&"verse"));
        assert!(action_list.contains(&"search"));
        assert!(action_list.contains(&"surah_list"));
        assert!(action_list.contains(&"tafsir"));
        assert!(action_list.contains(&"thematic"));
    }

    #[test]
    fn test_parameters_schema_has_tafsir_param() {
        let skill = QuranSkill::default();
        let schema = skill.parameters_schema();
        assert!(schema["properties"]["tafsir"].is_object());
        let tafsir_enum = &schema["properties"]["tafsir"]["enum"];
        let values: Vec<&str> = tafsir_enum
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(values.contains(&"ibn_kathir"));
        assert!(values.contains(&"jalalayn"));
    }

    #[test]
    fn test_parameters_schema_has_language_ar() {
        let skill = QuranSkill::default();
        let schema = skill.parameters_schema();
        let lang_enum = &schema["properties"]["language"]["enum"];
        let values: Vec<&str> = lang_enum
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(values.contains(&"ms"));
        assert!(values.contains(&"en"));
        assert!(values.contains(&"ar"));
    }

    #[tokio::test]
    async fn test_new_accepts_islamic_db() {
        let db = IslamicDb::new(":memory:").await.unwrap();
        let skill = QuranSkill::new(Arc::new(db));
        assert!(skill.db.is_some());
        let meta = skill.metadata();
        assert_eq!(meta.name, "quran");
    }

    #[test]
    fn test_surah_list_complete() {
        assert_eq!(SURAHS.len(), 114);
        assert_eq!(SURAHS[0].0, 1);
        assert_eq!(SURAHS[0].2, "Al-Fatihah");
        assert_eq!(SURAHS[113].0, 114);
        assert_eq!(SURAHS[113].2, "An-Nas");
    }

    #[test]
    fn test_surah_name_lookup() {
        let (arabic, translit) = surah_name(1).unwrap();
        assert_eq!(translit, "Al-Fatihah");
        assert!(!arabic.is_empty());

        let (_, translit) = surah_name(36).unwrap();
        assert_eq!(translit, "Ya-Sin");

        assert!(surah_name(0).is_none());
        assert!(surah_name(115).is_none());
    }

    #[test]
    fn test_strip_html() {
        assert_eq!(strip_html("Hello <b>world</b>"), "Hello world");
        assert_eq!(strip_html("No tags here"), "No tags here");
        assert_eq!(strip_html("<p>Paragraph</p>"), "Paragraph");
        assert_eq!(strip_html(""), "");
    }

    #[test]
    fn test_lang_label() {
        assert_eq!(lang_label("en"), "English");
        assert_eq!(lang_label("ar"), "Arabic");
        assert_eq!(lang_label("ms"), "Bahasa Melayu");
        assert_eq!(lang_label("other"), "Bahasa Melayu");
    }

    #[tokio::test]
    async fn test_missing_surah() {
        let skill = QuranSkill::default();
        let input = SkillInput {
            name: "quran".into(),
            args: r#"{"action": "verse"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Surah number required"));
    }

    #[tokio::test]
    async fn test_missing_ayat() {
        let skill = QuranSkill::default();
        let input = SkillInput {
            name: "quran".into(),
            args: r#"{"action": "verse", "surah": 1}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Ayat number required"));
    }

    #[tokio::test]
    async fn test_invalid_surah_number() {
        let skill = QuranSkill::default();
        let input = SkillInput {
            name: "quran".into(),
            args: r#"{"action": "verse", "surah": 200, "ayat": 1}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid surah number"));
    }

    #[tokio::test]
    async fn test_search_missing_query() {
        let skill = QuranSkill::default();
        let input = SkillInput {
            name: "quran".into(),
            args: r#"{"action": "search"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Search query is required"));
    }

    #[tokio::test]
    async fn test_surah_list_action() {
        let skill = QuranSkill::default();
        let input = SkillInput {
            name: "quran".into(),
            args: r#"{"action": "surah_list"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("Al-Fatihah"));
        assert!(result.output.contains("An-Nas"));
        assert!(result.output.contains("114"));
    }

    #[tokio::test]
    async fn test_invalid_args() {
        let skill = QuranSkill::default();
        let input = SkillInput {
            name: "quran".into(),
            args: "not json".into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid args"));
    }

    #[tokio::test]
    async fn test_tafsir_without_db() {
        let skill = QuranSkill::default();
        let input = SkillInput {
            name: "quran".into(),
            args: r#"{"action": "tafsir", "surah": 1, "ayat": 1}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("requires local IslamicDb"));
    }

    #[tokio::test]
    async fn test_thematic_without_db() {
        let skill = QuranSkill::default();
        let input = SkillInput {
            name: "quran".into(),
            args: r#"{"action": "thematic", "query": "mercy"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("requires local IslamicDb"));
    }

    #[tokio::test]
    async fn test_thematic_missing_query() {
        let db = IslamicDb::new(":memory:").await.unwrap();
        let skill = QuranSkill::new(Arc::new(db));
        let input = SkillInput {
            name: "quran".into(),
            args: r#"{"action": "thematic"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("query is required"));
    }

    #[tokio::test]
    async fn test_verse_from_local_db() {
        let db = IslamicDb::new(":memory:").await.unwrap();
        sqlx::query(
            "INSERT INTO quran_ayat (surah, ayat, text_uthmani, text_simple, translation_ms, translation_en, juz, hizb, page) VALUES (1, 1, 'بِسْمِ ٱللَّهِ ٱلرَّحْمَـٰنِ ٱلرَّحِيمِ', 'bismillah', 'Dengan nama Allah', 'In the name of Allah', 1, 1, 1)"
        )
        .execute(db.pool())
        .await
        .unwrap();

        let skill = QuranSkill::new(Arc::new(db));
        let input = SkillInput {
            name: "quran".into(),
            args: r#"{"action": "verse", "surah": 1, "ayat": 1}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("Al-Fatihah"));
        assert!(result.output.contains("Dengan nama Allah"));
    }

    #[tokio::test]
    async fn test_verse_from_local_db_english() {
        let db = IslamicDb::new(":memory:").await.unwrap();
        sqlx::query(
            "INSERT INTO quran_ayat (surah, ayat, text_uthmani, text_simple, translation_ms, translation_en, juz, hizb, page) VALUES (1, 1, 'بِسْمِ ٱللَّهِ', 'bismillah', 'Dengan nama Allah', 'In the name of Allah', 1, 1, 1)"
        )
        .execute(db.pool())
        .await
        .unwrap();

        let skill = QuranSkill::new(Arc::new(db));
        let input = SkillInput {
            name: "quran".into(),
            args: r#"{"action": "verse", "surah": 1, "ayat": 1, "language": "en"}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("In the name of Allah"));
        assert!(result.output.contains("English"));
    }

    #[tokio::test]
    async fn test_tafsir_from_local_db() {
        let db = IslamicDb::new(":memory:").await.unwrap();
        sqlx::query(
            "INSERT INTO quran_tafsir (surah, ayat, tafsir_name, language, text) VALUES (1, 1, 'ibn_kathir', 'en', 'The Basmalah is the opening verse.')"
        )
        .execute(db.pool())
        .await
        .unwrap();

        let skill = QuranSkill::new(Arc::new(db));
        let input = SkillInput {
            name: "quran".into(),
            args: r#"{"action": "tafsir", "surah": 1, "ayat": 1}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(result.success);
        assert!(result.output.contains("ibn_kathir"));
        assert!(result.output.contains("Basmalah"));
    }

    #[tokio::test]
    async fn test_tafsir_missing_surah() {
        let db = IslamicDb::new(":memory:").await.unwrap();
        let skill = QuranSkill::new(Arc::new(db));
        let input = SkillInput {
            name: "quran".into(),
            args: r#"{"action": "tafsir", "ayat": 1}"#.into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = skill.execute(input).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Surah number required"));
    }
}
