use crate::client::ToolCall;
use regex::Regex;
use std::sync::LazyLock;

static THINK_TAGGED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?si)<(?:think|thinking)>.*?</(?:think|thinking)>\s*").unwrap());
static THINK_BEFORE_CLOSE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?si)^.*?</(?:think|thinking)>\s*").unwrap());
static THINK_UNCLOSED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?si)<(?:think|thinking)>.*").unwrap());

/// Regex to match XML-style tool calls emitted by some LLMs (e.g. Qwen).
/// Matches: <tool_call> ... </tool_call>
static XML_TOOL_CALL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?si)<tool_call>\s*(.*?)\s*</tool_call>").unwrap()
});
static XML_FUNCTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?si)<function=([^>]+)>(.*?)</function>").unwrap()
});
static XML_PARAM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?si)<parameter=([^>]+)>(.*?)</parameter>").unwrap()
});

pub fn strip_thinking(text: &str) -> String {
    let text = THINK_TAGGED.replace_all(text, "");
    let text = THINK_BEFORE_CLOSE.replace_all(&text, "");
    let text = THINK_UNCLOSED.replace_all(&text, "");
    text.trim().to_string()
}

/// Try to parse XML-style tool calls from LLM text content.
/// Returns Some(Vec<ToolCall>) if XML tool calls are found, None otherwise.
pub fn parse_xml_tool_calls(text: &str) -> Option<Vec<ToolCall>> {
    let mut calls = Vec::new();
    let mut call_id = 0;

    // Format 1: <tool_call> containing <function=name>...<parameter=key>value</parameter>...</function>
    for tc_cap in XML_TOOL_CALL.captures_iter(text) {
        let inner = &tc_cap[1];
        for fn_cap in XML_FUNCTION.captures_iter(inner) {
            let name = fn_cap[1].trim().to_string();
            let fn_body = &fn_cap[2];

            let mut params = serde_json::Map::new();
            for param_cap in XML_PARAM.captures_iter(fn_body) {
                let key = param_cap[1].trim().to_string();
                let value = param_cap[2].trim().to_string();
                params.insert(key, serde_json::Value::String(value));
            }

            call_id += 1;
            calls.push(ToolCall {
                id: format!("xml_call_{call_id}"),
                name,
                arguments: serde_json::to_string(&params).unwrap_or_default(),
            });
        }
    }

    // Format 2: bare <function=name> without wrapping <tool_call> (also common)
    if calls.is_empty() {
        for fn_cap in XML_FUNCTION.captures_iter(text) {
            let name = fn_cap[1].trim().to_string();
            let fn_body = &fn_cap[2];

            let mut params = serde_json::Map::new();
            for param_cap in XML_PARAM.captures_iter(fn_body) {
                let key = param_cap[1].trim().to_string();
                let value = param_cap[2].trim().to_string();
                params.insert(key, serde_json::Value::String(value));
            }

            call_id += 1;
            calls.push(ToolCall {
                id: format!("xml_call_{call_id}"),
                name,
                arguments: serde_json::to_string(&params).unwrap_or_default(),
            });
        }
    }

    if calls.is_empty() { None } else { Some(calls) }
}
