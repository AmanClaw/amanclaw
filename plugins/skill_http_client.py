#!/usr/bin/env python3
"""Make HTTP requests (GET, POST, etc.)."""
import urllib.request
import json
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="http_client",
    description="Make HTTP requests. Supports GET, POST, PUT, DELETE with headers and body.",
    parameters={
        "type": "object",
        "properties": {
            "method": {"type": "string", "enum": ["GET", "POST", "PUT", "DELETE"], "description": "HTTP method"},
            "url": {"type": "string", "description": "URL to request"},
            "headers": {"type": "object", "description": "Request headers (key-value pairs)"},
            "body": {"type": "string", "description": "Request body (for POST/PUT)"}
        },
        "required": ["url"]
    },
    timeout_ms=15000
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    url = args.get("url", "")
    method = args.get("method", "GET")
    headers = args.get("headers", {})
    body = args.get("body")

    if not url:
        return SkillResult.err("Please provide a URL.")
    if not url.startswith(("http://", "https://")):
        return SkillResult.err("URL must start with http:// or https://")

    try:
        data = body.encode() if body else None
        req = urllib.request.Request(url, data=data, method=method)
        req.add_header("User-Agent", "AmanClaw/1.0")
        for k, v in headers.items():
            req.add_header(k, v)

        with urllib.request.urlopen(req, timeout=10) as resp:
            status = resp.status
            content = resp.read().decode("utf-8", errors="replace")

        # Try to pretty-print JSON
        try:
            parsed = json.loads(content)
            content = json.dumps(parsed, indent=2)
        except (json.JSONDecodeError, ValueError):
            pass

        if len(content) > 3000:
            content = content[:3000] + "\n[Truncated]"

        return SkillResult.ok(f"HTTP {status}\n\n{content}")
    except Exception as e:
        return SkillResult.err(f"HTTP error: {e}")

if __name__ == "__main__":
    execute.run()
