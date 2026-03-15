"""Plugin runner — communicates with AmanClaw engine via JSON protocol over stdin/stdout.

The protocol is simple JSON-RPC-like:
- Engine sends a JSON line to stdin
- Plugin responds with a JSON line to stdout

Commands:
  {"method": "metadata"}     -> {"name": ..., "description": ..., ...}
  {"method": "parameters"}   -> {"type": "object", "properties": ...}
  {"method": "execute", "input": {...}} -> {"success": true, "output": "...", ...}
  {"method": "shutdown"}     -> plugin exits
"""

from __future__ import annotations

import json
import sys
import functools
from typing import Any, Callable

from amanclaw_sdk.types import SkillInput, SkillMetadata, SkillResult


class PluginRunner:
    """Wraps a skill function and runs the JSON protocol loop."""

    def __init__(
        self,
        execute_fn: Callable[[SkillInput], SkillResult],
        metadata: SkillMetadata,
        parameters: dict[str, Any],
    ):
        self.execute_fn = execute_fn
        self.metadata = metadata
        self.parameters = parameters

    def run(self) -> None:
        """Run the stdin/stdout protocol loop."""
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue

            try:
                request = json.loads(line)
            except json.JSONDecodeError:
                self._respond({"error": "Invalid JSON"})
                continue

            method = request.get("method", "")

            if method == "metadata":
                self._respond(self.metadata.to_dict())
            elif method == "parameters":
                self._respond(self.parameters)
            elif method == "execute":
                input_data = request.get("input", {})
                skill_input = SkillInput.from_dict(input_data)
                try:
                    result = self.execute_fn(skill_input)
                    self._respond(result.to_dict())
                except Exception as e:
                    self._respond(SkillResult.err(str(e)).to_dict())
            elif method == "shutdown":
                break
            else:
                self._respond({"error": f"Unknown method: {method}"})

    @staticmethod
    def _respond(data: dict) -> None:
        """Write a JSON response line to stdout."""
        print(json.dumps(data), flush=True)


def plugin(
    name: str,
    description: str,
    parameters: dict[str, Any],
    version: str = "0.1.0",
    timeout_ms: int = 10000,
) -> Callable:
    """Decorator to register a function as an AmanClaw plugin.

    Usage:
        @plugin(
            name="my_skill",
            description="Does something",
            parameters={"type": "object", "properties": {"q": {"type": "string"}}},
        )
        def execute(input: SkillInput) -> SkillResult:
            return SkillResult.ok("Hello!")

        if __name__ == "__main__":
            execute.run()
    """
    meta = SkillMetadata(
        name=name,
        description=description,
        version=version,
        timeout_ms=timeout_ms,
    )

    def decorator(fn: Callable[[SkillInput], SkillResult]) -> PluginRunner:
        runner = PluginRunner(fn, meta, parameters)

        @functools.wraps(fn)
        def wrapper(*args, **kwargs):
            return fn(*args, **kwargs)

        wrapper.run = runner.run
        wrapper.metadata = meta
        wrapper.parameters = parameters
        return wrapper

    return decorator
