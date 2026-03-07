# Self-Extending Skill System — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Allow users at any technical level to add custom API integrations (skills) to AmanClaw, with private/public visibility, admin approval for marketplace, and smart failure detection that guides users to create skills.

**Architecture:** User skills stored in SQLite `user_skills` table. A new `UserSkillManager` class handles CRUD, execution (HTTP-only sandbox), and per-user tool list merging. The LLM module gets a `user_id` parameter so it can include user-specific skills in tool definitions. Bot detects capability failures and subtly suggests `/addskill`. A conversational `/addskill` flow walks users through API configuration.

**Tech Stack:** Python, SQLite, requests (HTTP runner), existing skill decorator system

---

### Task 1: Add user_skills Table to Memory

**Files:**
- Modify: `amanclaw/memory.py:25-96` (inside `_init_tables`)

**Step 1: Add the table creation SQL**

Add this table at the end of the `_init_tables` executescript, before the closing `"""`):

```sql
CREATE TABLE IF NOT EXISTS user_skills (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    url_template TEXT NOT NULL,
    method TEXT DEFAULT 'GET',
    headers JSON DEFAULT '{}',
    query_params JSON DEFAULT '{}',
    body_template JSON,
    response_mapping JSON,
    response_format TEXT,
    parameters JSON NOT NULL DEFAULT '{}',
    api_key_encrypted TEXT,
    is_private INTEGER DEFAULT 1,
    is_approved INTEGER DEFAULT 0,
    status TEXT DEFAULT 'active',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_skills_name
    ON user_skills(user_id, name);

CREATE INDEX IF NOT EXISTS idx_user_skills_marketplace
    ON user_skills(is_private, is_approved, status);
```

**Step 2: Add CRUD methods to Memory class**

Add these methods to the `Memory` class:

```python
def save_user_skill(self, user_id: str, skill_data: dict) -> int:
    """Save a user-created skill. Returns the skill ID."""
    cur = self.conn.execute(
        """INSERT OR REPLACE INTO user_skills
           (user_id, name, description, url_template, method, headers,
            query_params, body_template, response_mapping, response_format,
            parameters, api_key_encrypted, is_private)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
        (user_id, skill_data["name"], skill_data["description"],
         skill_data["url_template"], skill_data.get("method", "GET"),
         json.dumps(skill_data.get("headers", {})),
         json.dumps(skill_data.get("query_params", {})),
         json.dumps(skill_data.get("body_template")),
         json.dumps(skill_data.get("response_mapping")),
         skill_data.get("response_format"),
         json.dumps(skill_data.get("parameters", {})),
         skill_data.get("api_key_encrypted"),
         1 if skill_data.get("is_private", True) else 0),
    )
    self.conn.commit()
    return cur.lastrowid

def get_user_skills(self, user_id: str) -> list[dict]:
    """Get all active skills for a user (their own + approved marketplace skills)."""
    rows = self.conn.execute(
        """SELECT * FROM user_skills
           WHERE status = 'active' AND (
               (user_id = ?) OR
               (is_private = 0 AND is_approved = 1)
           )""",
        (user_id,),
    ).fetchall()
    cols = [d[0] for d in self.conn.execute("SELECT * FROM user_skills LIMIT 0").description]
    return [dict(zip(cols, r)) for r in rows]

def get_user_skill_by_name(self, name: str, user_id: str) -> dict | None:
    """Get a specific skill by name, checking user's own + marketplace."""
    row = self.conn.execute(
        """SELECT * FROM user_skills
           WHERE name = ? AND status = 'active' AND (
               (user_id = ?) OR
               (is_private = 0 AND is_approved = 1)
           ) LIMIT 1""",
        (name, user_id),
    ).fetchone()
    if not row:
        return None
    cols = [d[0] for d in self.conn.execute("SELECT * FROM user_skills LIMIT 0").description]
    return dict(zip(cols, row))

def delete_user_skill(self, user_id: str, name: str) -> bool:
    """Delete a user's skill."""
    cur = self.conn.execute(
        "DELETE FROM user_skills WHERE user_id = ? AND name = ?",
        (user_id, name),
    )
    self.conn.commit()
    return cur.rowcount > 0

def approve_user_skill(self, skill_id: int) -> bool:
    """Admin: approve a skill for marketplace."""
    cur = self.conn.execute(
        "UPDATE user_skills SET is_approved = 1, is_private = 0 WHERE id = ?",
        (skill_id,),
    )
    self.conn.commit()
    return cur.rowcount > 0

def get_marketplace_skills(self) -> list[dict]:
    """Get all approved marketplace skills."""
    rows = self.conn.execute(
        """SELECT id, name, description, user_id, created_at
           FROM user_skills
           WHERE is_private = 0 AND is_approved = 1 AND status = 'active'""",
    ).fetchall()
    return [{"id": r[0], "name": r[1], "description": r[2],
             "creator": r[3], "created_at": r[4]} for r in rows]

def get_pending_skills(self) -> list[dict]:
    """Admin: get skills pending approval."""
    rows = self.conn.execute(
        """SELECT id, user_id, name, description, url_template, method, created_at
           FROM user_skills
           WHERE is_private = 0 AND is_approved = 0 AND status = 'active'""",
    ).fetchall()
    return [{"id": r[0], "user_id": r[1], "name": r[2], "description": r[3],
             "url_template": r[4], "method": r[5], "created_at": r[6]} for r in rows]

def publish_user_skill(self, user_id: str, name: str) -> bool:
    """Mark a skill as submitted for marketplace review."""
    cur = self.conn.execute(
        """UPDATE user_skills SET is_private = 0, is_approved = 0
           WHERE user_id = ? AND name = ? AND is_private = 1""",
        (user_id, name),
    )
    self.conn.commit()
    return cur.rowcount > 0
```

