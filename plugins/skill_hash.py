#!/usr/bin/env python3
"""Hash text with various algorithms."""
import hashlib
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="hash_tool",
    description="Hash text using MD5, SHA1, SHA256, SHA512.",
    parameters={
        "type": "object",
        "properties": {
            "text": {"type": "string", "description": "Text to hash"},
            "algorithm": {"type": "string", "enum": ["md5", "sha1", "sha256", "sha512"], "description": "Hash algorithm (default: sha256)"}
        },
        "required": ["text"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    text = args.get("text", "")
    algo = args.get("algorithm", "sha256")

    try:
        h = hashlib.new(algo, text.encode())
        return SkillResult.ok(f"{algo}: {h.hexdigest()}")
    except Exception as e:
        return SkillResult.err(f"Hash error: {e}")

if __name__ == "__main__":
    execute.run()
