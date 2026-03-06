"""
Shell skill — run whitelisted commands only.
No sudo, no pipes, no redirects. Safe for personal use.
"""

import subprocess
import shlex
import logging
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.shell")

# Default allowlist — override via config
ALLOWED_COMMANDS = {
    "ls", "cat", "head", "tail", "wc",
    "grep", "find", "which",
    "df", "du", "free", "uptime", "date",
    "ps", "whoami", "hostname",
    "pwd", "echo", "tree",
}

# Characters that could enable command chaining
DANGEROUS_CHARS = {";", "|", "&", "`", "$", "(", ")", "{", "}", "<", ">", "\\"}


def configure(allowed_commands: list[str] = None, working_dir: str = None):
    """Update shell skill config at runtime."""
    global ALLOWED_COMMANDS, WORKING_DIR
    if allowed_commands:
        ALLOWED_COMMANDS = set(allowed_commands)
    if working_dir:
        WORKING_DIR = working_dir


WORKING_DIR = None  # Set via config, defaults to home


@skill(
    name="run_command",
    description="Run a shell command on the system. Only safe, read-only commands are allowed (ls, cat, grep, find, df, free, uptime, ps, etc). No sudo, no pipes, no destructive commands.",
    parameters={
        "command": {
            "type": "string",
            "description": "The shell command to run (e.g., 'ls -la /home', 'df -h', 'uptime')",
        },
    },
    timeout=30,
)
def run_command(command: str) -> str:
    """Run a whitelisted shell command."""

    # Check for dangerous characters (no pipes, chains, redirects)
    if any(c in command for c in DANGEROUS_CHARS):
        return (
            f"Command blocked: contains dangerous characters. "
            f"Pipes, redirects, and command chaining are not allowed."
        )

    # Parse and validate
    try:
        parts = shlex.split(command)
    except ValueError as e:
        return f"Invalid command syntax: {e}"

    if not parts:
        return "Empty command"

    cmd_name = parts[0]

    # Check allowlist
    if cmd_name not in ALLOWED_COMMANDS:
        return (
            f"Command '{cmd_name}' is not allowed.\n"
            f"Allowed commands: {', '.join(sorted(ALLOWED_COMMANDS))}"
        )

    # Block path traversal in arguments
    for arg in parts[1:]:
        if ".." in arg and ("etc/passwd" in arg or "etc/shadow" in arg):
            return "Command blocked: suspicious path detected."

    logger.info(f"Running: {command}")

    try:
        result = subprocess.run(
            parts,
            capture_output=True,
            text=True,
            timeout=25,
            cwd=WORKING_DIR,
        )

        output = result.stdout[:3000]
        if result.stderr:
            output += f"\n[stderr]: {result.stderr[:500]}"
        if result.returncode != 0:
            output += f"\n[exit code: {result.returncode}]"

        return output or "(no output)"

    except subprocess.TimeoutExpired:
        return "Command timed out after 25 seconds."
    except FileNotFoundError:
        return f"Command '{cmd_name}' not found on this system."
    except Exception as e:
        return f"Command failed: {e}"