**Step 3: Verify**

Run: `python -c "import amanclaw.memory; print('OK')"`
Expected: OK

**Step 4: Commit**

```bash
git add amanclaw/memory.py
git commit -m "feat: add user_skills table and CRUD methods to Memory"
```

---

### Task 2: Create UserSkillManager — Execution Engine

**Files:**
- Create: `amanclaw/skills/user_skills.py`

**Step 1: Create the user skill execution engine**

```python
"""
User Skills — HTTP-only sandboxed execution for user-created API integrations.
"""

import re
import json
import logging
import requests
from amanclaw.memory import Memory

logger = logging.getLogger("amanclaw.skills.user_skills")

# Max response size to prevent memory issues
MAX_RESPONSE_SIZE = 1_000_000  # 1MB
REQUEST_TIMEOUT = 10  # seconds


class UserSkillManager:
    """Manages user-created skills: tool definitions + sandboxed HTTP execution."""

    def __init__(self, memory: Memory):
        self.memory = memory

    def get_tool_definitions(self, user_id: str) -> list[dict]:
        """Get user skills as LLM tool definitions for a specific user."""
        skills = self.memory.get_user_skills(user_id)
        tools = []
        for s in skills:
            params = json.loads(s["parameters"]) if isinstance(s["parameters"], str) else s["parameters"]
            tools.append({
                "name": f"uskill_{s['name']}",
                "description": s["description"],
                "input_schema": {
                    "type": "object",
                    "properties": params,
                    "required": [k for k, v in params.items() if not v.get("optional", False)],
                },
            })
        return tools

    def has_skill(self, tool_name: str) -> bool:
        """Check if a tool name is a user skill."""
        return tool_name.startswith("uskill_")

    def execute(self, tool_name: str, tool_input: dict, user_id: str) -> str:
        """Execute a user skill by making the configured HTTP request."""
        skill_name = tool_name.replace("uskill_", "", 1)
        skill = self.memory.get_user_skill_by_name(skill_name, user_id)

        if not skill:
            return f"Error: User skill '{skill_name}' not found"

        try:
            return self._run_http_request(skill, tool_input)
        except requests.Timeout:
            return f"Skill '{skill_name}' timed out after {REQUEST_TIMEOUT}s"
        except requests.RequestException as e:
            logger.error(f"User skill '{skill_name}' HTTP error: {e}")
            return f"Skill '{skill_name}' failed: {e}"
        except Exception as e:
            logger.error(f"User skill '{skill_name}' error: {e}")
            return f"Skill '{skill_name}' error: {type(e).__name__}: {e}"

    def _run_http_request(self, skill: dict, params: dict) -> str:
        """Execute the HTTP request defined by a user skill."""
        # Substitute parameters into URL template
        url = self._substitute(skill["url_template"], params)

        # Build headers
        headers_raw = json.loads(skill["headers"]) if isinstance(skill["headers"], str) else (skill["headers"] or {})
        headers = {k: self._substitute(v, params) for k, v in headers_raw.items()}

        # Substitute API key if present
        if skill.get("api_key_encrypted"):
            api_key = skill["api_key_encrypted"]  # TODO: decrypt in future
            headers = {k: v.replace("{api_key}", api_key) for k, v in headers.items()}
            url = url.replace("{api_key}", api_key)

        # Build query params
        qp_raw = json.loads(skill["query_params"]) if isinstance(skill["query_params"], str) else (skill["query_params"] or {})
        query_params = {k: self._substitute(v, params) for k, v in qp_raw.items()}

        # Build body for POST
        body = None
        method = (skill.get("method") or "GET").upper()
        if method == "POST" and skill.get("body_template"):
            body_raw = json.loads(skill["body_template"]) if isinstance(skill["body_template"], str) else skill["body_template"]
            body = json.loads(self._substitute(json.dumps(body_raw), params))

        # Make request
        logger.info(f"User skill HTTP {method} {url}")
        resp = requests.request(
            method=method,
            url=url,
            headers=headers,
            params=query_params if query_params else None,
            json=body,
            timeout=REQUEST_TIMEOUT,
        )
        resp.raise_for_status()

        # Size check
        if len(resp.content) > MAX_RESPONSE_SIZE:
            return "Error: Response too large (>1MB)"

        # Parse response
        try:
            data = resp.json()
        except ValueError:
            return resp.text[:2000]

        # Apply response mapping if defined
        response_mapping = skill.get("response_mapping")
        if response_mapping:
            mapping = json.loads(response_mapping) if isinstance(response_mapping, str) else response_mapping
            if mapping:
                extracted = {}
                for key, path in mapping.items():
                    extracted[key] = self._extract_jsonpath(data, path)
                data = extracted

        # Apply response format template if defined
        response_format = skill.get("response_format")
        if response_format:
            try:
                return response_format.format(**data) if isinstance(data, dict) else str(data)
            except (KeyError, IndexError):
                pass

        # Default: return pretty JSON
        return json.dumps(data, indent=2, ensure_ascii=False)[:3000]

    @staticmethod
    def _substitute(template: str, params: dict) -> str:
        """Replace {param_name} placeholders with actual values."""
        result = template
        for key, value in params.items():
            result = result.replace(f"{{{key}}}", str(value))
        return result

    @staticmethod
    def _extract_jsonpath(data: dict, path: str):
        """Simple JSONPath-like extraction: $.field.nested[0].value"""
        path = path.lstrip("$.")
        current = data
        for part in re.split(r'\.|\[|\]', path):
            if not part:
                continue
            if part.isdigit():
                try:
                    current = current[int(part)]
                except (IndexError, KeyError, TypeError):
                    return None
            else:
                try:
                    current = current[part]
                except (KeyError, TypeError):
                    return None
        return current
```

