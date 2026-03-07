"""
Skill system — simple decorator-based registry.
Each skill is a Python function with metadata.
"""

import signal
import logging
import traceback
from typing import Any, Callable

logger = logging.getLogger("amanclaw.skills")

# Optional MCP manager (set during bot startup)
_mcp_manager = None

# Optional user skill manager (set during bot startup)
_user_skill_manager = None


def set_mcp_manager(manager):
    """Set the MCP manager instance for tool integration."""
    global _mcp_manager
    _mcp_manager = manager


def set_user_skill_manager(manager):
    """Set the UserSkillManager instance."""
    global _user_skill_manager
    _user_skill_manager = manager


# Global skill registry
REGISTRY: dict[str, dict] = {}


def skill(name: str, description: str, parameters: dict, timeout: int = 30):
    """
    Decorator to register a function as a skill.

    Args:
        name: Unique skill name (used as tool name for Claude)
        description: What this skill does (Claude reads this)
        parameters: JSON Schema for the skill's input parameters
        timeout: Max execution time in seconds
    """
    def decorator(func: Callable) -> Callable:
        REGISTRY[name] = {
            "name": name,
            "description": description,
            "parameters": parameters,
            "timeout": timeout,
            "function": func,
        }
        logger.info(f"Registered skill: {name}")
        return func
    return decorator


def get_tool_definitions(user_id: str = None) -> list[dict]:
    """Get all skills (built-in + MCP + user) as tool definitions."""
    tools = []
    for name, info in REGISTRY.items():
        tools.append({
            "name": info["name"],
            "description": info["description"],
            "input_schema": {
                "type": "object",
                "properties": info["parameters"],
                "required": [
                    k for k, v in info["parameters"].items()
                    if not v.get("optional", False)
                ],
            },
        })
    # Merge MCP tools
    if _mcp_manager:
        tools.extend(_mcp_manager.get_tool_definitions())
    # Merge user skills
    if _user_skill_manager and user_id:
        tools.extend(_user_skill_manager.get_tool_definitions(user_id))
    return tools


def get_skill_list() -> str:
    """Get a human-readable list of skills."""
    lines = []
    for name, info in REGISTRY.items():
        lines.append(f"- {name}: {info['description']}")
    return "\n".join(lines)


def execute(tool_name: str, tool_input: dict, user_id: str = None) -> str:
    """
    Execute a skill by name with timeout protection.
    Returns the result as a string.
    Delegates to MCP manager for MCP tools.
    """
    # Check user skills first
    if _user_skill_manager and _user_skill_manager.has_skill(tool_name):
        return _user_skill_manager.execute(tool_name, tool_input, user_id or "")

    # Check MCP for prefixed tools
    if _mcp_manager and _mcp_manager.has_tool(tool_name):
        # MCP tools are async — run in event loop
        import asyncio
        try:
            loop = asyncio.get_running_loop()
        except RuntimeError:
            loop = None

        if loop and loop.is_running():
            # We're already in an async context - create a future
            import concurrent.futures
            with concurrent.futures.ThreadPoolExecutor() as pool:
                result = pool.submit(
                    asyncio.run, _mcp_manager.execute(tool_name, tool_input)
                ).result(timeout=30)
            return result
        else:
            return asyncio.run(_mcp_manager.execute(tool_name, tool_input))

    if tool_name not in REGISTRY:
        return f"Error: Unknown skill '{tool_name}'"

    info = REGISTRY[tool_name]
    func = info["function"]
    timeout = info["timeout"]

    logger.info(f"Executing skill: {tool_name} (timeout: {timeout}s)")

    def _timeout_handler(signum, frame):
        raise TimeoutError(f"Skill '{tool_name}' timed out after {timeout}s")

    # Set timeout
    old_handler = signal.signal(signal.SIGALRM, _timeout_handler)
    signal.alarm(timeout)

    try:
        result = func(**tool_input)
        return str(result)
    except TimeoutError as e:
        logger.warning(str(e))
        return str(e)
    except Exception as e:
        error_msg = f"Skill '{tool_name}' failed: {type(e).__name__}: {e}"
        logger.error(error_msg)
        logger.debug(traceback.format_exc())
        return error_msg
    finally:
        signal.alarm(0)
        signal.signal(signal.SIGALRM, old_handler)


# Import built-in skills so they auto-register
from amanclaw.skills import shell, files, system_info, remember, reminder, documents, scheduled, prayer_times, web_fetch, web_search, mcp_manage
