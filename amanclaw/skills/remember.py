"""
Remember skill — lets the LLM save and recall facts about the user.
Writes to the knowledge graph (knowledge table).
"""

import re
import logging
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.remember")

_memory = None
_current_user_id = None
_learning_engine = None


def configure(memory=None):
    global _memory
    if memory is not None:
        _memory = memory


def set_current_user(user_id: str):
    global _current_user_id
    _current_user_id = user_id


def set_learning_engine(engine):
    global _learning_engine
    _learning_engine = engine


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


@skill(
    name="teach",
    description="Learn a rule or instruction from the user. Use when the user teaches you how to behave, respond, or handle specific situations. Examples: 'when I say deploy, push staging first', 'always respond in Malay'.",
    parameters={
        "rule": {"type": "string", "description": "The rule or instruction to learn, e.g. 'when I say deploy, push to staging first'"},
        "category": {"type": "string", "description": "Category: work, personal, communication, general. Default: general", "optional": True},
    },
    timeout=5,
)
def teach(rule: str, category: str = "general") -> str:
    if not _memory or not _current_user_id:
        return "Error: Memory not available."
    if _learning_engine:
        # Split rule into trigger and guidance if possible
        parts = re.split(r',\s*|\.\s+', rule, maxsplit=1)
        trigger = parts[0]
        guidance = parts[1] if len(parts) > 1 else rule
        _learning_engine.save_teaching(_current_user_id, trigger, guidance, category)
    else:
        _memory.save_teaching(_current_user_id, rule, rule, category)
    logger.info(f"Teaching saved for {_current_user_id}: {rule}")
    return f"Got it, I've learned: {rule}"


@skill(
    name="learned",
    description="Show what I've learned recently — new knowledge, corrections, teachings, and patterns. Use when the user asks 'what have you learned?' or 'show me your learning journal'.",
    parameters={
        "days": {"type": "integer", "description": "How many days back to look (default 7)", "optional": True},
    },
    timeout=5,
)
def learned(days: int = 7) -> str:
    if not _current_user_id:
        return "Error: No user context."
    if _learning_engine:
        return _learning_engine.get_learning_journal(_current_user_id, days=days)
    return "Learning engine not available."


@skill(
    name="forget",
    description="Forget specific knowledge about the user. Use when the user says 'forget about X' or 'remove what you know about X'.",
    parameters={
        "query": {"type": "string", "description": "What to forget, e.g. 'coffee preference', 'my old job'"},
    },
    timeout=5,
)
def forget(query: str) -> str:
    if not _memory or not _current_user_id:
        return "Error: Memory not available."
    results = _memory.search_knowledge(_current_user_id, query)
    if not results:
        return f"I don't have any knowledge matching: {query}"
    # Expire matching entries
    count = 0
    for r in results:
        _memory.conn.execute("UPDATE knowledge SET expired = 1 WHERE id = ?", (r["id"],))
        count += 1
    _memory.conn.commit()
    subjects = ", ".join(r["subject"] for r in results[:5])
    logger.info(f"Forgot {count} entries for {_current_user_id}: {subjects}")
    return f"Forgot {count} item(s) about: {subjects}"
