#!/usr/bin/env python3
"""Translation helper — structures translation requests for the LLM."""
from amanclaw_sdk import plugin, SkillInput, SkillResult

SUPPORTED_LANGS = [
    "en", "ms", "ar", "id", "tr", "ur", "zh", "ja", "ko",
    "fr", "de", "es", "pt", "ru", "hi", "bn", "ta"
]

@plugin(
    name="translate",
    description="Translate text between languages. Supports: English, Malay, Arabic, Indonesian, Turkish, Urdu, Chinese, Japanese, Korean, French, German, Spanish, Portuguese, Russian, Hindi, Bengali, Tamil.",
    parameters={
        "type": "object",
        "properties": {
            "text": {"type": "string", "description": "Text to translate"},
            "target_lang": {"type": "string", "description": "Target language code (e.g., 'ms', 'ar', 'en')"},
            "source_lang": {"type": "string", "description": "Source language (auto-detect if omitted)"}
        },
        "required": ["text", "target_lang"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    text = args.get("text", "")
    target = args.get("target_lang", "")
    source = args.get("source_lang", "auto")

    if not text:
        return SkillResult.err("Please provide text to translate.")
    if not target:
        return SkillResult.err(f"Please specify target language. Supported: {', '.join(SUPPORTED_LANGS)}")

    # The LLM handles the actual translation
    return SkillResult.ok(
        f"[Translation request: {source} → {target}]\n\n{text}"
    )

if __name__ == "__main__":
    execute.run()
