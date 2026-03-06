"""
Remember skill — lets the LLM save and recall facts about the user.
"""

import logging
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.remember")

# Memory instance is injected by bot.py at startup
_memory = None
_current_user_id = None


def configure(memory=None):
    global _memory
    if memory is not None:
        _memory = memory


def set_current_user(user_id: str):
    global _current_user_id
    _current_user_id = user_id


@skill(
    name="save_fact",
    description="Save a fact about the user for future conversations. Use when the user tells you their name, preferences, or other personal details worth remembering.",
    parameters={
        "key": {"type": "string", "description": "Short label for the fact, e.g. 'name', 'timezone', 'favorite_language'"},
        "value": {"type": "string", "description": "The fact value, e.g. 'Alice', 'UTC+8', 'Python'"},
    },
    timeout=5,
)
def save_fact(key: str, value: str) -> str:
    if not _memory or not _current_user_id:
        return "Error: Memory not available."
    _memory.save_fact(_current_user_id, key, value)
    logger.info(f"Saved fact for {_current_user_id}: {key}={value}")
    return f"Remembered: {key} = {value}"


@skill(
    name="get_facts",
    description="Recall all saved facts about the current user. Use when you need to remember something about them.",
    parameters={},
    timeout=5,
)
def get_facts() -> str:
    if not _memory or not _current_user_id:
        return "Error: Memory not available."
    facts = _memory.get_facts(_current_user_id)
    if not facts:
        return "No facts saved for this user yet."
    return "\n".join(f"- {k}: {v}" for k, v in facts.items())
