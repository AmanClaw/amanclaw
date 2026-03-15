#!/usr/bin/env python3
"""Example AmanClaw plugin in Python.

Run directly: python hello_plugin.py
Or register in config.yaml as an MCP-style script plugin.
"""

from amanclaw_sdk import plugin, SkillInput, SkillResult


@plugin(
    name="hello_python",
    description="A greeting skill written in Python",
    parameters={
        "type": "object",
        "properties": {
            "name": {"type": "string", "description": "Name to greet"},
        },
        "required": ["name"],
    },
)
def execute(input: SkillInput) -> SkillResult:
    args = input.parse_args()
    name = args.get("name", "World")
    return SkillResult.ok(f"Hello, {name}! (from Python)")


if __name__ == "__main__":
    execute.run()
