"""
Reminder skill — set timed reminders that fire as Telegram messages.
"""

import logging
from datetime import datetime, timedelta
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.reminder")

_memory = None
_current_user_id = None
_current_chat_id = None


def configure(memory=None):
    global _memory
    if memory is not None:
        _memory = memory


def set_context(user_id: str, chat_id: str):
    global _current_user_id, _current_chat_id
    _current_user_id = user_id
    _current_chat_id = chat_id


@skill(
    name="set_reminder",
    description="Set a reminder for the user. Specify the number of minutes from now and a message. Examples: 'remind me in 30 minutes to check the oven', 'set a reminder in 60 minutes for the meeting'.",
    parameters={
        "minutes": {
            "type": "integer",
            "description": "Number of minutes from now (e.g., 5, 30, 60, 1440 for 1 day)",
        },
        "message": {
            "type": "string",
            "description": "The reminder message to send",
        },
    },
    timeout=5,
)
def set_reminder(minutes: int, message: str) -> str:
    if not _memory or not _current_user_id or not _current_chat_id:
        return "Error: Reminder system not available."

    if minutes < 1:
        return "Error: Minutes must be at least 1."
    if minutes > 43200:  # 30 days
        return "Error: Maximum reminder time is 30 days (43200 minutes)."

    remind_at = datetime.now() + timedelta(minutes=int(minutes))
    remind_at_str = remind_at.strftime("%Y-%m-%d %H:%M:%S")

    _memory.add_reminder(
        _current_user_id, "telegram", _current_chat_id,
        message, remind_at_str
    )

    # Format nicely for the user
    if minutes < 60:
        time_str = f"{minutes} minute{'s' if minutes != 1 else ''}"
    elif minutes < 1440:
        hours = minutes // 60
        mins = minutes % 60
        time_str = f"{hours}h{mins}m" if mins else f"{hours} hour{'s' if hours != 1 else ''}"
    else:
        days = minutes // 1440
        time_str = f"{days} day{'s' if days != 1 else ''}"

    return f"Reminder set for {time_str} from now ({remind_at.strftime('%H:%M')}): {message}"


@skill(
    name="list_reminders",
    description="List all pending reminders for the current user.",
    parameters={},
    timeout=5,
)
def list_reminders() -> str:
    if not _memory or not _current_user_id:
        return "Error: Reminder system not available."

    reminders = _memory.get_user_reminders(_current_user_id)
    if not reminders:
        return "No pending reminders."

    lines = []
    for r in reminders:
        lines.append(f"#{r['id']} — {r['remind_at']}: {r['message']}")
    return "\n".join(lines)


@skill(
    name="cancel_reminder",
    description="Cancel a pending reminder by its ID number.",
    parameters={
        "reminder_id": {
            "type": "integer",
            "description": "The reminder ID to cancel (shown in list_reminders)",
        },
    },
    timeout=5,
)
def cancel_reminder(reminder_id: int) -> str:
    if not _memory or not _current_user_id:
        return "Error: Reminder system not available."

    if _memory.delete_reminder(int(reminder_id), _current_user_id):
        return f"Reminder #{reminder_id} cancelled."
    return f"Reminder #{reminder_id} not found or already delivered."