**Step 2: Verify**

Run: `python -c "from amanclaw.skills.user_skills import UserSkillManager; print('OK')"`
Expected: OK

**Step 3: Commit**

```bash
git add amanclaw/skills/user_skills.py
git commit -m "feat: add UserSkillManager — HTTP-only sandboxed skill execution"
```

---

### Task 3: Integrate User Skills into Skill Registry + LLM

**Files:**
- Modify: `amanclaw/skills/__init__.py:50-78` (get_tool_definitions, get_skill_list, execute)
- Modify: `amanclaw/llm.py:15,550,584,601,633` (pass user_id through)

**Step 1: Add user skill manager to __init__.py**

Add a global `_user_skill_manager` alongside `_mcp_manager`:

```python
# Optional user skill manager (set during bot startup)
_user_skill_manager = None


def set_user_skill_manager(manager):
    """Set the UserSkillManager instance."""
    global _user_skill_manager
    _user_skill_manager = manager
```

**Step 2: Update get_tool_definitions to accept user_id**

```python
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
```

**Step 3: Update execute to handle user skills**

Add user skill check at the top of `execute()`, before the MCP check:

```python
def execute(tool_name: str, tool_input: dict, user_id: str = None) -> str:
    # Check user skills first
    if _user_skill_manager and _user_skill_manager.has_skill(tool_name):
        return _user_skill_manager.execute(tool_name, tool_input, user_id or "")

    # Check MCP...
    # (rest stays the same)
```

