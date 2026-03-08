use serde::Deserialize;

const QURAN_API: &str = "https://api.quran.com/api/v4";

#[derive(Debug, Deserialize)]
pub struct Verse {
    #[allow(dead_code)]
    pub id: u32,
    pub verse_key: String,
    pub text_uthmani: String,
    pub translations: Option<Vec<Translation>>,
}

#[derive(Debug, Deserialize)]
pub struct Translation {
    pub text: String,
    pub resource_name: String,
    #[allow(dead_code)]
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

/// Fetch a specific verse with Malay (39) and English (131) translations.
pub async fn get_verse(surah: u32, ayat: u32) -> Result<Verse, String> {
    let url = format!(
        "{}/verses/by_key/{}:{}?language=ms&translations=39,131&fields=text_uthmani",
        QURAN_API, surah, ayat
    );
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let verse = data.get("verse").ok_or("No verse found".to_string())?;
    serde_json::from_value(verse.clone()).map_err(|e| format!("Deserialize error: {}", e))
}

/// Search Quran by keyword.
pub async fn search(query: &str, language: &str) -> Result<SearchResult, String> {
    let url = format!(
        "{}/search?q={}&size=5&language={}",
        QURAN_API,
        urlencoding::encode(query),
        language
    );
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let data: SearchResponse = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(data.search)
}
