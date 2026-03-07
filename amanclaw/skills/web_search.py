"""
Web search skill — search the internet using DuckDuckGo.
Automatically fetches top result content for detailed answers.
No API key required.
"""

import logging
import requests
import re
from html.parser import HTMLParser
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.web_search")

MAX_FETCH_SIZE = 3000
FETCH_TIMEOUT = 10


class _TextExtractor(HTMLParser):
    """Simple HTML to text converter."""
    SKIP_TAGS = {"script", "style", "noscript", "svg", "head", "nav", "footer", "header"}

    def __init__(self):
        super().__init__()
        self._text = []
        self._skip_depth = 0

    def handle_starttag(self, tag, attrs):
        if tag in self.SKIP_TAGS:
            self._skip_depth += 1
        if tag in ("br", "p", "div", "h1", "h2", "h3", "h4", "h5", "h6", "li", "tr"):
            self._text.append("\n")

    def handle_endtag(self, tag):
        if tag in self.SKIP_TAGS:
            self._skip_depth = max(0, self._skip_depth - 1)

    def handle_data(self, data):
        if self._skip_depth == 0:
            self._text.append(data)

    def get_text(self):
        text = " ".join(self._text)
        text = re.sub(r"[ \t]+", " ", text)
        text = re.sub(r"\n{3,}", "\n\n", text)
        return text.strip()


def _fetch_page_text(url: str) -> str | None:
    """Fetch a URL and extract readable text."""
    try:
        resp = requests.get(
            url, timeout=FETCH_TIMEOUT,
            headers={"User-Agent": "Mozilla/5.0 (compatible; AmanClaw-Bot/1.0)"},
            allow_redirects=True,
        )
        resp.raise_for_status()
        content_type = resp.headers.get("content-type", "")
        if "html" in content_type:
            parser = _TextExtractor()
            parser.feed(resp.text)
            text = parser.get_text()
            return text[:MAX_FETCH_SIZE] if text else None
        elif "json" in content_type:
            return resp.text[:MAX_FETCH_SIZE]
        else:
            return resp.text[:MAX_FETCH_SIZE]
    except Exception:
        return None


@skill(
    name="web_search",
    description="Search the internet for real-time, up-to-date information. Returns search results AND automatically reads the top result for detailed content. Use this for current events, news, prices, weather, sports, facts, or anything that needs recent data.",
    parameters={
        "query": {
            "type": "string",
            "description": "The search query (e.g., 'weather in Kuala Lumpur today', 'latest iPhone price Malaysia')",
        },
        "num_results": {
            "type": "integer",
            "description": "Number of results to return (default: 5, max: 10)",
            "optional": True,
        },
    },
    timeout=30,
)
def web_search(query: str, num_results: int = 5) -> str:
    """Search the web and auto-fetch top results for detailed content."""
    try:
        from duckduckgo_search import DDGS
    except ImportError:
        return "Error: duckduckgo-search not installed."

    num_results = min(max(num_results, 1), 10)
    logger.info(f"Searching: {query}")

    try:
        with DDGS() as ddgs:
            results = list(ddgs.text(query, max_results=num_results))

        if not results:
            return f"No results found for: {query}"

        # Build search results summary
        output = ["## Search Results\n"]
        for i, r in enumerate(results, 1):
            title = r.get("title", "")
            body = r.get("body", "")
            href = r.get("href", "")
            output.append(f"{i}. **{title}**\n   {body}\n   URL: {href}")

        # Auto-fetch top 2 results for detailed content
        output.append("\n\n## Detailed Content from Top Results\n")
        fetched = 0
        for r in results[:3]:
            if fetched >= 2:
                break
            url = r.get("href", "")
            if not url:
                continue
            logger.info(f"Auto-fetching: {url}")
            page_text = _fetch_page_text(url)
            if page_text and len(page_text) > 100:
                title = r.get("title", url)
                output.append(f"### From: {title}\n{page_text}\n")
                fetched += 1

        return "\n\n".join(output)

    except Exception as e:
        logger.error(f"Web search failed: {e}")
        return f"Search failed: {e}"