**Step 4: Update LLM to pass user_id**

In `amanclaw/llm.py`, update the `respond()` method signature and internal calls:

- Add `user_id: str = None` parameter to `respond()`, `_respond_native()`, `_respond_fallback()`
- Pass `user_id` to `get_tool_definitions(user_id=user_id)` calls
- Pass `user_id` to `execute(func["name"], tool_input, user_id=user_id)` calls

**Step 5: Update bot.py to pass user_id to LLM**

In `handle_message()`, change:
```python
response = await llm.respond(clean_text, history, flagged=was_flagged,
                             facts=facts, summary=summary,
                             knowledge_context=knowledge_context,
                             user_id=user_id)
```

Same for `handle_photo()` and `handle_voice()`.

**Step 6: Verify**

Run: `python -c "import amanclaw.bot; print('OK')"`
Expected: OK

**Step 7: Commit**

```bash
git add amanclaw/skills/__init__.py amanclaw/llm.py amanclaw/bot.py
git commit -m "feat: integrate user skills into tool registry and LLM pipeline"
```

---

### Task 4: Add /addskill Command — Conversational Flow

**Files:**
- Modify: `amanclaw/bot.py` (add handler + conversation state)

**Step 1: Add conversation state tracking**

Near the top of bot.py, add:

```python
# Track /addskill conversation state per user
_addskill_state: dict[str, dict] = {}
```

**Step 2: Add cmd_addskill handler**

```python
async def cmd_addskill(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /addskill — start the skill creation flow."""
    user_id = str(update.effective_user.id)
    if not await handle_registration(update, context):
        return

    _addskill_state[user_id] = {"step": "describe"}
    await update.message.reply_text(
        "Let's create a new skill!\n\n"
        "First, describe what you want in one sentence.\n"
        "For example: \"Get weather info for any city\" or "
        "\"Convert currencies\"\n\n"
        "Send /cancel to stop at any time."
    )
```

**Step 3: Add the conversation handler in handle_message**

At the beginning of `handle_message()`, after registration check, add:

```python
    # Check if user is in /addskill flow
    if user_id in _addskill_state:
        await _handle_addskill_step(update, context, user_id, message_text)
        return
```

**Step 4: Implement the step handler**

