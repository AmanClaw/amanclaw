# Smart Memory System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the flat key/value facts store with a SQLite knowledge graph supporting entities, relationships, temporal facts, and FTS5 search, with automatic LLM-powered knowledge extraction.

**Architecture:** Three new tables (`knowledge`, `entities`, `relationships`) plus an FTS5 index in the existing SQLite database. Background async extraction after each exchange. Existing `facts` table migrated on first run, then left read-only.

**Tech Stack:** Python 3.12, SQLite FTS5, asyncio, existing aiohttp LLM client.

---

### Task 1: Knowledge Graph Schema & Migration

**Files:**
- Modify: `amanclaw/memory.py` (add new tables to `_init_tables`, add migration logic)
- Test: `tests/test_skills.py` (add knowledge graph tests to `TestMemory` class)

**Step 1: Write failing tests for knowledge CRUD**

Add to `tests/test_skills.py` after the existing `TestMemory` class:

```python
class TestKnowledgeGraph:
    @pytest.fixture
    def memory(self):
        from amanclaw.memory import Memory
        m = Memory(":memory:")
        yield m
        m.close()

    def test_save_and_get_knowledge(self, memory):
        memory.save_knowledge("user1", category="preference", subject="coffee",
                              content="prefers dark roast", context="morning only")
        entries = memory.get_active_knowledge("user1")
        assert len(entries) == 1
        assert entries[0]["subject"] == "coffee"
        assert entries[0]["content"] == "prefers dark roast"
        assert entries[0]["context"] == "morning only"

    def test_knowledge_categories(self, memory):
        memory.save_knowledge("user1", category="preference", subject="coffee", content="dark roast")
        memory.save_knowledge("user1", category="personal", subject="name", content="Aman")
        memory.save_knowledge("user1", category="temporal", subject="diet",
                              content="keto diet", valid_until="2026-03-31")
        entries = memory.get_active_knowledge("user1")
        assert len(entries) == 3
        categories = {e["category"] for e in entries}
        assert categories == {"preference", "personal", "temporal"}

    def test_knowledge_expiry(self, memory):
        memory.save_knowledge("user1", category="temporal", subject="trip",
                              content="visiting Tokyo", valid_until="2020-01-01")
        entries = memory.get_active_knowledge("user1")
        assert len(entries) == 0  # expired

    def test_knowledge_update(self, memory):
        kid = memory.save_knowledge("user1", category="preference", subject="coffee",
                                    content="dark roast")
        memory.update_knowledge(kid, content="light roast")
        entries = memory.get_active_knowledge("user1")
        assert entries[0]["content"] == "light roast"

    def test_knowledge_user_isolation(self, memory):
        memory.save_knowledge("user1", category="personal", subject="name", content="Aman")
        memory.save_knowledge("user2", category="personal", subject="name", content="Ali")
        assert len(memory.get_active_knowledge("user1")) == 1
        assert len(memory.get_active_knowledge("user2")) == 1

    def test_search_knowledge(self, memory):
        memory.save_knowledge("user1", category="preference", subject="coffee", content="dark roast every morning")
        memory.save_knowledge("user1", category="preference", subject="tea", content="green tea in evening")
        results = memory.search_knowledge("user1", "morning coffee")
        assert len(results) >= 1
        assert any("coffee" in r["subject"] for r in results)

    def test_save_entity(self, memory):
        eid = memory.save_entity("user1", name="Ali", entity_type="person",
                                 attributes={"email": "ali@co.com", "role": "engineer"})
        entities = memory.get_entities("user1")
        assert len(entities) == 1
        assert entities[0]["name"] == "Ali"
        assert entities[0]["attributes"]["email"] == "ali@co.com"

    def test_entity_upsert(self, memory):
        memory.save_entity("user1", name="Ali", entity_type="person",
                           attributes={"role": "engineer"})
        memory.save_entity("user1", name="Ali", entity_type="person",
                           attributes={"role": "senior engineer", "email": "ali@co.com"})
        entities = memory.get_entities("user1")
        assert len(entities) == 1
        assert entities[0]["attributes"]["role"] == "senior engineer"

    def test_get_entity_by_name(self, memory):
        memory.save_entity("user1", name="SecureClaw", entity_type="project",
                           attributes={"desc": "security tool"})
        entity = memory.get_entity_by_name("user1", "SecureClaw")
        assert entity is not None
        assert entity["entity_type"] == "project"

    def test_save_relationship(self, memory):
        eid1 = memory.save_entity("user1", name="Ali", entity_type="person", attributes={})
        eid2 = memory.save_entity("user1", name="SecureClaw", entity_type="project", attributes={})
        memory.save_relationship("user1", eid1, "works_on", eid2)
        rels = memory.get_relationships("user1")
        assert len(rels) == 1
        assert rels[0]["relation"] == "works_on"

    def test_get_relationships_for_entity(self, memory):
        eid1 = memory.save_entity("user1", name="Ali", entity_type="person", attributes={})
        eid2 = memory.save_entity("user1", name="SecureClaw", entity_type="project", attributes={})
        eid3 = memory.save_entity("user1", name="Bob", entity_type="person", attributes={})
        memory.save_relationship("user1", eid1, "works_on", eid2)
        memory.save_relationship("user1", eid3, "works_on", eid2)
        rels = memory.get_relationships("user1", entity_id=eid2)
        assert len(rels) == 2

    def test_expire_old_knowledge(self, memory):
        memory.save_knowledge("user1", category="temporal", subject="trip",
                              content="visiting Tokyo", valid_until="2020-01-01")
        memory.save_knowledge("user1", category="personal", subject="name", content="Aman")
        count = memory.expire_old_knowledge()
        assert count == 1

    def test_migrate_facts_to_knowledge(self, memory):
        # Simulate old-style facts
        memory.save_fact("user1", "name", "Aman")
        memory.save_fact("user1", "timezone", "UTC+8")
        memory.migrate_facts_to_knowledge()
        entries = memory.get_active_knowledge("user1")
        subjects = {e["subject"] for e in entries}
        assert "name" in subjects
        assert "timezone" in subjects
```

