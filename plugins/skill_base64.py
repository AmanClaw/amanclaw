#!/usr/bin/env python3
"""Base64 encoding and decoding."""
import base64
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="base64_tool",
    description="Encode or decode Base64 strings.",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["encode", "decode"], "description": "Operation"},
            "data": {"type": "string", "description": "Data to encode/decode"}
        },
        "required": ["action", "data"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "encode")
    data = args.get("data", "")

    try:
        if action == "encode":
            result = base64.b64encode(data.encode()).decode()
            return SkillResult.ok(result)
        elif action == "decode":
            result = base64.b64decode(data).decode()
            return SkillResult.ok(result)
        return SkillResult.err(f"Unknown action: {action}")
    except Exception as e:
        return SkillResult.err(f"Base64 error: {e}")

if __name__ == "__main__":
    execute.run()