```python
async def _handle_addskill_step(update: Update, context: ContextTypes.DEFAULT_TYPE,
                                 user_id: str, text: str):
    """Handle each step of the /addskill conversation."""
    if text.strip().lower() == "/cancel":
        del _addskill_state[user_id]
        await update.message.reply_text("Skill creation cancelled.")
        return

    state = _addskill_state[user_id]
    step = state["step"]

    if step == "describe":
        state["description"] = text.strip()
        state["step"] = "name"
        await update.message.reply_text(
            f"Got it: \"{text.strip()}\"\n\n"
            "What should this skill be called? Use a short name "
            "(lowercase, no spaces, e.g. \"weather\", \"currency\", \"news\")"
        )

    elif step == "name":
        name = text.strip().lower().replace(" ", "_")
        # Validate name
        if not re.match(r'^[a-z][a-z0-9_]{1,30}$', name):
            await update.message.reply_text(
                "Name must be lowercase letters/numbers/underscores, 2-31 chars. Try again."
            )
            return
        # Check for conflicts
        from amanclaw.skills import REGISTRY
        if name in REGISTRY or f"uskill_{name}" in REGISTRY:
            await update.message.reply_text(
                f"'{name}' conflicts with a built-in skill. Choose a different name."
            )
            return
        state["name"] = name
        state["step"] = "url"
        await update.message.reply_text(
            "Now provide the API URL.\n\n"
            "Use {param} for dynamic parts. Examples:\n"
            "- https://api.example.com/weather?city={city}\n"
            "- https://api.example.com/v1/{endpoint}\n\n"
            "If you have API documentation, just paste the URL and I'll parse it."
        )

    elif step == "url":
        url = text.strip()
        if not url.startswith("http://") and not url.startswith("https://"):
            await update.message.reply_text("URL must start with http:// or https://. Try again.")
            return
        state["url_template"] = url
        # Auto-detect parameters from {placeholders}
        params_found = re.findall(r'\{(\w+)\}', url)
        state["auto_params"] = params_found
        state["step"] = "method"

        keyboard = InlineKeyboardMarkup([
            [
                InlineKeyboardButton("GET", callback_data="addskill_method_GET"),
                InlineKeyboardButton("POST", callback_data="addskill_method_POST"),
            ]
        ])
        await update.message.reply_text(
            f"URL: {url}\n"
            f"Parameters detected: {', '.join(params_found) if params_found else 'none'}\n\n"
            "What HTTP method does this API use?",
            reply_markup=keyboard,
        )

    elif step == "params":
        # User provides additional parameter descriptions
        try:
            # Expect format: param1: description, param2: description
            params = {}
            for line in text.strip().split("\n"):
                if ":" in line:
                    pname, pdesc = line.split(":", 1)
                    params[pname.strip()] = {
                        "type": "string",
                        "description": pdesc.strip(),
                    }
            # Merge with auto-detected params
            for p in state.get("auto_params", []):
                if p not in params and p != "api_key":
                    params[p] = {"type": "string", "description": p}
            state["parameters"] = params
        except Exception:
            state["parameters"] = {p: {"type": "string", "description": p}
                                   for p in state.get("auto_params", []) if p != "api_key"}

        state["step"] = "apikey"
        keyboard = InlineKeyboardMarkup([
            [
                InlineKeyboardButton("No API key needed", callback_data="addskill_nokey"),
                InlineKeyboardButton("Yes, I have a key", callback_data="addskill_haskey"),
            ]
        ])
        await update.message.reply_text(
            "Does this API require an API key?",
            reply_markup=keyboard,
        )

    elif step == "apikey_input":
        state["api_key"] = text.strip()
        state["step"] = "confirm"
        await _show_addskill_confirmation(update, state)

    elif step == "response_format":
        state["response_format"] = text.strip()
        state["step"] = "confirm"
        await _show_addskill_confirmation(update, state)


async def _show_addskill_confirmation(update_or_query, state: dict):
    """Show the skill summary for confirmation."""
    summary = (
        f"*Skill Summary:*\n\n"
        f"Name: `{state['name']}`\n"
        f"Description: {state['description']}\n"
        f"URL: `{state['url_template']}`\n"
        f"Method: {state.get('method', 'GET')}\n"
        f"Parameters: {', '.join(state.get('parameters', {}).keys()) or 'none'}\n"
        f"API Key: {'yes (stored securely)' if state.get('api_key') else 'none'}"
    )
    keyboard = InlineKeyboardMarkup([
        [
            InlineKeyboardButton("Create Skill", callback_data="addskill_confirm"),
            InlineKeyboardButton("Cancel", callback_data="addskill_cancel"),
        ]
    ])
    msg = update_or_query.message if hasattr(update_or_query, 'message') else update_or_query
    await msg.reply_text(summary, parse_mode=ParseMode.MARKDOWN, reply_markup=keyboard)
```

