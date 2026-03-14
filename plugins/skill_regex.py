#!/usr/bin/env python3
"""Regex testing and extraction."""
import re
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="regex_tool",
    description="Test regex patterns, find matches, and extract groups from text.",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["test", "find_all", "replace"], "description": "Operation"},
            "pattern": {"type": "string", "description": "Regex pattern"},
            "text": {"type": "string", "description": "Text to search"},
            "replacement": {"type": "string", "description": "Replacement string (for replace action)"}
        },
        "required": ["action", "pattern", "text"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "test")
    pattern = args.get("pattern", "")
    text = args.get("text", "")

    try:
        if action == "test":
            match = re.search(pattern, text)
            if match:
                return SkillResult.ok(f"Match found: '{match.group()}' at position {match.start()}-{match.end()}")
            return SkillResult.ok("No match found.")
        elif action == "find_all":
            matches = re.findall(pattern, text)
            if matches:
                return SkillResult.ok(f"Found {len(matches)} match(es):\n" + "\n".join(f"  - {m}" for m in matches))
            return SkillResult.ok("No matches found.")
        elif action == "replace":
            replacement = args.get("replacement", "")
            result = re.sub(pattern, replacement, text)
            return SkillResult.ok(result)
        return SkillResult.err(f"Unknown action: {action}")
    except re.error as e:
        return SkillResult.err(f"Invalid regex: {e}")
    except Exception as e:
        return SkillResult.err(f"Regex error: {e}")

if __name__ == "__main__":
    execute.run()
