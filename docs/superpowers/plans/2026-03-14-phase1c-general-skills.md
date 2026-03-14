# Plan 1C: General Skills (Batch 1) — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 15 general-purpose Python skills across developer, research, and productivity categories, making AmanClaw useful beyond Islamic-specific tasks.

**Architecture:** All skills are Python scripts using the existing `amanclaw_sdk` decorator pattern. Each communicates via JSON over stdin/stdout. No Rust changes needed — just new `.py` files in `plugins/` and config entries.

**Tech Stack:** Python 3, amanclaw_sdk, stdlib + minimal pip packages

---

## File Structure

All new files go in `plugins/`. Config entries go in `config.yaml` under `script_plugins`.

| File | Category | Dependencies |
|------|----------|-------------|
| `plugins/skill_web_search.py` | Research | `urllib` (stdlib) |
| `plugins/skill_summarize.py` | Research | None (LLM-based) |
| `plugins/skill_translate.py` | Research | `urllib` (stdlib) |
| `plugins/skill_url_reader.py` | Research | `urllib` (stdlib) |
| `plugins/skill_datetime.py` | Productivity | `datetime` (stdlib) |
| `plugins/skill_unit_convert.py` | Productivity | None (math) |
| `plugins/skill_reminder.py` | Productivity | `json`, `os` (stdlib) |
| `plugins/skill_todo.py` | Productivity | `json`, `os` (stdlib) |
| `plugins/skill_weather.py` | Productivity | `urllib` (stdlib) |
| `plugins/skill_json_tool.py` | Developer | `json` (stdlib) |
| `plugins/skill_base64.py` | Developer | `base64` (stdlib) |
| `plugins/skill_hash.py` | Developer | `hashlib` (stdlib) |
| `plugins/skill_regex.py` | Developer | `re` (stdlib) |
| `plugins/skill_http_client.py` | Developer | `urllib` (stdlib) |
| `plugins/skill_csv_tool.py` | Developer | `csv` (stdlib) |

---

## Pattern Reference

Every skill follows this exact template (from existing plugins):

```python
#!/usr/bin/env python3
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="skill_name",
    description="What this skill does",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "description": "Operation", "enum": ["op1", "op2"]},
            "input": {"type": "string", "description": "Input data"}
        },
        "required": ["action"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "default")
    try:
        # Logic here
        return SkillResult.ok("result")
    except Exception as e:
        return SkillResult.err(f"Failed: {e}")

if __name__ == "__main__":
    execute.run()
```

---

## Chunk 1: Research Skills (4 skills)

### Task 1: Web Search skill

**Files:**
- Create: `plugins/skill_web_search.py`

- [ ] **Step 1: Create the skill**

Uses DuckDuckGo HTML API (no API key needed):

```python
#!/usr/bin/env python3
"""Web search using DuckDuckGo (no API key required)."""
import urllib.request
import urllib.parse
import json
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="web_search",
    description="Search the web using DuckDuckGo. Returns top results with titles and URLs.",
    parameters={
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "Search query"},
            "max_results": {"type": "integer", "description": "Max results (default 5)"}
        },
        "required": ["query"]
    },
    timeout_ms=15000
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    query = args.get("query", "")
    max_results = args.get("max_results", 5)

    if not query:
        return SkillResult.err("Please provide a search query.")

    try:
        url = f"https://api.duckduckgo.com/?q={urllib.parse.quote(query)}&format=json&no_html=1"
        req = urllib.request.Request(url, headers={"User-Agent": "AmanClaw/1.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode())

        results = []
        # Abstract (instant answer)
        if data.get("Abstract"):
            results.append(f"**Summary:** {data['Abstract']}\nSource: {data.get('AbstractURL', '')}")

        # Related topics
        for topic in data.get("RelatedTopics", [])[:max_results]:
            if "Text" in topic:
                text = topic["Text"]
                url = topic.get("FirstURL", "")
                results.append(f"- {text}\n  {url}")

        if not results:
            return SkillResult.ok(f"No results found for '{query}'. Try a different search term.")

        return SkillResult.ok("\n\n".join(results))
    except Exception as e:
        return SkillResult.err(f"Search failed: {e}")

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Add config entry**

Add to `config.yaml` under `script_plugins`:
```yaml
  web_search:
    command: "python3"
    args: ["plugins/skill_web_search.py"]
    env: {}
