"""Core types for AmanClaw plugins."""

from __future__ import annotations
import json
from dataclasses import dataclass, field
from typing import Any, Optional


@dataclass
class SkillMetadata:
    """Metadata describing a skill plugin."""
    name: str
    description: str
    version: str = "0.1.0"
    timeout_ms: int = 10000

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "description": self.description,
            "timeout_ms": self.timeout_ms,
            "version": self.version,
        }


@dataclass
class SkillInput:
    """Input passed to a skill's execute function."""
    name: str
    args: str
    user_id: str
    platform: str

    def parse_args(self) -> dict[str, Any]:
        """Parse the JSON args string into a dict."""
        try:
            return json.loads(self.args)
        except json.JSONDecodeError:
            return {}

    @classmethod
    def from_dict(cls, data: dict) -> SkillInput:
        return cls(
            name=data.get("name", ""),
            args=data.get("args", "{}"),
            user_id=data.get("user_id", ""),
            platform=data.get("platform", ""),
        )


@dataclass
class SkillResult:
    """Result returned from a skill's execute function."""
    success: bool
    output: str
    error: Optional[str] = None

    @classmethod
    def ok(cls, output: str) -> SkillResult:
        return cls(success=True, output=output)

    @classmethod
    def err(cls, error: str) -> SkillResult:
        return cls(success=False, output="", error=error)

    def to_dict(self) -> dict:
        return {
            "success": self.success,
            "output": self.output,
            "error": self.error,
        }