**Step 2: Run tests to verify they fail**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_skills.py::TestKnowledgeGraph -v`
Expected: FAIL — `Memory` has no attribute `save_knowledge`

**Step 3: Add knowledge graph tables to `_init_tables` in `memory.py`**

Add the following SQL to the `_init_tables` method's `executescript` call, after the existing `schedules` table and indexes:

```python
            CREATE TABLE IF NOT EXISTS knowledge (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                category TEXT NOT NULL,
                subject TEXT NOT NULL,
                content TEXT NOT NULL,
                context TEXT,
                valid_from DATE,
                valid_until DATE,
                confidence REAL DEFAULT 1.0,
                source TEXT DEFAULT 'conversation',
                expired INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_knowledge_user
                ON knowledge(user_id, expired);

            CREATE TABLE IF NOT EXISTS entities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                attributes TEXT DEFAULT '{}',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, name, entity_type)
            );

            CREATE TABLE IF NOT EXISTS relationships (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                from_entity_id INTEGER REFERENCES entities(id),
                relation TEXT NOT NULL,
                to_entity_id INTEGER REFERENCES entities(id),
                context TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
```

Then add FTS5 setup **after** `self.conn.commit()` in `_init_tables` (FTS5 cannot be in executescript with IF NOT EXISTS reliably):

```python
        # FTS5 index for knowledge search
        try:
            self.conn.execute("""
                CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(
                    subject, content, context,
                    content=knowledge, content_rowid=id
                )
            """)
            self.conn.commit()
        except Exception:
            pass  # FTS5 may already exist
```

**Step 4: Add knowledge CRUD methods to `Memory` class**

Add these methods to `memory.py` after the existing `close()` method:

```python
    # --- Knowledge Graph ---

    def save_knowledge(self, user_id: str, category: str, subject: str, content: str,
                       context: str = None, valid_from: str = None, valid_until: str = None,
                       confidence: float = 1.0, source: str = "conversation") -> int:
        """Save a knowledge entry. Returns the entry ID."""
        cursor = self.conn.execute(
            """INSERT INTO knowledge (user_id, category, subject, content, context,
               valid_from, valid_until, confidence, source)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)""",
            (str(user_id), category, subject, content, context,
             valid_from, valid_until, confidence, source)
        )
        self.conn.commit()
        kid = cursor.lastrowid
        # Sync FTS index
        try:
            self.conn.execute(
                "INSERT INTO knowledge_fts(rowid, subject, content, context) VALUES (?, ?, ?, ?)",
                (kid, subject, content, context or "")
            )
            self.conn.commit()
        except Exception:
            pass
        return kid

    def update_knowledge(self, knowledge_id: int, content: str = None, context: str = None,
                         valid_until: str = None, confidence: float = None):
        """Update a knowledge entry."""
        updates = []
        params = []
        if content is not None:
            updates.append("content = ?")
            params.append(content)
        if context is not None:
            updates.append("context = ?")
            params.append(context)
        if valid_until is not None:
            updates.append("valid_until = ?")
            params.append(valid_until)
        if confidence is not None:
            updates.append("confidence = ?")
            params.append(confidence)
        if not updates:
            return
        updates.append("updated_at = CURRENT_TIMESTAMP")
        params.append(knowledge_id)
        self.conn.execute(
            f"UPDATE knowledge SET {', '.join(updates)} WHERE id = ?", params
        )
        self.conn.commit()
        # Sync FTS
        try:
            row = self.conn.execute(
                "SELECT subject, content, context FROM knowledge WHERE id = ?",
                (knowledge_id,)
            ).fetchone()
            if row:
                self.conn.execute("DELETE FROM knowledge_fts WHERE rowid = ?", (knowledge_id,))
                self.conn.execute(
                    "INSERT INTO knowledge_fts(rowid, subject, content, context) VALUES (?, ?, ?, ?)",
                    (knowledge_id, row[0], row[1], row[2] or "")
                )
                self.conn.commit()
        except Exception:
            pass

    def get_active_knowledge(self, user_id: str) -> list[dict]:
        """Get all non-expired knowledge for a user."""
        rows = self.conn.execute(
            """SELECT id, category, subject, content, context, valid_from, valid_until,
                      confidence, source, created_at
               FROM knowledge
               WHERE user_id = ? AND expired = 0
               AND (valid_until IS NULL OR valid_until >= date('now'))
               ORDER BY category, subject""",
            (str(user_id),)
        ).fetchall()
        return [
            {"id": r[0], "category": r[1], "subject": r[2], "content": r[3],
             "context": r[4], "valid_from": r[5], "valid_until": r[6],
             "confidence": r[7], "source": r[8], "created_at": r[9]}
            for r in rows
        ]

    def search_knowledge(self, user_id: str, query: str, limit: int = 10) -> list[dict]:
        """Full-text search over knowledge entries."""
        try:
            rows = self.conn.execute(
                """SELECT k.id, k.category, k.subject, k.content, k.context,
                          k.valid_until, k.confidence
                   FROM knowledge_fts fts
                   JOIN knowledge k ON k.id = fts.rowid
                   WHERE knowledge_fts MATCH ? AND k.user_id = ? AND k.expired = 0
                   ORDER BY rank
                   LIMIT ?""",
                (query, str(user_id), limit)
            ).fetchall()
        except Exception:
            # Fallback to LIKE search if FTS fails
            search = f"%{query}%"
            rows = self.conn.execute(
                """SELECT id, category, subject, content, context, valid_until, confidence
                   FROM knowledge
                   WHERE user_id = ? AND expired = 0
                   AND (subject LIKE ? OR content LIKE ? OR context LIKE ?)
                   LIMIT ?""",
                (str(user_id), search, search, search, limit)
            ).fetchall()
        return [
            {"id": r[0], "category": r[1], "subject": r[2], "content": r[3],
             "context": r[4], "valid_until": r[5], "confidence": r[6]}
            for r in rows
        ]

    def save_entity(self, user_id: str, name: str, entity_type: str,
                    attributes: dict = None) -> int:
        """Save or update an entity. Returns entity ID."""
        attrs_json = json.dumps(attributes or {})
        try:
            cursor = self.conn.execute(
                """INSERT INTO entities (user_id, name, entity_type, attributes)
                   VALUES (?, ?, ?, ?)""",
                (str(user_id), name, entity_type, attrs_json)
            )
            self.conn.commit()
            return cursor.lastrowid
        except sqlite3.IntegrityError:
            # Update existing
            self.conn.execute(
                """UPDATE entities SET attributes = ?, created_at = CURRENT_TIMESTAMP
                   WHERE user_id = ? AND name = ? AND entity_type = ?""",
                (attrs_json, str(user_id), name, entity_type)
            )
            self.conn.commit()
            row = self.conn.execute(
                "SELECT id FROM entities WHERE user_id = ? AND name = ? AND entity_type = ?",
                (str(user_id), name, entity_type)
            ).fetchone()
            return row[0]

    def get_entities(self, user_id: str, entity_type: str = None) -> list[dict]:
        """Get entities for a user, optionally filtered by type."""
        if entity_type:
            rows = self.conn.execute(
                "SELECT id, name, entity_type, attributes FROM entities WHERE user_id = ? AND entity_type = ?",
                (str(user_id), entity_type)
            ).fetchall()
        else:
            rows = self.conn.execute(
                "SELECT id, name, entity_type, attributes FROM entities WHERE user_id = ?",
                (str(user_id),)
            ).fetchall()
        return [
            {"id": r[0], "name": r[1], "entity_type": r[2],
             "attributes": json.loads(r[3]) if r[3] else {}}
            for r in rows
        ]

    def get_entity_by_name(self, user_id: str, name: str) -> dict | None:
        """Get an entity by name (case-insensitive)."""
        row = self.conn.execute(
            "SELECT id, name, entity_type, attributes FROM entities WHERE user_id = ? AND name = ? COLLATE NOCASE",
            (str(user_id), name)
        ).fetchone()
        if not row:
            return None
        return {"id": row[0], "name": row[1], "entity_type": row[2],
                "attributes": json.loads(row[3]) if row[3] else {}}

    def save_relationship(self, user_id: str, from_entity_id: int, relation: str,
                          to_entity_id: int, context: str = None):
        """Save a relationship between two entities."""
        self.conn.execute(
            """INSERT INTO relationships (user_id, from_entity_id, relation, to_entity_id, context)
               VALUES (?, ?, ?, ?, ?)""",
            (str(user_id), from_entity_id, relation, to_entity_id, context)
        )
        self.conn.commit()

    def get_relationships(self, user_id: str, entity_id: int = None) -> list[dict]:
        """Get relationships, optionally filtered to those involving a specific entity."""
        if entity_id:
            rows = self.conn.execute(
                """SELECT r.id, r.relation, r.context,
                          e1.name as from_name, e1.entity_type as from_type,
                          e2.name as to_name, e2.entity_type as to_type
                   FROM relationships r
                   JOIN entities e1 ON r.from_entity_id = e1.id
                   JOIN entities e2 ON r.to_entity_id = e2.id
                   WHERE r.user_id = ? AND (r.from_entity_id = ? OR r.to_entity_id = ?)""",
                (str(user_id), entity_id, entity_id)
            ).fetchall()
        else:
            rows = self.conn.execute(
                """SELECT r.id, r.relation, r.context,
                          e1.name as from_name, e1.entity_type as from_type,
                          e2.name as to_name, e2.entity_type as to_type
                   FROM relationships r
                   JOIN entities e1 ON r.from_entity_id = e1.id
                   JOIN entities e2 ON r.to_entity_id = e2.id
                   WHERE r.user_id = ?""",
                (str(user_id),)
            ).fetchall()
        return [
            {"id": r[0], "relation": r[1], "context": r[2],
             "from_name": r[3], "from_type": r[4],
             "to_name": r[5], "to_type": r[6]}
            for r in rows
        ]

    def expire_old_knowledge(self) -> int:
        """Mark expired knowledge entries. Returns count of newly expired."""
        cursor = self.conn.execute(
            """UPDATE knowledge SET expired = 1
               WHERE expired = 0 AND valid_until IS NOT NULL AND valid_until < date('now')"""
        )
        self.conn.commit()
        return cursor.rowcount

    def migrate_facts_to_knowledge(self):
        """One-time migration: copy facts table entries to knowledge table."""
        rows = self.conn.execute("SELECT user_id, key, value, source FROM facts").fetchall()
        for user_id, key, value, source in rows:
            # Check if already migrated
            existing = self.conn.execute(
                "SELECT id FROM knowledge WHERE user_id = ? AND subject = ? AND content = ?",
                (user_id, key, value)
            ).fetchone()
            if not existing:
                self.save_knowledge(
                    user_id, category="personal", subject=key,
                    content=value, source=source or "migrated"
                )
        logger.info(f"Migrated {len(rows)} facts to knowledge table")
```

**Step 5: Run tests to verify they pass**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_skills.py::TestKnowledgeGraph -v`
Expected: All PASS

**Step 6: Run full test suite to verify no regressions**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All existing tests still PASS

**Step 7: Commit**

```bash
git add amanclaw/memory.py tests/test_skills.py
git commit -m "feat: add knowledge graph schema, CRUD methods, and migration"
```

---

### Task 2: Knowledge Extraction (LLM Module)

**Files:**
- Modify: `amanclaw/llm.py` (add extraction prompt and method)
- Test: `tests/test_skills.py` (add extraction parsing tests)

**Step 1: Write failing tests for extraction JSON parsing**

Add to `tests/test_skills.py`:

```python
class TestKnowledgeExtraction:
    def test_parse_extraction_response_valid(self):
        from amanclaw.llm import parse_extraction_response
        raw = json.dumps({
            "knowledge": [
                {"category": "preference", "subject": "coffee", "content": "dark roast"}
            ],
            "entities": [
                {"name": "Ali", "type": "person", "attributes": {"role": "engineer"}}
            ],
            "relationships": [
                {"from": "Ali", "relation": "works_on", "to": "SecureClaw"}
            ],
            "updates": []
        })
        result = parse_extraction_response(raw)
        assert len(result["knowledge"]) == 1
        assert result["knowledge"][0]["subject"] == "coffee"
        assert len(result["entities"]) == 1
        assert len(result["relationships"]) == 1

    def test_parse_extraction_response_empty(self):
        from amanclaw.llm import parse_extraction_response
        raw = json.dumps({"knowledge": [], "entities": [], "relationships": [], "updates": []})
        result = parse_extraction_response(raw)
        assert result["knowledge"] == []

    def test_parse_extraction_response_invalid_json(self):
        from amanclaw.llm import parse_extraction_response
        result = parse_extraction_response("not json at all")
        assert result is None

    def test_parse_extraction_response_json_in_markdown(self):
        from amanclaw.llm import parse_extraction_response
        raw = '```json\n{"knowledge": [{"category": "personal", "subject": "name", "content": "Aman"}], "entities": [], "relationships": [], "updates": []}\n```'
        result = parse_extraction_response(raw)
        assert len(result["knowledge"]) == 1
```

Add `import json` at the top of `tests/test_skills.py` if not already present.

**Step 2: Run tests to verify they fail**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_skills.py::TestKnowledgeExtraction -v`
Expected: FAIL — cannot import `parse_extraction_response`

**Step 3: Add extraction prompt and parsing to `llm.py`**

Add these after the existing `SUMMARY_PROMPT` constant in `llm.py`:

```python
EXTRACTION_PROMPT = """Extract structured knowledge from this conversation exchange.
Return ONLY valid JSON, no other text.

User message: {user_message}
Assistant reply: {assistant_reply}

Existing knowledge about this user:
{existing_knowledge}

Return this JSON structure:
{{
  "knowledge": [
    {{"category": "preference|personal|work|health|routine|temporal", "subject": "topic", "content": "the knowledge", "context": "optional condition", "valid_until": "YYYY-MM-DD or null"}}
  ],
  "entities": [
    {{"name": "entity name", "type": "person|project|place|organization", "attributes": {{}}}}
  ],
  "relationships": [
    {{"from": "entity_name", "relation": "works_on|manages|lives_in|reports_to|etc", "to": "entity_name"}}
  ],
  "updates": [
    {{"id": 123, "content": "corrected value"}}
  ]
}}

Rules:
- Only extract NEW or CHANGED information. Skip greetings and small talk.
- If the user corrects a previous fact, include it in "updates" with the knowledge ID.
- Set valid_until for temporary facts (diets, deadlines, trips).
- Return empty arrays if nothing to extract.
- Return ONLY the JSON object, no markdown fences or extra text."""


def parse_extraction_response(text: str) -> dict | None:
    """Parse the LLM's extraction response into structured data."""
    if not text:
        return None

    # Try direct JSON parse
    try:
        data = json.loads(text.strip())
        if "knowledge" in data:
            return data
    except (json.JSONDecodeError, TypeError):
        pass

    # Try extracting JSON from markdown code block
    json_match = re.search(r'```(?:json)?\s*\n?(.*?)\n?\s*```', text, re.DOTALL)
    if json_match:
        try:
            data = json.loads(json_match.group(1).strip())
            if "knowledge" in data:
                return data
        except (json.JSONDecodeError, TypeError):
            pass

    # Try finding a JSON object in the text
    brace_match = re.search(r'\{.*\}', text, re.DOTALL)
    if brace_match:
        try:
            data = json.loads(brace_match.group(0))
            if "knowledge" in data:
                return data
        except (json.JSONDecodeError, TypeError):
            pass

    return None
```

Then add the `extract_knowledge` method to the `LLM` class, after the `summarize` method:

```python
    async def extract_knowledge(self, user_message: str, assistant_reply: str,
                                existing_knowledge: str = "") -> dict | None:
        """Ask the LLM to extract structured knowledge from an exchange."""
        prompt = EXTRACTION_PROMPT.format(
            user_message=user_message[:2000],
            assistant_reply=assistant_reply[:2000],
            existing_knowledge=existing_knowledge[:1000] or "(none yet)",
        )
        try:
            resp = await self._call_api([
                {"role": "system", "content": "You are a knowledge extraction assistant. Return only valid JSON."},
                {"role": "user", "content": prompt},
            ])
            content = resp["choices"][0]["message"].get("content", "")
            return parse_extraction_response(content)
        except Exception as e:
            logger.warning(f"Knowledge extraction failed: {e}")
            return None
```

**Step 4: Run tests to verify they pass**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_skills.py::TestKnowledgeExtraction -v`
Expected: All PASS

**Step 5: Commit**

```bash
git add amanclaw/llm.py tests/test_skills.py
git commit -m "feat: add LLM knowledge extraction prompt and JSON parser"
```

---

### Task 3: Context Building & System Prompt Upgrade

**Files:**
- Modify: `amanclaw/llm.py` (upgrade `_build_system_prompt` to format knowledge by category)
- Modify: `amanclaw/bot.py` (upgrade `build_context` to query knowledge graph)

**Step 1: Add `format_knowledge_context` function to `llm.py`**

Add this function before the `LLM` class:

```python
def format_knowledge_context(knowledge: list[dict], entities: list[dict],
                              relationships: list[dict]) -> str:
    """Format knowledge graph data for injection into the system prompt."""
    if not knowledge and not entities:
        return ""

    sections = []

    # Group knowledge by category
    by_category = {}
    for k in knowledge:
        cat = k["category"]
        if cat not in by_category:
            by_category[cat] = []
        by_category[cat].append(k)

    category_labels = {
        "preference": "Preferences",
        "personal": "Personal",
        "work": "Work",
        "health": "Health",
        "routine": "Routines",
        "temporal": "Temporal (active now)",
    }

    for cat, label in category_labels.items():
        items = by_category.get(cat, [])
        if not items:
            continue
        lines = [f"### {label}"]
        for item in items:
            line = f"- {item['subject']}: {item['content']}"
            if item.get("context"):
                line += f" (context: {item['context']})"
            if item.get("valid_until"):
                line += f" [expires: {item['valid_until']}]"
            lines.append(line)
        sections.append("\n".join(lines))

    # Entities and relationships
    if entities:
        lines = ["### People & Projects"]
        for e in entities:
            attrs = e.get("attributes", {})
            attr_str = ", ".join(f"{k}: {v}" for k, v in attrs.items()) if attrs else ""
            line = f"- {e['name']} ({e['entity_type']})"
            if attr_str:
                line += f": {attr_str}"
            # Find relationships for this entity
            rels = [r for r in relationships
                    if r.get("from_name") == e["name"] or r.get("to_name") == e["name"]]
            for r in rels:
                line += f" -- {r['relation']} {r['to_name'] if r['from_name'] == e['name'] else r['from_name']}"
            lines.append(line)
        sections.append("\n".join(lines))

    return "\n\n".join(sections)
```

**Step 2: Update `_build_system_prompt` in `LLM` class**

Change the method signature and body to accept the new knowledge format:

```python
    def _build_system_prompt(self, base_prompt: str, facts: dict = None,
                             summary: str = None, knowledge_context: str = None) -> str:
        prompt = base_prompt.format(datetime=datetime.now().strftime("%Y-%m-%d %H:%M %A"))

        # New knowledge graph context (preferred over flat facts)
        if knowledge_context:
            prompt += f"\n\n## What I know about this user\n{knowledge_context}"
        elif facts:
            # Backward compatibility: flat facts dict
            facts_text = "\n".join(f"- {k}: {v}" for k, v in facts.items())
            prompt += f"\n\n## What I know about this user\n{facts_text}"

        if summary:
            prompt += f"\n\n## Previous conversation summary\n{summary}"

        return prompt
```

**Step 3: Update all callers of `_build_system_prompt`**

In `_respond_native`:
```python
        system = self._build_system_prompt(SYSTEM_PROMPT_NATIVE, facts, summary,
                                           knowledge_context=kwargs.get("knowledge_context"))
```

In `_respond_fallback`:
```python
        system = self._build_system_prompt(base, facts, summary,
                                           knowledge_context=kwargs.get("knowledge_context"))
```

Update `respond` method signature to pass through `knowledge_context`:
```python
    async def respond(self, message, history: list[dict], flagged: bool = False,
                      facts: dict = None, summary: str = None,
                      knowledge_context: str = None) -> str:
```

And pass it to both `_respond_native` and `_respond_fallback` (add `knowledge_context` param to both methods and thread it through).

**Step 4: Update `build_context` in `bot.py`**

Replace the current `build_context` function:

```python
async def build_context(user_id: str, message_text: str = "") -> tuple[list, dict, str, str]:
    """Build the smart context: history, facts, summary, knowledge context.
    Auto-summarize if needed."""
    history = memory.get_history(user_id)
    facts = memory.get_facts(user_id)  # backward compat
    summary = memory.get_latest_summary(user_id)

    # Build knowledge graph context
    knowledge_entries = memory.get_active_knowledge(user_id)
    entities = memory.get_entities(user_id)
    relationships = memory.get_relationships(user_id)

    # Also search for relevant knowledge based on message
    if message_text:
        relevant = memory.search_knowledge(user_id, message_text, limit=5)
        # Merge relevant results (deduplicate by ID)
        existing_ids = {k["id"] for k in knowledge_entries}
        for r in relevant:
            if r["id"] not in existing_ids:
                knowledge_entries.append(r)

    from amanclaw.llm import format_knowledge_context
    knowledge_context = format_knowledge_context(knowledge_entries, entities, relationships)

    # Auto-summarize when conversation gets long
    msg_count = memory.get_message_count(user_id)
    summarized_count = memory.get_summarized_message_count(user_id)
    unsummarized = msg_count - summarized_count
    if unsummarized > 40:
        old_msgs = memory.get_old_messages(user_id, before_last_n=20, limit=40)
        if old_msgs:
            new_summary = llm.summarize(old_msgs)
            if new_summary:
                if summary:
                    new_summary = f"{summary}\n\n{new_summary}"
                memory.save_summary(user_id, new_summary, len(old_msgs))
                summary = new_summary
                logger.info(f"Auto-summarized {len(old_msgs)} messages for user {user_id}")

    return history, facts, summary, knowledge_context
```

**Step 5: Update `handle_message` and `handle_photo` callers in `bot.py`**

In `handle_message`, change:
```python
        history, facts, summary, knowledge_context = await build_context(user_id, clean_text)
        response = await llm.respond(clean_text, history, flagged=was_flagged,
                                     facts=facts, summary=summary,
                                     knowledge_context=knowledge_context)
```

In `handle_photo`, change:
```python
        history, facts, summary, knowledge_context = await build_context(user_id)
        response = await llm.respond(vision_msg, history, flagged=was_flagged,
                                     facts=facts, summary=summary,
                                     knowledge_context=knowledge_context)
```

**Step 6: Run full test suite**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All PASS

**Step 7: Commit**

```bash
git add amanclaw/llm.py amanclaw/bot.py
git commit -m "feat: upgrade context building to use knowledge graph"
```

---

### Task 4: Background Knowledge Extraction in Bot

**Files:**
- Modify: `amanclaw/bot.py` (add background extraction after each exchange)

**Step 1: Add background extraction function to `bot.py`**

Add this function after `build_context`:

```python
async def extract_and_save_knowledge(user_id: str, user_msg: str, assistant_reply: str):
    """Background task: extract knowledge from exchange and save to DB."""
    try:
        # Get existing knowledge for dedup context
        existing = memory.get_active_knowledge(user_id)
        existing_summary = "\n".join(
            f"- [{e['category']}] {e['subject']}: {e['content']}" for e in existing[:20]
        )

        extracted = await llm.extract_knowledge(user_msg, assistant_reply, existing_summary)
        if not extracted:
            return

        # Save knowledge entries
        for k in extracted.get("knowledge", []):
            memory.save_knowledge(
                user_id,
                category=k.get("category", "personal"),
                subject=k.get("subject", ""),
                content=k.get("content", ""),
                context=k.get("context"),
                valid_until=k.get("valid_until"),
                source="conversation",
            )

        # Save entities
        entity_name_to_id = {}
        for e in extracted.get("entities", []):
            eid = memory.save_entity(
                user_id,
                name=e.get("name", ""),
                entity_type=e.get("type", "person"),
                attributes=e.get("attributes", {}),
            )
            entity_name_to_id[e.get("name", "")] = eid

        # Save relationships
        for r in extracted.get("relationships", []):
            from_name = r.get("from", "")
            to_name = r.get("to", "")
            # Resolve entity IDs
            from_id = entity_name_to_id.get(from_name)
            to_id = entity_name_to_id.get(to_name)
            if not from_id:
                ent = memory.get_entity_by_name(user_id, from_name)
                from_id = ent["id"] if ent else None
            if not to_id:
                ent = memory.get_entity_by_name(user_id, to_name)
                to_id = ent["id"] if ent else None
            if from_id and to_id:
                memory.save_relationship(user_id, from_id, r.get("relation", "related_to"), to_id)

        # Apply updates to existing knowledge
        for u in extracted.get("updates", []):
            kid = u.get("id")
            if kid and u.get("content"):
                memory.update_knowledge(kid, content=u["content"])

        count = len(extracted.get("knowledge", [])) + len(extracted.get("entities", []))
        if count:
            logger.info(f"Extracted {count} knowledge items for user {user_id}")

    except Exception as e:
        logger.warning(f"Background knowledge extraction failed for {user_id}: {e}")
```

**Step 2: Fire extraction after each exchange in `handle_message`**

After `memory.save_exchange(...)` in `handle_message`, add:

```python
    # Background knowledge extraction (non-blocking)
    asyncio.create_task(extract_and_save_knowledge(user_id, message_text, response))
```

Do the same in `handle_photo` after `memory.save_exchange(...)`:

```python
    asyncio.create_task(extract_and_save_knowledge(user_id, save_text, response))
```

**Step 3: Do the same in WhatsApp adapter**

In `amanclaw/whatsapp.py`, in `_process_message`, after `self.memory.save_exchange(...)`:

```python
            # Background knowledge extraction
            from amanclaw.bot import extract_and_save_knowledge
            asyncio.create_task(extract_and_save_knowledge(user_id, text, response))
```

**Step 4: Add knowledge expiry to the daily prune job**

In `bot.py`, update `prune_job`:

```python
async def prune_job(context: ContextTypes.DEFAULT_TYPE):
    """Daily cleanup of old messages, delivered reminders, and expired knowledge."""
    msgs = memory.prune_all_users(keep_last=200)
    reminders = memory.prune_delivered_reminders(older_than_days=30)
    expired = memory.expire_old_knowledge()
    if msgs or reminders or expired:
        logger.info(f"Pruned {msgs} old messages, {reminders} delivered reminders, {expired} expired knowledge")
```

**Step 5: Run full test suite**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All PASS

**Step 6: Commit**

```bash
git add amanclaw/bot.py amanclaw/whatsapp.py
git commit -m "feat: add background knowledge extraction after each exchange"
```

---

### Task 5: Update Remember Skill

**Files:**
- Modify: `amanclaw/skills/remember.py` (update `save_fact` to write to knowledge, add `recall` skill)
- Test: `tests/test_skills.py` (add tests for updated skills)

**Step 1: Write failing tests**

Add to `tests/test_skills.py`:

```python
class TestRememberSkillKnowledge:
    @pytest.fixture(autouse=True)
    def setup(self):
        from amanclaw.memory import Memory
        from amanclaw.skills.remember import configure, set_current_user
        self.memory = Memory(":memory:")
        configure(memory=self.memory)
        set_current_user("testuser")
        yield
        self.memory.close()

    def test_save_fact_writes_to_knowledge(self):
        from amanclaw.skills.remember import save_fact
        result = save_fact(key="timezone", value="UTC+8")
        assert "Remembered" in result
        # Should be in knowledge table
        entries = self.memory.get_active_knowledge("testuser")
        assert any(e["subject"] == "timezone" for e in entries)

    def test_save_fact_with_category(self):
        from amanclaw.skills.remember import save_fact
        result = save_fact(key="coffee", value="dark roast", category="preference")
        entries = self.memory.get_active_knowledge("testuser")
        coffee = [e for e in entries if e["subject"] == "coffee"]
        assert len(coffee) == 1
        assert coffee[0]["category"] == "preference"

    def test_recall_skill(self):
        from amanclaw.skills.remember import save_fact, recall
        save_fact(key="name", value="Aman")
        save_fact(key="language", value="Python")
        result = recall(query="name")
        assert "Aman" in result
```

**Step 2: Run tests to verify they fail**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_skills.py::TestRememberSkillKnowledge -v`
Expected: FAIL

**Step 3: Update `remember.py`**

```python
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
```

**Step 4: Run tests to verify they pass**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_skills.py::TestRememberSkillKnowledge -v`
Expected: All PASS

**Step 5: Run full test suite**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All PASS

**Step 6: Commit**

```bash
git add amanclaw/skills/remember.py tests/test_skills.py
git commit -m "feat: update remember skill to use knowledge graph, add recall skill"
```

---

### Task 6: Migration & Integration Test

**Files:**
- Modify: `amanclaw/memory.py` (add auto-migration on init)
- Modify: `amanclaw/bot.py` (trigger migration on startup)
- Test: `tests/test_skills.py` (integration test)

**Step 1: Add auto-migration check to `Memory.__init__`**

At the end of `__init__`, after `_init_tables()`:

```python
        # Auto-migrate facts to knowledge if needed
        self._maybe_migrate_facts()
```

Add the method:

```python
    def _maybe_migrate_facts(self):
        """Check if facts need migration to knowledge table."""
        fact_count = self.conn.execute("SELECT COUNT(*) FROM facts").fetchone()[0]
        if fact_count == 0:
            return
        knowledge_count = self.conn.execute("SELECT COUNT(*) FROM knowledge WHERE source = 'migrated'").fetchone()[0]
        if knowledge_count == 0 and fact_count > 0:
            self.migrate_facts_to_knowledge()
```

**Step 2: Write integration test**

Add to `tests/test_skills.py`:

```python
class TestKnowledgeIntegration:
    @pytest.fixture
    def memory(self):
        from amanclaw.memory import Memory
        m = Memory(":memory:")
        yield m
        m.close()

    def test_full_knowledge_flow(self, memory):
        """Test the complete knowledge lifecycle."""
        # Save some knowledge
        kid = memory.save_knowledge("user1", "preference", "coffee", "dark roast",
                                    context="morning only")
        assert kid > 0

        # Save entities
        eid1 = memory.save_entity("user1", "Ali", "person", {"role": "engineer"})
        eid2 = memory.save_entity("user1", "SecureClaw", "project", {"type": "security"})

        # Save relationship
        memory.save_relationship("user1", eid1, "works_on", eid2)

        # Query everything
        knowledge = memory.get_active_knowledge("user1")
        assert len(knowledge) == 1

        entities = memory.get_entities("user1")
        assert len(entities) == 2

        rels = memory.get_relationships("user1")
        assert len(rels) == 1
        assert rels[0]["from_name"] == "Ali"
        assert rels[0]["to_name"] == "SecureClaw"

        # Search
        results = memory.search_knowledge("user1", "coffee")
        assert len(results) >= 1

        # Update
        memory.update_knowledge(kid, content="light roast")
        knowledge = memory.get_active_knowledge("user1")
        assert knowledge[0]["content"] == "light roast"

    def test_facts_migration_on_init(self):
        """Test that existing facts are migrated to knowledge on init."""
        from amanclaw.memory import Memory
        m = Memory(":memory:")
        # Save facts using old method
        m.save_fact("user1", "name", "Aman")
        m.save_fact("user1", "lang", "Python")
        # Migration should have happened at init since facts exist
        # But we need to re-trigger since we added facts after init
        m.migrate_facts_to_knowledge()
        entries = m.get_active_knowledge("user1")
        assert len(entries) >= 2
        m.close()
```

**Step 3: Run full test suite**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All PASS

**Step 4: Commit**

```bash
git add amanclaw/memory.py amanclaw/bot.py tests/test_skills.py
git commit -m "feat: add auto-migration and integration tests for knowledge graph"
```

---

### Task 7: Update Test Registry & Final Verification

**Files:**
- Modify: `tests/test_skills.py` (update `TestSkillRegistry` to include new `recall` skill)

**Step 1: Update skill registry test**

In `TestSkillRegistry.test_skills_registered`, add `"recall"` to the expected set:

```python
    def test_skills_registered(self):
        expected = {"run_command", "read_file", "write_file", "list_files",
                    "system_status", "save_fact", "get_facts", "recall",
                    "set_reminder", "list_reminders", "cancel_reminder"}
        assert expected.issubset(set(REGISTRY.keys()))
```

**Step 2: Run full test suite**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All PASS

**Step 3: Commit**

```bash
git add tests/test_skills.py
git commit -m "test: update skill registry test for recall skill"
```