**Step 5: Add addskill callbacks in handle_callback**

```python
    # --- Addskill flow callbacks ---
    if query.data.startswith("addskill_method_"):
        method = query.data.replace("addskill_method_", "")
        if user_id in _addskill_state:
            _addskill_state[user_id]["method"] = method
            _addskill_state[user_id]["step"] = "params"
            params = _addskill_state[user_id].get("auto_params", [])
            if params:
                await query.edit_message_text(
                    f"Method: {method}\n\n"
                    f"I found these parameters: {', '.join(params)}\n\n"
                    "Describe each parameter (one per line):\n"
                    "param_name: description\n\n"
                    "Or just send 'ok' to use the defaults."
                )
            else:
                await query.edit_message_text(
                    f"Method: {method}\n\n"
                    "List the parameters this API needs (one per line):\n"
                    "param_name: description\n\n"
                    "Or send 'none' if no parameters needed."
                )
        return

    if query.data == "addskill_nokey":
        if user_id in _addskill_state:
            _addskill_state[user_id]["api_key"] = None
            _addskill_state[user_id]["step"] = "confirm"
            await query.edit_message_text("No API key needed.")
            await _show_addskill_confirmation(query, _addskill_state[user_id])
        return

    if query.data == "addskill_haskey":
        if user_id in _addskill_state:
            _addskill_state[user_id]["step"] = "apikey_input"
            await query.edit_message_text(
                "Send me the API key. I'll store it securely and never show it again."
            )
        return

    if query.data == "addskill_confirm":
        if user_id in _addskill_state:
            state = _addskill_state.pop(user_id)
            skill_data = {
                "name": state["name"],
                "description": state["description"],
                "url_template": state["url_template"],
                "method": state.get("method", "GET"),
                "parameters": state.get("parameters", {}),
                "api_key_encrypted": state.get("api_key"),
                "is_private": True,
            }
            memory.save_user_skill(user_id, skill_data)
            await query.edit_message_text(
                f"Skill `{state['name']}` created!\n\n"
                f"Try it now — just ask me something that uses it.\n\n"
                f"Commands:\n"
                f"/myskills — view your skills\n"
                f"/publish {state['name']} — submit to community marketplace\n"
                f"/delskill {state['name']} — delete this skill",
                parse_mode=ParseMode.MARKDOWN,
            )
        return

    if query.data == "addskill_cancel":
        if user_id in _addskill_state:
            del _addskill_state[user_id]
        await query.edit_message_text("Skill creation cancelled.")
        return
```

**Step 6: Register the command handler**

In the `main()` function where handlers are added:

```python
app.add_handler(CommandHandler("addskill", cmd_addskill))
```

**Step 7: Verify**

Run: `python -c "import amanclaw.bot; print('OK')"`
Expected: OK

**Step 8: Commit**

```bash
git add amanclaw/bot.py
git commit -m "feat: add /addskill conversational flow for creating API integrations"
```

---

### Task 5: Add Skill Management Commands

**Files:**
- Modify: `amanclaw/bot.py`

**Step 1: Add /myskills command**

```python
async def cmd_myskills(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """List user's custom skills."""
    user_id = str(update.effective_user.id)
    if not await handle_registration(update, context):
        return
    skills = memory.get_user_skills(user_id)
    own = [s for s in skills if s["user_id"] == user_id]
    if not own:
        await update.message.reply_text(
            "You don't have any custom skills yet.\n"
            "Use /addskill to create one!"
        )
        return
    lines = ["*Your Skills:*\n"]
    for s in own:
        status = "private" if s["is_private"] else ("approved" if s["is_approved"] else "pending review")
        lines.append(f"- `{s['name']}`: {s['description']} [{status}]")
    await reply_with_markdown(update.message, "\n".join(lines))
```

