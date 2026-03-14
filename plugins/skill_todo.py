#!/usr/bin/env python3
"""Simple persistent todo list."""
import json
import os
from amanclaw_sdk import plugin, SkillInput, SkillResult

TODO_FILE = os.path.join(os.environ.get("DATA_DIR", "data"), "todos.json")

def load_todos(user_id):
    if os.path.exists(TODO_FILE):
        with open(TODO_FILE, "r") as f:
            all_todos = json.load(f)
        return all_todos.get(user_id, [])
    return []

def save_todos(user_id, todos):
    all_todos = {}
    if os.path.exists(TODO_FILE):
        with open(TODO_FILE, "r") as f:
            all_todos = json.load(f)
    all_todos[user_id] = todos
    os.makedirs(os.path.dirname(TODO_FILE), exist_ok=True)
    with open(TODO_FILE, "w") as f:
        json.dump(all_todos, f, indent=2)

@plugin(
    name="todo",
    description="Manage a personal todo list. Add, complete, remove, and list tasks.",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["add", "list", "done", "remove", "clear"], "description": "Operation"},
            "task": {"type": "string", "description": "Task description (for add)"},
            "index": {"type": "integer", "description": "Task number (for done/remove, 1-based)"}
        },
        "required": ["action"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "list")
    user_id = inp.user_id

    try:
        todos = load_todos(user_id)

        if action == "add":
            task = args.get("task", "")
            if not task:
                return SkillResult.err("Please provide a task description.")
            todos.append({"task": task, "done": False})
            save_todos(user_id, todos)
            return SkillResult.ok(f"Added: {task} (#{len(todos)})")

        elif action == "list":
            if not todos:
                return SkillResult.ok("No todos. Add one with: todo add <task>")
            lines = []
            for i, t in enumerate(todos, 1):
                status = "\u2713" if t["done"] else "\u25cb"
                lines.append(f"{i}. [{status}] {t['task']}")
            return SkillResult.ok("\n".join(lines))

        elif action == "done":
            idx = args.get("index", 0) - 1
            if 0 <= idx < len(todos):
                todos[idx]["done"] = True
                save_todos(user_id, todos)
                return SkillResult.ok(f"Completed: {todos[idx]['task']}")
            return SkillResult.err(f"Invalid task number. You have {len(todos)} tasks.")

        elif action == "remove":
            idx = args.get("index", 0) - 1
            if 0 <= idx < len(todos):
                removed = todos.pop(idx)
                save_todos(user_id, todos)
                return SkillResult.ok(f"Removed: {removed['task']}")
            return SkillResult.err(f"Invalid task number.")

        elif action == "clear":
            save_todos(user_id, [])
            return SkillResult.ok("All todos cleared.")

        return SkillResult.err(f"Unknown action: {action}")
    except Exception as e:
        return SkillResult.err(f"Todo error: {e}")

if __name__ == "__main__":
    execute.run()
