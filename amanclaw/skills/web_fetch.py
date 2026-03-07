"""
Web fetch skill — fetch web pages and return readable content.
Supports HTML (extracts text), JSON, and plain text.
"""

import logging
import re
from html.parser import HTMLParser
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.web_fetch")

# Max content size to return (characters)
MAX_CONTENT_SIZE = 4000
# Request timeout
REQUEST_TIMEOUT = 15


class _TextExtractor(HTMLParser):
    """Simple HTML to text converter."""

    SKIP_TAGS = {"script", "style", "noscript", "svg", "head"}

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
        # Collapse whitespace
        text = re.sub(r"[ \t]+", " ", text)
        text = re.sub(r"\n{3,}", "\n\n", text)
        return text.strip()


def _extract_text(html: str) -> str:
    """Extract readable text from HTML."""
    parser = _TextExtractor()
    parser.feed(html)
    return parser.get_text()


@skill(
    name="web_fetch",
    description="Fetch a web page or API endpoint and return its content. Returns readable text for HTML pages, or raw content for JSON/text. Useful for reading articles, documentation, API responses, checking websites, etc.",
    parameters={
        "url": {
            "type": "string",
            "description": "The URL to fetch (must start with http:// or https://)",
        },
        "raw": {
            "type": "boolean",
            "description": "If true, return raw HTML/content without text extraction. Default: false",
            "optional": True,
        },
    },
    timeout=20,
)
def web_fetch(url: str, raw: bool = False) -> str:
    """Fetch a URL and return its content."""
    import requests

    # Validate URL
    if not url.startswith(("http://", "https://")):
        return "Error: URL must start with http:// or https://"

    # Block internal/private IPs (SSRF protection)
    from urllib.parse import urlparse
    hostname = urlparse(url).hostname or ""
    blocked = ("localhost", "127.0.0.1", "0.0.0.0", "169.254.", "10.", "192.168.", "172.16.")
    if any(hostname.startswith(b) or hostname == b for b in blocked):
        return "Error: Cannot fetch internal/private URLs."

    logger.info(f"Fetching: {url}")

    try:
        resp = requests.get(
            url,
            timeout=REQUEST_TIMEOUT,
            headers={"User-Agent": "AmanClaw-Bot/1.0"},
            allow_redirects=True,
        )
        resp.raise_for_status()
    except requests.exceptions.Timeout:
        return "Error: Request timed out."
    except requests.exceptions.ConnectionError:
        return f"Error: Could not connect to {hostname}."
    except requests.exceptions.HTTPError as e:
        return f"Error: HTTP {resp.status_code} — {e}"
    except Exception as e:
        return f"Error: {e}"

    content_type = resp.headers.get("content-type", "")

    # JSON — return formatted
    if "json" in content_type:
        try:
            import json
            data = resp.json()
            return json.dumps(data, indent=2, ensure_ascii=False)[:MAX_CONTENT_SIZE]
        except Exception:
            pass

    content = resp.text

    # HTML — extract text unless raw requested
    if "html" in content_type and not raw:
        content = _extract_text(content)

    if len(content) > MAX_CONTENT_SIZE:
        content = content[:MAX_CONTENT_SIZE] + f"\n\n[... truncated, {len(resp.text)} chars total]"

    return content or "(empty response)"