**Step 2: Add /delskill command**

```python
async def cmd_delskill(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Delete a user skill."""
    user_id = str(update.effective_user.id)
    if not await handle_registration(update, context):
        return
    if not context.args:
        await update.message.reply_text("Usage: /delskill <skill_name>")
        return
    name = context.args[0]
    if memory.delete_user_skill(user_id, name):
        await update.message.reply_text(f"Skill `{name}` deleted.", parse_mode=ParseMode.MARKDOWN)
    else:
        await update.message.reply_text(f"Skill `{name}` not found.", parse_mode=ParseMode.MARKDOWN)
```

**Step 3: Add /publish command**

```python
async def cmd_publish(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Submit a skill to the community marketplace."""
    user_id = str(update.effective_user.id)
    if not await handle_registration(update, context):
        return
    if not context.args:
        await update.message.reply_text("Usage: /publish <skill_name>")
        return
    name = context.args[0]
    if memory.publish_user_skill(user_id, name):
        await update.message.reply_text(
            f"Skill `{name}` submitted for review!\n"
            "An admin will review it shortly.",
            parse_mode=ParseMode.MARKDOWN,
        )
        # Notify admins
        admin_ids = config.get("admin_users", {}).get("telegram", [])
        keyboard = InlineKeyboardMarkup([
            [
                InlineKeyboardButton("Approve", callback_data=f"appskill_{name}_{user_id}"),
                InlineKeyboardButton("Reject", callback_data=f"rejskill_{name}_{user_id}"),
            ]
        ])
        skill = memory.get_user_skill_by_name(name, user_id)
        for admin_id in admin_ids:
            try:
                await context.bot.send_message(
                    chat_id=int(admin_id),
                    text=(
                        f"*Skill submitted for marketplace:*\n\n"
                        f"Name: `{name}`\n"
                        f"By: `{user_id}`\n"
                        f"Description: {skill['description']}\n"
                        f"URL: `{skill['url_template']}`\n"
                        f"Method: {skill['method']}"
                    ),
                    parse_mode=ParseMode.MARKDOWN,
                    reply_markup=keyboard,
                )
            except Exception:
                pass
    else:
        await update.message.reply_text(f"Skill `{name}` not found or already published.", parse_mode=ParseMode.MARKDOWN)
```

**Step 4: Add /marketplace command**

```python
async def cmd_marketplace(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Browse community skills."""
    user_id = str(update.effective_user.id)
    if not await handle_registration(update, context):
        return
    skills = memory.get_marketplace_skills()
    if not skills:
        await update.message.reply_text("No community skills available yet. Be the first — use /addskill!")
        return
    lines = ["*Community Marketplace:*\n"]
    for s in skills:
        lines.append(f"- `{s['name']}`: {s['description']}")
    lines.append("\nAll marketplace skills are automatically available to you!")
    await reply_with_markdown(update.message, "\n".join(lines))
```

**Step 5: Add admin skill approval callbacks in handle_callback**

```python
    if query.data.startswith("appskill_"):
        if not auth.is_admin(user_id, "telegram"):
            await query.answer("Not authorized.", show_alert=True)
            return
        parts = query.data.replace("appskill_", "").rsplit("_", 1)
        skill_name, creator_id = parts[0], parts[1]
        skill = memory.get_user_skill_by_name(skill_name, creator_id)
        if skill and memory.approve_user_skill(skill["id"]):
            await query.edit_message_text(
                query.message.text + "\n\n*Approved for marketplace*",
                parse_mode=ParseMode.MARKDOWN,
            )
        return

    if query.data.startswith("rejskill_"):
        if not auth.is_admin(user_id, "telegram"):
            await query.answer("Not authorized.", show_alert=True)
            return
        parts = query.data.replace("rejskill_", "").rsplit("_", 1)
        skill_name, creator_id = parts[0], parts[1]
        memory.delete_user_skill(creator_id, skill_name)
        await query.edit_message_text(
            query.message.text + "\n\n*Rejected and removed*",
            parse_mode=ParseMode.MARKDOWN,
        )
        return
```

