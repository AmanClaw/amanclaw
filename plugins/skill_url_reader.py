#!/usr/bin/env python3
"""Fetch and extract text content from a URL."""
import urllib.request
import html
import re
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="url_reader",
    description="Fetch a URL and extract its text content. Useful for reading web pages, APIs, or raw text files.",
    parameters={
        "type": "object",
        "properties": {
            "url": {"type": "string", "description": "URL to fetch"},
            "max_chars": {"type": "integer", "description": "Max characters to return (default 3000)"}
        },
        "required": ["url"]
    },
    timeout_ms=15000
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    url = args.get("url", "")
    max_chars = args.get("max_chars", 3000)

    if not url:
        return SkillResult.err("Please provide a URL.")
    if not url.startswith(("http://", "https://")):
        return SkillResult.err("URL must start with http:// or https://")

    try:
        req = urllib.request.Request(url, headers={"User-Agent": "AmanClaw/1.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            content_type = resp.headers.get("Content-Type", "")
            raw = resp.read().decode("utf-8", errors="replace")

        if "html" in content_type:
            # Strip HTML tags and decode entities
            text = re.sub(r"<script[^>]*>.*?</script>", "", raw, flags=re.DOTALL)
            text = re.sub(r"<style[^>]*>.*?</style>", "", text, flags=re.DOTALL)
            text = re.sub(r"<[^>]+>", " ", text)
            text = html.unescape(text)
            text = re.sub(r"\s+", " ", text).strip()
        else:
            text = raw.strip()

        if len(text) > max_chars:
            text = text[:max_chars] + f"\n\n[Truncated at {max_chars} characters]"

        return SkillResult.ok(f"Content from {url}:\n\n{text}")
    except Exception as e:
        return SkillResult.err(f"Failed to fetch URL: {e}")

if __name__ == "__main__":
    execute.run()
