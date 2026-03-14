#!/usr/bin/env python3
"""JSON utilities: format, validate, query, minify."""
import json
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="json_tool",
    description="JSON utilities: format/prettify, validate, minify, extract fields.",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["format", "validate", "minify", "extract"], "description": "Operation"},
            "data": {"type": "string", "description": "JSON string to process"},
            "path": {"type": "string", "description": "Dot-notation path for extract (e.g., 'user.name')"}
        },
        "required": ["action", "data"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "format")
    data = args.get("data", "")

    try:
        if action == "validate":
            json.loads(data)
            return SkillResult.ok("Valid JSON.")
        elif action == "format":
            parsed = json.loads(data)
            return SkillResult.ok(json.dumps(parsed, indent=2))
        elif action == "minify":
            parsed = json.loads(data)
            return SkillResult.ok(json.dumps(parsed, separators=(",", ":")))
        elif action == "extract":
            parsed = json.loads(data)
            path = args.get("path", "")
            for key in path.split("."):
                if isinstance(parsed, dict):
                    parsed = parsed.get(key)
                elif isinstance(parsed, list) and key.isdigit():
                    parsed = parsed[int(key)]
                else:
                    return SkillResult.err(f"Cannot navigate path: {path}")
            return SkillResult.ok(json.dumps(parsed, indent=2) if parsed is not None else "null")
        return SkillResult.err(f"Unknown action: {action}")
    except json.JSONDecodeError as e:
        return SkillResult.err(f"Invalid JSON: {e}")
    except Exception as e:
        return SkillResult.err(f"JSON error: {e}")

if __name__ == "__main__":
    execute.run()