```

- [ ] **Step 3: Test manually**

Run: `cd rust && cargo run -- dev` then send a message: "search for Rust programming language"
Expected: Returns DuckDuckGo results

- [ ] **Step 4: Commit**

```bash
git add plugins/skill_web_search.py config.yaml
git commit -m "feat(skills): add web_search skill (DuckDuckGo)"
```

---

### Task 2: URL Reader skill

- [ ] **Step 1: Create the skill**

Create `plugins/skill_url_reader.py`:

```python
#!/usr/bin/env python3
"""Fetch and extract text content from a URL."""
import urllib.request
import html
import re
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="url_reader",
    description="Fetch a URL and extract its text content. Useful for reading web pages, APIs, or raw text files.",
    parameters={
        "type": "object",
        "properties": {
            "url": {"type": "string", "description": "URL to fetch"},
            "max_chars": {"type": "integer", "description": "Max characters to return (default 3000)"}
        },
        "required": ["url"]
    },
    timeout_ms=15000
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    url = args.get("url", "")
    max_chars = args.get("max_chars", 3000)

    if not url:
        return SkillResult.err("Please provide a URL.")
    if not url.startswith(("http://", "https://")):
        return SkillResult.err("URL must start with http:// or https://")

    try:
        req = urllib.request.Request(url, headers={"User-Agent": "AmanClaw/1.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            content_type = resp.headers.get("Content-Type", "")
            raw = resp.read().decode("utf-8", errors="replace")

        if "html" in content_type:
            # Strip HTML tags and decode entities
            text = re.sub(r"<script[^>]*>.*?</script>", "", raw, flags=re.DOTALL)
            text = re.sub(r"<style[^>]*>.*?</style>", "", text, flags=re.DOTALL)
            text = re.sub(r"<[^>]+>", " ", text)
            text = html.unescape(text)
            text = re.sub(r"\s+", " ", text).strip()
        else:
            text = raw.strip()

        if len(text) > max_chars:
            text = text[:max_chars] + f"\n\n[Truncated at {max_chars} characters]"

        return SkillResult.ok(f"Content from {url}:\n\n{text}")
    except Exception as e:
        return SkillResult.err(f"Failed to fetch URL: {e}")

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Add config + commit**

```bash
git add plugins/skill_url_reader.py
git commit -m "feat(skills): add url_reader skill"
```

---

### Task 3: Summarize skill

- [ ] **Step 1: Create the skill**

Create `plugins/skill_summarize.py`:

```python
#!/usr/bin/env python3
"""Summarize text content. Works best when the LLM uses this for long inputs."""
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="summarize",
    description="Summarize long text into key points. Provide the text and desired length.",
    parameters={
        "type": "object",
        "properties": {
            "text": {"type": "string", "description": "Text to summarize"},
            "style": {"type": "string", "description": "Style: bullets, paragraph, tldr", "enum": ["bullets", "paragraph", "tldr"]}
        },
        "required": ["text"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    text = args.get("text", "")
    style = args.get("style", "bullets")

    if not text:
        return SkillResult.err("Please provide text to summarize.")

    # This skill returns the text back for the LLM to summarize
    # The LLM will use its own capabilities to create the summary
    word_count = len(text.split())
    return SkillResult.ok(
        f"[Summarize request — {style} style, {word_count} words]\n\n{text}"
    )

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_summarize.py
git commit -m "feat(skills): add summarize skill"
```

---

### Task 4: Translate skill

- [ ] **Step 1: Create the skill**

Create `plugins/skill_translate.py`:

```python
#!/usr/bin/env python3
"""Translation helper — structures translation requests for the LLM."""
from amanclaw_sdk import plugin, SkillInput, SkillResult

SUPPORTED_LANGS = [
    "en", "ms", "ar", "id", "tr", "ur", "zh", "ja", "ko",
    "fr", "de", "es", "pt", "ru", "hi", "bn", "ta"
]

@plugin(
    name="translate",
    description="Translate text between languages. Supports: English, Malay, Arabic, Indonesian, Turkish, Urdu, Chinese, Japanese, Korean, French, German, Spanish, Portuguese, Russian, Hindi, Bengali, Tamil.",
    parameters={
        "type": "object",
        "properties": {
            "text": {"type": "string", "description": "Text to translate"},
            "target_lang": {"type": "string", "description": "Target language code (e.g., 'ms', 'ar', 'en')"},
            "source_lang": {"type": "string", "description": "Source language (auto-detect if omitted)"}
        },
        "required": ["text", "target_lang"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    text = args.get("text", "")
    target = args.get("target_lang", "")
    source = args.get("source_lang", "auto")

    if not text:
        return SkillResult.err("Please provide text to translate.")
    if not target:
        return SkillResult.err(f"Please specify target language. Supported: {', '.join(SUPPORTED_LANGS)}")

    # The LLM handles the actual translation
    return SkillResult.ok(
        f"[Translation request: {source} → {target}]\n\n{text}"
    )

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_translate.py
git commit -m "feat(skills): add translate skill"
```

---

## Chunk 2: Productivity Skills (5 skills)

### Task 5: DateTime skill

- [ ] **Step 1: Create `plugins/skill_datetime.py`**

```python
#!/usr/bin/env python3
"""Date, time, and timezone utilities."""
from datetime import datetime, timezone, timedelta
from amanclaw_sdk import plugin, SkillInput, SkillResult

TIMEZONES = {
    "MYT": 8, "SGT": 8, "WIB": 7, "WITA": 8, "WIT": 9,
    "JST": 9, "KST": 9, "CST": 8, "IST": 5.5, "AST": 3,
    "UTC": 0, "GMT": 0, "EST": -5, "PST": -8, "CET": 1,
}

@plugin(
    name="datetime_tool",
    description="Get current date/time, convert between timezones, calculate date differences.",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["now", "convert", "diff"], "description": "Operation"},
            "timezone": {"type": "string", "description": "Timezone (e.g., MYT, UTC, JST)"},
            "from_tz": {"type": "string", "description": "Source timezone for convert"},
            "to_tz": {"type": "string", "description": "Target timezone for convert"},
            "time": {"type": "string", "description": "Time string (HH:MM) for convert"},
            "date1": {"type": "string", "description": "First date (YYYY-MM-DD) for diff"},
            "date2": {"type": "string", "description": "Second date (YYYY-MM-DD) for diff"}
        },
        "required": ["action"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "now")

    try:
        if action == "now":
            tz_name = args.get("timezone", "MYT").upper()
            offset = TIMEZONES.get(tz_name, 0)
            tz = timezone(timedelta(hours=offset))
            now = datetime.now(tz)
            return SkillResult.ok(
                f"Current time ({tz_name}): {now.strftime('%Y-%m-%d %H:%M:%S %Z')}\n"
                f"Day: {now.strftime('%A')}\n"
                f"Unix timestamp: {int(now.timestamp())}"
            )
        elif action == "convert":
            from_tz = args.get("from_tz", "UTC").upper()
            to_tz = args.get("to_tz", "MYT").upper()
            time_str = args.get("time", "12:00")
            h, m = map(int, time_str.split(":"))
            from_offset = TIMEZONES.get(from_tz, 0)
            to_offset = TIMEZONES.get(to_tz, 0)
            diff = to_offset - from_offset
            result_h = (h + diff) % 24
            return SkillResult.ok(f"{time_str} {from_tz} = {int(result_h):02d}:{m:02d} {to_tz}")
        elif action == "diff":
            d1 = datetime.strptime(args.get("date1", ""), "%Y-%m-%d")
            d2 = datetime.strptime(args.get("date2", ""), "%Y-%m-%d")
            delta = abs((d2 - d1).days)
            return SkillResult.ok(f"Difference: {delta} days ({delta // 7} weeks, {delta // 30} months approx)")
        else:
            return SkillResult.err(f"Unknown action: {action}")
    except Exception as e:
        return SkillResult.err(f"DateTime error: {e}")

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_datetime.py
git commit -m "feat(skills): add datetime_tool skill"
```

---

### Task 6: Unit Converter skill

- [ ] **Step 1: Create `plugins/skill_unit_convert.py`**

```python
#!/usr/bin/env python3
"""Unit conversion: length, weight, temperature, currency concepts."""
from amanclaw_sdk import plugin, SkillInput, SkillResult

CONVERSIONS = {
    "km_to_mi": 0.621371, "mi_to_km": 1.60934,
    "kg_to_lb": 2.20462, "lb_to_kg": 0.453592,
    "m_to_ft": 3.28084, "ft_to_m": 0.3048,
    "cm_to_in": 0.393701, "in_to_cm": 2.54,
    "l_to_gal": 0.264172, "gal_to_l": 3.78541,
    "g_to_oz": 0.035274, "oz_to_g": 28.3495,
}

@plugin(
    name="unit_convert",
    description="Convert between units: length (km/mi/m/ft/cm/in), weight (kg/lb/g/oz), volume (l/gal), temperature (C/F/K).",
    parameters={
        "type": "object",
        "properties": {
            "value": {"type": "number", "description": "Value to convert"},
            "from_unit": {"type": "string", "description": "Source unit"},
            "to_unit": {"type": "string", "description": "Target unit"}
        },
        "required": ["value", "from_unit", "to_unit"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    value = args.get("value", 0)
    from_u = args.get("from_unit", "").lower()
    to_u = args.get("to_unit", "").lower()

    try:
        # Temperature special cases
        if from_u == "c" and to_u == "f":
            result = value * 9/5 + 32
        elif from_u == "f" and to_u == "c":
            result = (value - 32) * 5/9
        elif from_u == "c" and to_u == "k":
            result = value + 273.15
        elif from_u == "k" and to_u == "c":
            result = value - 273.15
        else:
            key = f"{from_u}_to_{to_u}"
            if key not in CONVERSIONS:
                return SkillResult.err(f"Unknown conversion: {from_u} → {to_u}")
            result = value * CONVERSIONS[key]

        return SkillResult.ok(f"{value} {from_u} = {result:.4f} {to_u}")
    except Exception as e:
        return SkillResult.err(f"Conversion error: {e}")

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_unit_convert.py
git commit -m "feat(skills): add unit_convert skill"
```

---

### Task 7: Todo skill

- [ ] **Step 1: Create `plugins/skill_todo.py`**

```python
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
                status = "✓" if t["done"] else "○"
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
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_todo.py
git commit -m "feat(skills): add todo skill with persistent storage"
```

---

### Task 8: Weather skill

- [ ] **Step 1: Create `plugins/skill_weather.py`**

Uses Open-Meteo API (free, no key):

```python
#!/usr/bin/env python3
"""Weather using Open-Meteo API (free, no API key)."""
import urllib.request
import urllib.parse
import json
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="weather",
    description="Get current weather and forecast for any city. No API key needed.",
    parameters={
        "type": "object",
        "properties": {
            "city": {"type": "string", "description": "City name (e.g., 'Kuala Lumpur', 'London')"},
            "days": {"type": "integer", "description": "Forecast days (1-7, default 1)"}
        },
        "required": ["city"]
    },
    timeout_ms=15000
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    city = args.get("city", "")
    days = min(args.get("days", 1), 7)

    if not city:
        return SkillResult.err("Please provide a city name.")

    try:
        # Geocode city
        geo_url = f"https://geocoding-api.open-meteo.com/v1/search?name={urllib.parse.quote(city)}&count=1"
        req = urllib.request.Request(geo_url, headers={"User-Agent": "AmanClaw/1.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            geo = json.loads(resp.read().decode())

        if not geo.get("results"):
            return SkillResult.err(f"City not found: {city}")

        loc = geo["results"][0]
        lat, lon = loc["latitude"], loc["longitude"]
        name = loc.get("name", city)
        country = loc.get("country", "")

        # Get weather
        wx_url = (
            f"https://api.open-meteo.com/v1/forecast?"
            f"latitude={lat}&longitude={lon}"
            f"&current=temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code"
            f"&daily=temperature_2m_max,temperature_2m_min,weather_code"
            f"&forecast_days={days}&timezone=auto"
        )
        req = urllib.request.Request(wx_url, headers={"User-Agent": "AmanClaw/1.0"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            wx = json.loads(resp.read().decode())

        current = wx.get("current", {})
        temp = current.get("temperature_2m", "?")
        humidity = current.get("relative_humidity_2m", "?")
        wind = current.get("wind_speed_10m", "?")

        output = f"Weather for {name}, {country}:\n\n"
        output += f"Now: {temp}°C, Humidity {humidity}%, Wind {wind} km/h\n"

        daily = wx.get("daily", {})
        dates = daily.get("time", [])
        maxes = daily.get("temperature_2m_max", [])
        mins = daily.get("temperature_2m_min", [])

        if dates:
            output += "\nForecast:\n"
            for i, date in enumerate(dates):
                output += f"  {date}: {mins[i]}°C – {maxes[i]}°C\n"

        return SkillResult.ok(output)
    except Exception as e:
        return SkillResult.err(f"Weather error: {e}")

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_weather.py
git commit -m "feat(skills): add weather skill (Open-Meteo, no API key)"
```

---

### Task 9: Reminder skill

- [ ] **Step 1: Create `plugins/skill_reminder.py`**

```python
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
                lines.append(f"{i}. {r['message']} — {r['when']}")
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
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_reminder.py
git commit -m "feat(skills): add reminder skill with persistent storage"
```

---

## Chunk 3: Developer Skills (6 skills)

### Task 10: JSON Tool skill

- [ ] **Step 1: Create `plugins/skill_json_tool.py`**

```python
#!/usr/bin/env python3
"""JSON utilities: format, validate, query, minify."""
import json
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="json_tool",
    description="JSON utilities: format/prettify, validate, minify, extract fields.",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["format", "validate", "minify", "extract"], "description": "Operation"},
            "data": {"type": "string", "description": "JSON string to process"},
            "path": {"type": "string", "description": "Dot-notation path for extract (e.g., 'user.name')"}
        },
        "required": ["action", "data"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "format")
    data = args.get("data", "")

    try:
        if action == "validate":
            json.loads(data)
            return SkillResult.ok("Valid JSON.")
        elif action == "format":
            parsed = json.loads(data)
            return SkillResult.ok(json.dumps(parsed, indent=2))
        elif action == "minify":
            parsed = json.loads(data)
            return SkillResult.ok(json.dumps(parsed, separators=(",", ":")))
        elif action == "extract":
            parsed = json.loads(data)
            path = args.get("path", "")
            for key in path.split("."):
                if isinstance(parsed, dict):
                    parsed = parsed.get(key)
                elif isinstance(parsed, list) and key.isdigit():
                    parsed = parsed[int(key)]
                else:
                    return SkillResult.err(f"Cannot navigate path: {path}")
            return SkillResult.ok(json.dumps(parsed, indent=2) if parsed is not None else "null")
        return SkillResult.err(f"Unknown action: {action}")
    except json.JSONDecodeError as e:
        return SkillResult.err(f"Invalid JSON: {e}")
    except Exception as e:
        return SkillResult.err(f"JSON error: {e}")

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_json_tool.py
git commit -m "feat(skills): add json_tool skill"
```

---

### Task 11: Base64 skill

- [ ] **Step 1: Create `plugins/skill_base64.py`**

```python
#!/usr/bin/env python3
"""Base64 encoding and decoding."""
import base64
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="base64_tool",
    description="Encode or decode Base64 strings.",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["encode", "decode"], "description": "Operation"},
            "data": {"type": "string", "description": "Data to encode/decode"}
        },
        "required": ["action", "data"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "encode")
    data = args.get("data", "")

    try:
        if action == "encode":
            result = base64.b64encode(data.encode()).decode()
            return SkillResult.ok(result)
        elif action == "decode":
            result = base64.b64decode(data).decode()
            return SkillResult.ok(result)
        return SkillResult.err(f"Unknown action: {action}")
    except Exception as e:
        return SkillResult.err(f"Base64 error: {e}")

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_base64.py
git commit -m "feat(skills): add base64_tool skill"
```

---

### Task 12: Hash skill

- [ ] **Step 1: Create `plugins/skill_hash.py`**

```python
#!/usr/bin/env python3
"""Hash text with various algorithms."""
import hashlib
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="hash_tool",
    description="Hash text using MD5, SHA1, SHA256, SHA512.",
    parameters={
        "type": "object",
        "properties": {
            "text": {"type": "string", "description": "Text to hash"},
            "algorithm": {"type": "string", "enum": ["md5", "sha1", "sha256", "sha512"], "description": "Hash algorithm (default: sha256)"}
        },
        "required": ["text"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    text = args.get("text", "")
    algo = args.get("algorithm", "sha256")

    try:
        h = hashlib.new(algo, text.encode())
        return SkillResult.ok(f"{algo}: {h.hexdigest()}")
    except Exception as e:
        return SkillResult.err(f"Hash error: {e}")

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_hash.py
git commit -m "feat(skills): add hash_tool skill"
```

---

### Task 13: Regex skill

- [ ] **Step 1: Create `plugins/skill_regex.py`**

```python
#!/usr/bin/env python3
"""Regex testing and extraction."""
import re
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="regex_tool",
    description="Test regex patterns, find matches, and extract groups from text.",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["test", "find_all", "replace"], "description": "Operation"},
            "pattern": {"type": "string", "description": "Regex pattern"},
            "text": {"type": "string", "description": "Text to search"},
            "replacement": {"type": "string", "description": "Replacement string (for replace action)"}
        },
        "required": ["action", "pattern", "text"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "test")
    pattern = args.get("pattern", "")
    text = args.get("text", "")

    try:
        if action == "test":
            match = re.search(pattern, text)
            if match:
                return SkillResult.ok(f"Match found: '{match.group()}' at position {match.start()}-{match.end()}")
            return SkillResult.ok("No match found.")
        elif action == "find_all":
            matches = re.findall(pattern, text)
            if matches:
                return SkillResult.ok(f"Found {len(matches)} match(es):\n" + "\n".join(f"  - {m}" for m in matches))
            return SkillResult.ok("No matches found.")
        elif action == "replace":
            replacement = args.get("replacement", "")
            result = re.sub(pattern, replacement, text)
            return SkillResult.ok(result)
        return SkillResult.err(f"Unknown action: {action}")
    except re.error as e:
        return SkillResult.err(f"Invalid regex: {e}")
    except Exception as e:
        return SkillResult.err(f"Regex error: {e}")

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_regex.py
git commit -m "feat(skills): add regex_tool skill"
```

---

### Task 14: HTTP Client skill

- [ ] **Step 1: Create `plugins/skill_http_client.py`**

```python
#!/usr/bin/env python3
"""Make HTTP requests (GET, POST, etc.)."""
import urllib.request
import json
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="http_client",
    description="Make HTTP requests. Supports GET, POST, PUT, DELETE with headers and body.",
    parameters={
        "type": "object",
        "properties": {
            "method": {"type": "string", "enum": ["GET", "POST", "PUT", "DELETE"], "description": "HTTP method"},
            "url": {"type": "string", "description": "URL to request"},
            "headers": {"type": "object", "description": "Request headers (key-value pairs)"},
            "body": {"type": "string", "description": "Request body (for POST/PUT)"}
        },
        "required": ["url"]
    },
    timeout_ms=15000
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    url = args.get("url", "")
    method = args.get("method", "GET")
    headers = args.get("headers", {})
    body = args.get("body")

    if not url:
        return SkillResult.err("Please provide a URL.")
    if not url.startswith(("http://", "https://")):
        return SkillResult.err("URL must start with http:// or https://")

    try:
        data = body.encode() if body else None
        req = urllib.request.Request(url, data=data, method=method)
        req.add_header("User-Agent", "AmanClaw/1.0")
        for k, v in headers.items():
            req.add_header(k, v)

        with urllib.request.urlopen(req, timeout=10) as resp:
            status = resp.status
            resp_headers = dict(resp.headers)
            content = resp.read().decode("utf-8", errors="replace")

        # Try to pretty-print JSON
        try:
            parsed = json.loads(content)
            content = json.dumps(parsed, indent=2)
        except (json.JSONDecodeError, ValueError):
            pass

        if len(content) > 3000:
            content = content[:3000] + "\n[Truncated]"

        return SkillResult.ok(f"HTTP {status}\n\n{content}")
    except Exception as e:
        return SkillResult.err(f"HTTP error: {e}")

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_http_client.py
git commit -m "feat(skills): add http_client skill"
```

---

### Task 15: CSV Tool skill

- [ ] **Step 1: Create `plugins/skill_csv_tool.py`**

```python
#!/usr/bin/env python3
"""CSV parsing, formatting, and analysis."""
import csv
import io
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="csv_tool",
    description="Parse CSV data, convert to table, get stats, extract columns.",
    parameters={
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["parse", "stats", "column", "to_json"], "description": "Operation"},
            "data": {"type": "string", "description": "CSV data as string"},
            "column": {"type": "string", "description": "Column name (for column/stats)"}
        },
        "required": ["action", "data"]
    }
)
def execute(inp: SkillInput) -> SkillResult:
    args = inp.parse_args()
    action = args.get("action", "parse")
    data = args.get("data", "")

    try:
        reader = csv.DictReader(io.StringIO(data))
        rows = list(reader)

        if not rows:
            return SkillResult.err("No data found in CSV.")

        if action == "parse":
            headers = list(rows[0].keys())
            output = f"Columns: {', '.join(headers)}\nRows: {len(rows)}\n\n"
            for i, row in enumerate(rows[:10]):
                output += f"Row {i+1}: {dict(row)}\n"
            if len(rows) > 10:
                output += f"\n... and {len(rows) - 10} more rows"
            return SkillResult.ok(output)

        elif action == "stats":
            col = args.get("column", "")
            if col and col in rows[0]:
                values = [r[col] for r in rows if r.get(col)]
                try:
                    nums = [float(v) for v in values]
                    avg = sum(nums) / len(nums)
                    return SkillResult.ok(
                        f"Column '{col}': {len(nums)} values\n"
                        f"Min: {min(nums)}, Max: {max(nums)}, Avg: {avg:.2f}, Sum: {sum(nums)}"
                    )
                except ValueError:
                    unique = len(set(values))
                    return SkillResult.ok(f"Column '{col}': {len(values)} values, {unique} unique")
            return SkillResult.ok(f"Available columns: {', '.join(rows[0].keys())}")

        elif action == "column":
            col = args.get("column", "")
            if col not in rows[0]:
                return SkillResult.err(f"Column '{col}' not found. Available: {', '.join(rows[0].keys())}")
            values = [r[col] for r in rows]
            return SkillResult.ok("\n".join(values))

        elif action == "to_json":
            import json
            return SkillResult.ok(json.dumps(rows, indent=2))

        return SkillResult.err(f"Unknown action: {action}")
    except Exception as e:
        return SkillResult.err(f"CSV error: {e}")

if __name__ == "__main__":
    execute.run()
```

- [ ] **Step 2: Commit**

```bash
git add plugins/skill_csv_tool.py
git commit -m "feat(skills): add csv_tool skill"
```

---

## Chunk 4: Config Update

### Task 16: Register all new skills in config

- [ ] **Step 1: Add all 15 skills to config.yaml**

Add under `script_plugins`:

```yaml
  # General — Research
  web_search:
    command: "python3"
    args: ["plugins/skill_web_search.py"]
    env: {}
  url_reader:
    command: "python3"
    args: ["plugins/skill_url_reader.py"]
    env: {}
  summarize:
    command: "python3"
    args: ["plugins/skill_summarize.py"]
    env: {}
  translate:
    command: "python3"
    args: ["plugins/skill_translate.py"]
    env: {}
  # General — Productivity
  datetime_tool:
    command: "python3"
    args: ["plugins/skill_datetime.py"]
    env: {}
  unit_convert:
    command: "python3"
    args: ["plugins/skill_unit_convert.py"]
    env: {}
  todo:
    command: "python3"
    args: ["plugins/skill_todo.py"]
    env: {}
  weather:
    command: "python3"
    args: ["plugins/skill_weather.py"]
    env: {}
  reminder:
    command: "python3"
    args: ["plugins/skill_reminder.py"]
    env: {}
  # General — Developer
  json_tool:
    command: "python3"
    args: ["plugins/skill_json_tool.py"]
    env: {}
  base64_tool:
    command: "python3"
    args: ["plugins/skill_base64.py"]
    env: {}
  hash_tool:
    command: "python3"
    args: ["plugins/skill_hash.py"]
    env: {}
  regex_tool:
    command: "python3"
    args: ["plugins/skill_regex.py"]
    env: {}
  http_client:
    command: "python3"
    args: ["plugins/skill_http_client.py"]
    env: {}
  csv_tool:
    command: "python3"
    args: ["plugins/skill_csv_tool.py"]
    env: {}
```

- [ ] **Step 2: Update config.example.yaml too**

- [ ] **Step 3: Commit**

```bash
git add config.yaml config.example.yaml
git commit -m "feat(config): register 15 general-purpose skills"
```

---

## Summary

| Task | Skill | Category | API Key? |
|------|-------|----------|----------|
| 1 | web_search | Research | No |
| 2 | url_reader | Research | No |
| 3 | summarize | Research | No |
| 4 | translate | Research | No |
| 5 | datetime_tool | Productivity | No |
| 6 | unit_convert | Productivity | No |
| 7 | todo | Productivity | No |
| 8 | weather | Productivity | No |
| 9 | reminder | Productivity | No |
| 10 | json_tool | Developer | No |
| 11 | base64_tool | Developer | No |
| 12 | hash_tool | Developer | No |
| 13 | regex_tool | Developer | No |
| 14 | http_client | Developer | No |
| 15 | csv_tool | Developer | No |
| 16 | Config registration | — | — |

**Total: 16 tasks, 32 steps**

All 15 skills use **zero API keys** (DuckDuckGo and Open-Meteo are free). This means they work immediately on any installation without configuration.
