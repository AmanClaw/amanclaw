"""AmanClaw Plugin SDK for Python.

Write plugins as Python scripts that communicate with the AmanClaw engine
via a simple JSON protocol over stdin/stdout.

Usage:
    from amanclaw_sdk import plugin, SkillMetadata, SkillInput, SkillResult

    @plugin(
        name="my_skill",
        description="Does something useful",
        version="0.1.0",
        parameters={
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search query"}
            },
            "required": ["query"]
        }
    )
    def execute(input: SkillInput) -> SkillResult:
        args = input.parse_args()
        query = args.get("query", "none")
        return SkillResult.ok(f"Result for: {query}")

    if __name__ == "__main__":
        execute.run()
"""

from amanclaw_sdk.types import SkillMetadata, SkillInput, SkillResult
from amanclaw_sdk.runner import plugin

__all__ = ["SkillMetadata", "SkillInput", "SkillResult", "plugin"]
__version__ = "0.1.0"
