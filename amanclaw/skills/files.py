"""
File skill — read, write, and list files in a safe workspace directory.
All operations are confined to the workspace path.
"""

import os
import logging
from pathlib import Path
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.files")

# Workspace directory — all file ops are confined here
WORKSPACE = Path.home() / "amanclaw-workspace"


def configure(workspace_dir: str = None):
    """Set the workspace directory."""
    global WORKSPACE
    if workspace_dir:
        WORKSPACE = Path(workspace_dir).expanduser().resolve()


def _safe_path(filepath: str) -> Path:
    """Resolve a path and ensure it's inside the workspace."""
    resolved = (WORKSPACE / filepath).resolve()
    if not str(resolved).startswith(str(WORKSPACE)):
        raise ValueError(f"Path escapes workspace: {filepath}")
    return resolved


@skill(
    name="read_file",
    description="Read the contents of a file in the workspace. Use this to check file contents, read notes, configs, etc.",
    parameters={
        "path": {
            "type": "string",
            "description": "Relative path within the workspace (e.g., 'notes/todo.txt')",
        },
    },
    timeout=10,
)
def read_file(path: str) -> str:
    """Read a file from the workspace."""
    try:
        safe = _safe_path(path)
        if not safe.exists():
            return f"File not found: {path}"
        if not safe.is_file():
            return f"Not a file: {path}"
        if safe.stat().st_size > 100_000:
            return f"File too large ({safe.stat().st_size} bytes). Max 100KB."

        content = safe.read_text(encoding="utf-8", errors="replace")
        return content[:5000]  # Cap output
    except ValueError as e:
        return str(e)
    except Exception as e:
        return f"Error reading file: {e}"


@skill(
    name="write_file",
    description="Write content to a file in the workspace. Creates parent directories if needed. Use for saving notes, creating scripts, etc.",
    parameters={
        "path": {
            "type": "string",
            "description": "Relative path within the workspace (e.g., 'notes/meeting.md')",
        },
        "content": {
            "type": "string",
            "description": "The content to write to the file",
        },
    },
    timeout=10,
)
def write_file(path: str, content: str) -> str:
    """Write content to a file in the workspace."""
    try:
        safe = _safe_path(path)
        safe.parent.mkdir(parents=True, exist_ok=True)
        safe.write_text(content, encoding="utf-8")
        return f"Written {len(content)} characters to {path}"
    except ValueError as e:
        return str(e)
    except Exception as e:
        return f"Error writing file: {e}"


@skill(
    name="list_files",
    description="List files and directories in the workspace. Optionally specify a subdirectory.",
    parameters={
        "path": {
            "type": "string",
            "description": "Relative subdirectory path (default: root of workspace)",
            "optional": True,
        },
    },
    timeout=10,
)
def list_files(path: str = ".") -> str:
    """List files in the workspace directory."""
    try:
        safe = _safe_path(path)
        if not safe.exists():
            return f"Directory not found: {path}"
        if not safe.is_dir():
            return f"Not a directory: {path}"

        entries = sorted(safe.iterdir())
        if not entries:
            return f"(empty directory: {path})"

        lines = []
        for entry in entries[:100]:  # Cap at 100 entries
            rel = entry.relative_to(WORKSPACE)
            if entry.is_dir():
                lines.append(f"  {rel}/")
            else:
                size = entry.stat().st_size
                lines.append(f"  {rel}  ({_fmt_size(size)})")

        return f"Workspace: {WORKSPACE}\n\n" + "\n".join(lines)

    except ValueError as e:
        return str(e)
    except Exception as e:
        return f"Error listing files: {e}"


def _fmt_size(size: int) -> str:
    for unit in ("B", "KB", "MB"):
        if size < 1024:
            return f"{size:.0f}{unit}"
        size /= 1024
    return f"{size:.1f}GB"
