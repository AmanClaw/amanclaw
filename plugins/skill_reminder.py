#!/usr/bin/env python3
"""Simple reminder storage — saves reminders for the user."""
import json
import os
from datetime import datetime
from amanclaw_sdk import plugin, SkillInput, SkillResult

REMINDERS_FILE = os.path.join(os.environ.get("DATA_DIR", "data"), "reminders.json")

def load_reminders(user_id):
    if os.path.exists(REMINDERS_FILE):
        with open(REMINDERS_FILE, "r") as f:
            all_data = json.load(f)
        return all_data.get(user_id, [])
    return []

def save_reminders(user_id, reminders):
    all_data = {}
    if os.path.exists(REMINDERS_FILE):
        with open(REMINDERS_FILE, "r") as f:
            all_data = json.load(f)
    all_data[user_id] = reminders
    os.makedirs(os.path.dirname(REMINDERS_FILE), exist_ok=True)
    with open(REMINDERS_FILE, "w") as f:
        json.dump(all_data, f, indent=2)

@plugin(
    name="reminder",
    description="Set and manage reminders. Reminders are stored persistently.",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["set", "list", "remove", "clear"], "description": "Operation"},
            "message": {"type": "string", "description": "Reminder message (for set)"},
            "when": {"type": "string", "description": "When to remind (e.g., 'tomorrow', '2026-03-20', 'Friday')"},
            "index": {"type": "integer", "description": "Reminder number to remove (1-based)"}
        },
        "required": ["action"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "list")
    user_id = inp.user_id

    try:
        reminders = load_reminders(user_id)

        if action == "set":
            message = args.get("message", "")
            when = args.get("when", "unspecified")
            if not message:
                return SkillResult.err("Please provide a reminder message.")
            reminders.append({
                "message": message,
                "when": when,
                "created": datetime.now().isoformat(),
            })
            save_reminders(user_id, reminders)
            return SkillResult.ok(f"Reminder set: {message} (when: {when})")

        elif action == "list":
            if not reminders:
                return SkillResult.ok("No reminders set.")
            lines = []
            for i, r in enumerate(reminders, 1):
                lines.append(f"{i}. {r['message']} \u2014 {r['when']}")
            return SkillResult.ok("\n".join(lines))

        elif action == "remove":
            idx = args.get("index", 0) - 1
            if 0 <= idx < len(reminders):
                removed = reminders.pop(idx)
                save_reminders(user_id, reminders)
                return SkillResult.ok(f"Removed: {removed['message']}")
            return SkillResult.err("Invalid reminder number.")

        elif action == "clear":
            save_reminders(user_id, [])
            return SkillResult.ok("All reminders cleared.")

        return SkillResult.err(f"Unknown action: {action}")
    except Exception as e:
        return SkillResult.err(f"Reminder error: {e}")

if __name__ == "__main__":
    execute.run()
