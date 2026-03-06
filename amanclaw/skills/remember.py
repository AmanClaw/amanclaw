"""
Remember skill — lets the LLM save and recall facts about the user.
Writes to the knowledge graph (knowledge table).
"""

import logging
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.remember")

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
        "category": {"type": "string", "description": "Category: preference, personal, work, health, routine, temporal. Default: personal", "optional": True},
        "context": {"type": "string", "description": "Optional condition, e.g. 'in the evening', 'on weekdays'", "optional": True},
    },
    timeout=5,
)
def save_fact(key: str, value: str, category: str = "personal", context: str = None) -> str:
    if not _memory or not _current_user_id:
        return "Error: Memory not available."
    # Write to knowledge graph
    _memory.save_knowledge(
        _current_user_id,
        category=category,
        subject=key,
        content=value,
        context=context,
        source="explicit",
    )
    # Also write to legacy facts table for backward compat
    _memory.save_fact(_current_user_id, key, value)
    logger.info(f"Saved knowledge for {_current_user_id}: [{category}] {key}={value}")
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
    entries = _memory.get_active_knowledge(_current_user_id)
    if not entries:
        # Fallback to legacy facts
        facts = _memory.get_facts(_current_user_id)
        if not facts:
            return "No facts saved for this user yet."
        return "\n".join(f"- {k}: {v}" for k, v in facts.items())
    lines = []
    for e in entries:
        line = f"- [{e['category']}] {e['subject']}: {e['content']}"
        if e.get("context"):
            line += f" (context: {e['context']})"
        lines.append(line)
    return "\n".join(lines)


@skill(
    name="recall",
    description="Search through saved knowledge about the user. Use when you need to find specific information the user told you before.",
    parameters={
        "query": {"type": "string", "description": "What to search for, e.g. 'coffee preferences', 'work projects'"},
    },
    timeout=5,
)
def recall(query: str) -> str:
    if not _memory or not _current_user_id:
        return "Error: Memory not available."
    results = _memory.search_knowledge(_current_user_id, query)
    if not results:
        return f"No knowledge found matching: {query}"
    lines = [f"Search results for '{query}':"]
    for r in results:
        line = f"- [{r['category']}] {r['subject']}: {r['content']}"
        if r.get("context"):
            line += f" (context: {r['context']})"
        lines.append(line)
    return "\n".join(lines)
