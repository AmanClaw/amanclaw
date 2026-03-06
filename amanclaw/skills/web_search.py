"""
Web search skill — search the web using Brave Search API.
"""

import os
import logging
import requests
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.web_search")

BRAVE_SEARCH_URL = "https://api.search.brave.com/res/v1/web/search"


@skill(
    name="web_search",
    description="Search the web using Brave Search. Returns titles, URLs, and descriptions for the top results. Useful for answering questions about current events, looking up facts, or finding information online.",
    parameters={
        "query": {
            "type": "string",
            "description": "The search query (e.g., 'weather in Tokyo', 'Python 3.12 release date')",
        },
        "count": {
            "type": "integer",
            "description": "Number of results to return (1-10, default 5)",
            "optional": True,
        },
    },
    timeout=15,
)
def web_search(query: str, count: int = 5) -> str:
    """Search the web via Brave Search API."""

    api_key = os.environ.get("BRAVE_API_KEY")
    if not api_key:
        return (
            "Error: BRAVE_API_KEY environment variable is not set. "
            "Get a free API key at https://api.search.brave.com/ and add it to your .env file."
        )

    # Clamp count to valid range
    count = max(1, min(10, count))

    headers = {
        "Accept": "application/json",
        "Accept-Encoding": "gzip",
        "X-Subscription-Token": api_key,
    }
    params = {
        "q": query,
        "count": count,
    }

    try:
        response = requests.get(
            BRAVE_SEARCH_URL,
            headers=headers,
            params=params,
            timeout=12,
        )
        response.raise_for_status()
    except requests.exceptions.Timeout:
        return "Error: Search request timed out. Please try again."
    except requests.exceptions.ConnectionError:
        return "Error: Could not connect to Brave Search API. Check your internet connection."
    except requests.exceptions.HTTPError as e:
        if response.status_code == 401:
            return "Error: Invalid BRAVE_API_KEY. Check your API key and try again."
        if response.status_code == 429:
            return "Error: Brave Search rate limit exceeded. Please wait and try again."
        return f"Error: Brave Search API returned HTTP {response.status_code}: {e}"
    except requests.exceptions.RequestException as e:
        return f"Error: Search request failed: {e}"

    data = response.json()
    results = data.get("web", {}).get("results", [])

    if not results:
        return f"No results found for: {query}"

    # Format results
    lines = [f"Search results for: {query}\n"]
    for i, item in enumerate(results, 1):
        title = item.get("title", "No title")
        url = item.get("url", "")
        description = item.get("description", "No description")
        lines.append(f"{i}. {title}")
        lines.append(f"   URL: {url}")
        lines.append(f"   {description}")
        lines.append("")

    return "\n".join(lines).strip()
