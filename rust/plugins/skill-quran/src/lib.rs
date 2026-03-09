mod api;

use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};

pub struct QuranSkill;

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
            description: "Quran verse lookup and search using Quran.com API. Supports verse lookup by surah:ayat, keyword search, and surah listing.".into(),
            timeout_ms: 15000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["verse", "search", "surah_list"],
                    "description": "Action to perform. verse = lookup by surah:ayat, search = keyword search, surah_list = list all 114 surahs. Default: verse"
                },
                "surah": {
                    "type": "integer",
                    "description": "Surah number (1-114) for verse lookup"
                },
                "ayat": {
                    "type": "integer",
                    "description": "Ayat (verse) number within the surah"
                },
                "query": {
                    "type": "string",
                    "description": "Search keyword for action=search"
                },
                "language": {
                    "type": "string",
                    "enum": ["ms", "en"],
                    "description": "Language for search results. ms = Malay, en = English. Default: ms"
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
            "search" => {
                let query = match args.get("query").and_then(|v| v.as_str()) {
                    Some(q) if !q.is_empty() => q,
                    _ => {
                        return SkillResult {
                            success: false,
                            output: String::new(),
                            error: Some(
                                "Search query is required. Provide 'query' parameter.".into(),
                            ),
                        };
                    }
                };
                let language = args
                    .get("language")
                    .and_then(|v| v.as_str())
                    .unwrap_or("ms");

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
            _ => {
                // verse lookup
                let surah = match args.get("surah").and_then(|v| v.as_u64()) {
                    Some(s) if (1..=114).contains(&s) => s as u32,
                    Some(s) => return SkillResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Invalid surah number: {s}. Must be 1-114.")),
                    },
                    None => return SkillResult {
                        success: false,
                        output: String::new(),
                        error: Some("Surah number required for verse lookup. Provide 'surah' (1-114) and 'ayat' parameters.".into()),
                    },
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

                let (arabic_name, translit) = surah_name(surah).unwrap_or(("", "Unknown"));

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
        }
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
    fn test_metadata() {
        let skill = QuranSkill;
        let meta = skill.metadata();
        assert_eq!(meta.name, "quran");
        assert_eq!(meta.version, "0.1.0");
    }

    #[test]
    fn test_parameters_schema() {
        let skill = QuranSkill;
        let schema = skill.parameters_schema();
        assert!(schema["properties"]["action"].is_object());
        assert!(schema["properties"]["surah"].is_object());
        assert!(schema["properties"]["ayat"].is_object());
        assert!(schema["properties"]["query"].is_object());
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

    #[tokio::test]
    async fn test_missing_surah() {
        let skill = QuranSkill;
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
        let skill = QuranSkill;
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
        let skill = QuranSkill;
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
        let skill = QuranSkill;
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
        let skill = QuranSkill;
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
        let skill = QuranSkill;
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
}
