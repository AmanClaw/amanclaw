#!/usr/bin/env python3
"""Web search using DuckDuckGo (no API key required)."""
import urllib.request
import urllib.parse
import json
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="web_search",
    description="Search the web using DuckDuckGo. Returns top results with titles and URLs.",
    parameters={
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "Search query"},
            "max_results": {"type": "integer", "description": "Max results (default 5)"}
        },
        "required": ["query"]
    },
    timeout_ms=15000
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    query = args.get("query", "")
    max_results = args.get("max_results", 5)

    if not query:
        return SkillResult.err("Please provide a search query.")

    try:
        url = f"https://api.duckduckgo.com/?q={urllib.parse.quote(query)}&format=json&no_html=1"
        req = urllib.request.Request(url, headers={"User-Agent": "AmanClaw/1.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode())

        results = []
        # Abstract (instant answer)
        if data.get("Abstract"):
            results.append(f"**Summary:** {data['Abstract']}\nSource: {data.get('AbstractURL', '')}")

        # Related topics
        for topic in data.get("RelatedTopics", [])[:max_results]:
            if "Text" in topic:
                text = topic["Text"]
                url = topic.get("FirstURL", "")
                results.append(f"- {text}\n  {url}")

        if not results:
            return SkillResult.ok(f"No results found for '{query}'. Try a different search term.")

        return SkillResult.ok("\n\n".join(results))
    except Exception as e:
        return SkillResult.err(f"Search failed: {e}")

if __name__ == "__main__":
    execute.run()