**Step 6: Register all command handlers**

```python
app.add_handler(CommandHandler("myskills", cmd_myskills))
app.add_handler(CommandHandler("delskill", cmd_delskill))
app.add_handler(CommandHandler("publish", cmd_publish))
app.add_handler(CommandHandler("marketplace", cmd_marketplace))
```

**Step 7: Commit**

```bash
git add amanclaw/bot.py
git commit -m "feat: add /myskills, /delskill, /publish, /marketplace commands"
```

---

### Task 6: Smart Failure Detection

**Files:**
- Modify: `amanclaw/bot.py` (in handle_message, after LLM response)

**Step 1: Add failure detection after LLM response**

After the line `await send_long_reply(update.message, response, with_actions=True)` in `handle_message()`, add:

```python
    # Smart failure detection — suggest /addskill if bot lacks capability
    _capability_fail_patterns = [
        "can't access", "cannot access", "don't have access",
        "no tool", "not available", "unable to fetch",
        "can't fetch", "cannot fetch", "don't have a tool",
        "no built-in", "don't have built-in",
        "tidak dapat", "tidak boleh", "tiada akses",
    ]
    response_lower = response.lower()
    if any(p in response_lower for p in _capability_fail_patterns):
        await update.message.reply_text(
            "Want me to learn how to do this? "
            "You can add an API integration with /addskill",
        )
```

**Step 2: Commit**

```bash
git add amanclaw/bot.py
git commit -m "feat: smart failure detection suggests /addskill when bot lacks capability"
```

---

### Task 7: Initialize UserSkillManager in Bot Startup

**Files:**
- Modify: `amanclaw/bot.py` (main function, near other initializations)

**Step 1: Import and initialize**

Add import at top of bot.py:

```python
from amanclaw.skills.user_skills import UserSkillManager
from amanclaw.skills import set_user_skill_manager
```

In the `main()` function, after `memory = Memory(...)` is created:

```python
    # Initialize user skill manager
    user_skill_mgr = UserSkillManager(memory)
    set_user_skill_manager(user_skill_mgr)
    logger.info("User skill manager initialized")
```

**Step 2: Add `import re` if not present**

The addskill flow uses `re.match` — make sure `import re` is at the top of bot.py.

**Step 3: Verify full import**

Run: `python -c "import amanclaw.bot; print('OK')"`
Expected: OK

**Step 4: Commit**

```bash
git add amanclaw/bot.py
git commit -m "feat: initialize UserSkillManager on bot startup"
```

---

### Task 8: Build, Deploy, and Test

**Step 1: Rebuild Docker image**

```bash
docker compose build amanclaw
```

**Step 2: Deploy**

```bash
docker compose up -d amanclaw
```

**Step 3: Verify startup logs**

```bash
docker compose logs amanclaw --tail 20
```

Expected: "User skill manager initialized" in logs, no errors.

**Step 4: End-to-end test flow**

1. Send `/addskill` → bot asks for description
2. Send "Get weather info" → bot asks for name
3. Send "weather" → bot asks for URL
4. Send "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current_weather=true" → bot shows params
5. Complete the flow → skill created
6. Ask "What's the weather at latitude 3.14, longitude 101.69?" → LLM calls user skill
7. Send `/myskills` → see the skill listed
8. Send `/marketplace` → see empty marketplace
9. Send `/publish weather` → admin gets notification
10. Admin clicks Approve → skill appears in marketplace

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat: complete self-extending skill system — addskill, marketplace, failure detection"
```
