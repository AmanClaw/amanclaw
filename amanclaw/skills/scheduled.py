"""
Scheduled tasks skill — recurring tasks that fire on a cron-like schedule.
"""

import logging
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.scheduled")

_memory = None
_current_user_id = None
_current_chat_id = None

DAY_LABELS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]


def configure(memory=None):
    global _memory
    if memory is not None:
        _memory = memory


def set_context(user_id: str, chat_id: str):
    global _current_user_id, _current_chat_id
    _current_user_id = user_id
    _current_chat_id = chat_id


@skill(
    name="create_schedule",
    description="Create a recurring scheduled task. Runs at a specific time on specified days. Examples: 'every weekday at 9am remind me to check email', 'every Monday at 14:00 send weekly report reminder'.",
    parameters={
        "hour": {"type": "integer", "description": "Hour in 24h format (0-23)"},
        "minute": {"type": "integer", "description": "Minute (0-59)"},
        "message": {"type": "string", "description": "The message to send each time"},
        "days": {
            "type": "string",
            "description": "Comma-separated day numbers (0=Mon, 6=Sun) or 'weekdays' or 'daily'. Default: 'daily'",
            "optional": True,
        },
    },
    timeout=5,
)
def create_schedule(hour: int, minute: int, message: str, days: str = "daily") -> str:
    if not _memory or not _current_user_id or not _current_chat_id:
        return "Error: Schedule system not available."
    if not (0 <= int(hour) <= 23) or not (0 <= int(minute) <= 59):
        return "Error: Invalid time. Hour must be 0-23, minute 0-59."

    if days == "daily":
        day_str = "0,1,2,3,4,5,6"
    elif days == "weekdays":
        day_str = "0,1,2,3,4"
    elif days == "weekends":
        day_str = "5,6"
    else:
        day_str = days

    _memory.add_schedule(
        _current_user_id, "telegram", _current_chat_id,
        message, int(hour), int(minute), day_str
    )

    day_labels = [DAY_LABELS[int(d)] for d in day_str.split(",")
                  if d.strip().isdigit() and 0 <= int(d) <= 6]
    return f"Scheduled: '{message}' at {int(hour):02d}:{int(minute):02d} on {', '.join(day_labels)}"


@skill(
    name="list_schedules",
    description="List all recurring scheduled tasks for the current user.",
    parameters={},
    timeout=5,
)
def list_schedules() -> str:
    if not _memory or not _current_user_id:
        return "Error: Schedule system not available."
    schedules = _memory.get_user_schedules(_current_user_id)
    if not schedules:
        return "No scheduled tasks."
    lines = []
    for s in schedules:
        status = "ON" if s["enabled"] else "OFF"
        day_labels = [DAY_LABELS[int(d)] for d in s["days"].split(",") if d.strip().isdigit()]
        lines.append(f"#{s['id']} [{status}] {s['hour']:02d}:{s['minute']:02d} {','.join(day_labels)} — {s['message']}")
    return "\n".join(lines)


@skill(
    name="delete_schedule",
    description="Delete a recurring scheduled task by its ID.",
    parameters={
        "schedule_id": {"type": "integer", "description": "The schedule ID to delete (from list_schedules)"},
    },
    timeout=5,
)
def delete_schedule(schedule_id: int) -> str:
    if not _memory or not _current_user_id:
        return "Error: Schedule system not available."
    if _memory.delete_schedule(int(schedule_id), _current_user_id):
        return f"Schedule #{schedule_id} deleted."
    return f"Schedule #{schedule_id} not found."
