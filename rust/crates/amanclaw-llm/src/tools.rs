use regex::Regex;
use std::sync::LazyLock;

static THINK_TAGGED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?si)<(?:think|thinking)>.*?</(?:think|thinking)>\s*").unwrap());
static THINK_BEFORE_CLOSE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?si)^.*?</(?:think|thinking)>\s*").unwrap());
static THINK_UNCLOSED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?si)<(?:think|thinking)>.*").unwrap());

pub fn strip_thinking(text: &str) -> String {
    let text = THINK_TAGGED.replace_all(text, "");
    let text = THINK_BEFORE_CLOSE.replace_all(&text, "");
    let text = THINK_UNCLOSED.replace_all(&text, "");
    text.trim().to_string()
}
