# Self-Learning Engine Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a self-learning engine so AmanClaw learns from corrections, teachings, documents, failures, and behavioral patterns — like raising a baby.

**Architecture:** New `learning.py` module orchestrates all learning pipelines. Extends `memory.py` with 5 new tables. Enhances existing skills and adds new commands (`/teach`, `/learned`, `/forget`). Background jobs for pattern analysis and proactive check-ins.

**Tech Stack:** Python, SQLite, FTS5, existing LLM module (no new dependencies)

---

### Task 1: Add New Learning Tables to Memory

**Files:**
- Modify: `amanclaw/memory.py`
- Test: `tests/test_skills.py`

**Step 1: Write failing tests for new tables**

Add to `tests/test_skills.py`:

```python
class TestLearningTables:
    @pytest.fixture
    def memory(self):
        from amanclaw.memory import Memory
        m = Memory(":memory:")
        yield m
        m.close()

    # --- Corrections ---
    def test_save_correction(self, memory):
        kid = memory.save_knowledge("user1", "preference", "coffee", "americano")
        cid = memory.save_correction("user1", kid, "americano", "latte", "no I meant latte")
        assert cid > 0
        corrections = memory.get_corrections("user1")
        assert len(corrections) == 1
        assert corrections[0]["old_content"] == "americano"
        assert corrections[0]["new_content"] == "latte"

    def test_correction_count(self, memory):
        kid = memory.save_knowledge("user1", "preference", "coffee", "americano")
        memory.save_correction("user1", kid, "americano", "latte", "wrong")
        memory.save_correction("user1", kid, "latte", "cappuccino", "actually this")
        assert len(memory.get_corrections("user1")) == 2

    # --- Teachings ---
    def test_save_teaching(self, memory):
        tid = memory.save_teaching("user1", "when I say deploy", "push to staging first", "work")
        assert tid > 0
        teachings = memory.get_teachings("user1")
        assert len(teachings) == 1
        assert teachings[0]["trigger_pattern"] == "when I say deploy"

    def test_teaching_active_filter(self, memory):
        memory.save_teaching("user1", "trigger1", "response1", "general")
        tid = memory.save_teaching("user1", "trigger2", "response2", "general")
        memory.deactivate_teaching("user1", tid)
        active = memory.get_teachings("user1", active_only=True)
        assert len(active) == 1

    def test_increment_teaching_usage(self, memory):
        tid = memory.save_teaching("user1", "trigger", "response", "general")
        memory.increment_teaching_usage(tid)
        memory.increment_teaching_usage(tid)
        teachings = memory.get_teachings("user1")
        assert teachings[0]["usage_count"] == 2

    # --- Documents ---
    def test_save_document_chunks(self, memory):
        memory.save_document_chunk("user1", "readme.txt", "txt", 0, "first chunk of text")
        memory.save_document_chunk("user1", "readme.txt", "txt", 1, "second chunk of text")
        chunks = memory.get_document_chunks("user1", "readme.txt")
        assert len(chunks) == 2
        assert chunks[0]["chunk_index"] == 0

    def test_search_documents(self, memory):
        memory.save_document_chunk("user1", "notes.txt", "txt", 0, "python is great for automation")
        memory.save_document_chunk("user1", "notes.txt", "txt", 1, "rust is great for performance")
        results = memory.search_documents("user1", "python automation")
        assert len(results) >= 1

    def test_list_user_documents(self, memory):
        memory.save_document_chunk("user1", "a.txt", "txt", 0, "aaa")
        memory.save_document_chunk("user1", "b.pdf", "pdf", 0, "bbb")
        docs = memory.list_documents("user1")
        assert len(docs) == 2

    def test_delete_document(self, memory):
        memory.save_document_chunk("user1", "temp.txt", "txt", 0, "temp content")
        deleted = memory.delete_document("user1", "temp.txt")
        assert deleted > 0
        assert len(memory.get_document_chunks("user1", "temp.txt")) == 0

    # --- Failure Log ---
    def test_save_failure(self, memory):
        fid = memory.save_failure("user1", "run_command", '{"command": "ls"}', "permission denied")
        assert fid > 0
        failures = memory.get_recent_failures("user1", limit=10)
        assert len(failures) == 1
        assert failures[0]["skill_name"] == "run_command"

    def test_resolve_failure(self, memory):
        fid = memory.save_failure("user1", "web_search", '{}', "timeout")
        memory.resolve_failure(fid, "user retried successfully")
        failures = memory.get_recent_failures("user1", limit=10)
        assert failures[0]["resolved"] == 1

    # --- Behavioral Patterns ---
    def test_save_pattern(self, memory):
        pid = memory.save_behavioral_pattern(
            "user1", "response_length", "user prefers short answers",
            '{"avg_words": 50}', 0.8
        )
        assert pid > 0
        patterns = memory.get_behavioral_patterns("user1")
        assert len(patterns) == 1
        assert patterns[0]["confidence"] == 0.8

    def test_confirm_pattern(self, memory):
        pid = memory.save_behavioral_pattern("user1", "topic", "asks about servers daily", "{}", 0.6)
        memory.confirm_pattern(pid)
        patterns = memory.get_behavioral_patterns("user1")
        assert patterns[0]["confirmed"] == 1

    def test_pattern_update(self, memory):
        pid = memory.save_behavioral_pattern("user1", "topic", "desc", "{}", 0.5)
        memory.update_behavioral_pattern(pid, confidence=0.9, description="updated desc")
        patterns = memory.get_behavioral_patterns("user1")
        assert patterns[0]["confidence"] == 0.9
        assert patterns[0]["description"] == "updated desc"
```

**Step 2: Run tests to verify they fail**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_skills.py::TestLearningTables -v`
Expected: FAIL — methods don't exist yet

**Step 3: Add table schemas and CRUD methods to memory.py**

In `amanclaw/memory.py`, add these tables to `_init_tables()` after the existing `CREATE TABLE` statements:

```python
            CREATE TABLE IF NOT EXISTS corrections (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                knowledge_id INTEGER REFERENCES knowledge(id),
                old_content TEXT NOT NULL,
                new_content TEXT NOT NULL,
                trigger_text TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS teachings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                trigger_pattern TEXT NOT NULL,
                response_guidance TEXT NOT NULL,
                category TEXT DEFAULT 'general',
                active INTEGER DEFAULT 1,
                usage_count INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                source_name TEXT NOT NULL,
                source_type TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS failure_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                skill_name TEXT NOT NULL,
                skill_input TEXT DEFAULT '{}',
                error_message TEXT NOT NULL,
                user_feedback TEXT,
                resolved INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS behavioral_patterns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                pattern_type TEXT NOT NULL,
                description TEXT NOT NULL,
                evidence TEXT DEFAULT '{}',
                confidence REAL DEFAULT 0.5,
                confirmed INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
```

After `_init_tables()`, add FTS5 for documents (inside the try block alongside `knowledge_fts`):

```python
        try:
            self.conn.execute("""
                CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
                    content,
                    content=documents, content_rowid=id
                )
            """)
            self.conn.commit()
        except Exception as e:
            logger.debug("Documents FTS5 operation failed: %s", e)
```

Then add these methods to the `Memory` class:

```python
    # --- Corrections ---

    def save_correction(self, user_id: str, knowledge_id: int,
                        old_content: str, new_content: str,
                        trigger_text: str = None) -> int:
        cursor = self.conn.execute(
            "INSERT INTO corrections (user_id, knowledge_id, old_content, new_content, trigger_text) "
            "VALUES (?, ?, ?, ?, ?)",
            (str(user_id), knowledge_id, old_content, new_content, trigger_text)
        )
        self.conn.commit()
        return cursor.lastrowid

    def get_corrections(self, user_id: str, limit: int = 20) -> list[dict]:
        rows = self.conn.execute(
            "SELECT id, knowledge_id, old_content, new_content, trigger_text, created_at "
            "FROM corrections WHERE user_id = ? ORDER BY created_at DESC LIMIT ?",
            (str(user_id), limit)
        ).fetchall()
        return [{"id": r[0], "knowledge_id": r[1], "old_content": r[2],
                 "new_content": r[3], "trigger_text": r[4], "created_at": r[5]} for r in rows]

    # --- Teachings ---

    def save_teaching(self, user_id: str, trigger_pattern: str,
                      response_guidance: str, category: str = "general") -> int:
        cursor = self.conn.execute(
            "INSERT INTO teachings (user_id, trigger_pattern, response_guidance, category) "
            "VALUES (?, ?, ?, ?)",
            (str(user_id), trigger_pattern, response_guidance, category)
        )
        self.conn.commit()
        return cursor.lastrowid

    def get_teachings(self, user_id: str, active_only: bool = False) -> list[dict]:
        query = "SELECT id, trigger_pattern, response_guidance, category, active, usage_count, created_at FROM teachings WHERE user_id = ?"
        params = [str(user_id)]
        if active_only:
            query += " AND active = 1"
        query += " ORDER BY usage_count DESC"
        rows = self.conn.execute(query, params).fetchall()
        return [{"id": r[0], "trigger_pattern": r[1], "response_guidance": r[2],
                 "category": r[3], "active": r[4], "usage_count": r[5], "created_at": r[6]} for r in rows]

    def deactivate_teaching(self, user_id: str, teaching_id: int):
        self.conn.execute(
            "UPDATE teachings SET active = 0 WHERE id = ? AND user_id = ?",
            (teaching_id, str(user_id))
        )
        self.conn.commit()

    def increment_teaching_usage(self, teaching_id: int):
        self.conn.execute(
            "UPDATE teachings SET usage_count = usage_count + 1 WHERE id = ?",
            (teaching_id,)
        )
        self.conn.commit()

    # --- Documents ---

    def save_document_chunk(self, user_id: str, source_name: str, source_type: str,
                            chunk_index: int, content: str) -> int:
        cursor = self.conn.execute(
            "INSERT INTO documents (user_id, source_name, source_type, chunk_index, content) "
            "VALUES (?, ?, ?, ?, ?)",
            (str(user_id), source_name, source_type, chunk_index, content)
        )
        did = cursor.lastrowid
        try:
            self.conn.execute(
                "INSERT INTO documents_fts(rowid, content) VALUES (?, ?)",
                (did, content)
            )
        except Exception as e:
            logger.debug("Documents FTS5 operation failed: %s", e)
        self.conn.commit()
        return did

    def get_document_chunks(self, user_id: str, source_name: str) -> list[dict]:
        rows = self.conn.execute(
            "SELECT id, chunk_index, content FROM documents WHERE user_id = ? AND source_name = ? ORDER BY chunk_index",
            (str(user_id), source_name)
        ).fetchall()
        return [{"id": r[0], "chunk_index": r[1], "content": r[2]} for r in rows]

    def search_documents(self, user_id: str, query: str, limit: int = 5) -> list[dict]:
        try:
            rows = self.conn.execute(
                """SELECT d.id, d.source_name, d.chunk_index, d.content
                   FROM documents_fts fts
                   JOIN documents d ON d.id = fts.rowid
                   WHERE documents_fts MATCH ? AND d.user_id = ?
                   LIMIT ?""",
                (query, str(user_id), limit)
            ).fetchall()
        except Exception:
            rows = []
        if not rows:
            terms = query.split()
            conditions = []
            params = [str(user_id)]
            for term in terms:
                conditions.append("content LIKE ?")
                params.append(f"%{term}%")
            where = " OR ".join(conditions) if conditions else "1=1"
            rows = self.conn.execute(
                f"SELECT id, source_name, chunk_index, content FROM documents WHERE user_id = ? AND ({where}) LIMIT ?",
                params + [limit]
            ).fetchall()
        return [{"id": r[0], "source_name": r[1], "chunk_index": r[2], "content": r[3]} for r in rows]

    def list_documents(self, user_id: str) -> list[dict]:
        rows = self.conn.execute(
            "SELECT source_name, source_type, COUNT(*) as chunks, MIN(created_at) as created_at "
            "FROM documents WHERE user_id = ? GROUP BY source_name, source_type ORDER BY created_at DESC",
            (str(user_id),)
        ).fetchall()
        return [{"source_name": r[0], "source_type": r[1], "chunks": r[2], "created_at": r[3]} for r in rows]

    def delete_document(self, user_id: str, source_name: str) -> int:
        # Delete FTS entries first
        try:
            self.conn.execute(
                "DELETE FROM documents_fts WHERE rowid IN (SELECT id FROM documents WHERE user_id = ? AND source_name = ?)",
                (str(user_id), source_name)
            )
        except Exception as e:
            logger.debug("Documents FTS5 delete failed: %s", e)
        cursor = self.conn.execute(
            "DELETE FROM documents WHERE user_id = ? AND source_name = ?",
            (str(user_id), source_name)
        )
        self.conn.commit()
        return cursor.rowcount

    # --- Failure Log ---

    def save_failure(self, user_id: str, skill_name: str, skill_input: str,
                     error_message: str) -> int:
        cursor = self.conn.execute(
            "INSERT INTO failure_log (user_id, skill_name, skill_input, error_message) VALUES (?, ?, ?, ?)",
            (str(user_id), skill_name, skill_input, error_message)
        )
        self.conn.commit()
        return cursor.lastrowid

    def resolve_failure(self, failure_id: int, feedback: str = None):
        self.conn.execute(
            "UPDATE failure_log SET resolved = 1, user_feedback = ? WHERE id = ?",
            (feedback, failure_id)
        )
        self.conn.commit()

    def get_recent_failures(self, user_id: str, limit: int = 10) -> list[dict]:
        rows = self.conn.execute(
            "SELECT id, skill_name, skill_input, error_message, user_feedback, resolved, created_at "
            "FROM failure_log WHERE user_id = ? ORDER BY created_at DESC LIMIT ?",
            (str(user_id), limit)
        ).fetchall()
        return [{"id": r[0], "skill_name": r[1], "skill_input": r[2], "error_message": r[3],
                 "user_feedback": r[4], "resolved": r[5], "created_at": r[6]} for r in rows]

    # --- Behavioral Patterns ---

    def save_behavioral_pattern(self, user_id: str, pattern_type: str,
                                description: str, evidence: str = "{}",
                                confidence: float = 0.5) -> int:
        cursor = self.conn.execute(
            "INSERT INTO behavioral_patterns (user_id, pattern_type, description, evidence, confidence) "
            "VALUES (?, ?, ?, ?, ?)",
            (str(user_id), pattern_type, description, evidence, confidence)
        )
        self.conn.commit()
        return cursor.lastrowid

    def get_behavioral_patterns(self, user_id: str, min_confidence: float = 0.0) -> list[dict]:
        rows = self.conn.execute(
            "SELECT id, pattern_type, description, evidence, confidence, confirmed, created_at, updated_at "
            "FROM behavioral_patterns WHERE user_id = ? AND confidence >= ? ORDER BY confidence DESC",
            (str(user_id), min_confidence)
        ).fetchall()
        return [{"id": r[0], "pattern_type": r[1], "description": r[2], "evidence": r[3],
                 "confidence": r[4], "confirmed": r[5], "created_at": r[6], "updated_at": r[7]} for r in rows]

    def confirm_pattern(self, pattern_id: int):
        self.conn.execute(
            "UPDATE behavioral_patterns SET confirmed = 1, confidence = MAX(confidence, 0.9), "
            "updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            (pattern_id,)
        )
        self.conn.commit()

    def update_behavioral_pattern(self, pattern_id: int, confidence: float = None,
                                  description: str = None, evidence: str = None):
        updates = []
        params = []
        if confidence is not None:
            updates.append("confidence = ?")
            params.append(confidence)
        if description is not None:
            updates.append("description = ?")
            params.append(description)
        if evidence is not None:
            updates.append("evidence = ?")
            params.append(evidence)
        if not updates:
            return
        updates.append("updated_at = CURRENT_TIMESTAMP")
        params.append(pattern_id)
        self.conn.execute(
            f"UPDATE behavioral_patterns SET {', '.join(updates)} WHERE id = ?", params
        )
        self.conn.commit()
```

**Step 4: Run tests to verify they pass**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_skills.py::TestLearningTables -v`
Expected: All PASS

**Step 5: Run ALL existing tests to make sure nothing broke**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All PASS

**Step 6: Commit**

```bash
git add amanclaw/memory.py tests/test_skills.py
git commit -m "feat: add learning tables (corrections, teachings, documents, failure_log, behavioral_patterns)"
```

---

### Task 2: Create Learning Engine Module

**Files:**
- Create: `amanclaw/learning.py`
- Test: `tests/test_learning.py`

**Step 1: Write failing tests**

Create `tests/test_learning.py`:

```python
"""Tests for the self-learning engine."""

import json
import pytest
from unittest.mock import AsyncMock, MagicMock, patch
from amanclaw.memory import Memory
from amanclaw.learning import LearningEngine


class TestCorrectionDetection:
    @pytest.fixture
    def engine(self):
        memory = Memory(":memory:")
        engine = LearningEngine(memory)
        yield engine
        memory.close()

    def test_detect_correction_phrases(self, engine):
        assert engine.is_correction("no, I meant latte not americano")
        assert engine.is_correction("actually it's Python not Java")
        assert engine.is_correction("wrong, my name is Ali")
        assert engine.is_correction("that's not right, I prefer tea")
        assert not engine.is_correction("tell me about coffee")
        assert not engine.is_correction("what's the weather?")

    def test_process_correction(self, engine):
        # Set up existing knowledge
        kid = engine.memory.save_knowledge("user1", "preference", "coffee", "americano")
        # Process a correction
        result = engine.process_correction(
            "user1", "no I prefer latte", kid, "americano", "latte"
        )
        assert result is True
        # Knowledge should be updated
        entries = engine.memory.get_active_knowledge("user1")
        assert entries[0]["content"] == "latte"
        # Correction should be logged
        corrections = engine.memory.get_corrections("user1")
        assert len(corrections) == 1


class TestTeachingProcessor:
    @pytest.fixture
    def engine(self):
        memory = Memory(":memory:")
        engine = LearningEngine(memory)
        yield engine
        memory.close()

    def test_detect_teaching_intent(self, engine):
        assert engine.is_teaching("remember that when I say deploy I mean staging")
        assert engine.is_teaching("always respond in Malay when I write in Malay")
        assert engine.is_teaching("from now on, keep answers short")
        assert engine.is_teaching("teach: if I ask about servers, check status first")
        assert not engine.is_teaching("what's the weather?")
        assert not engine.is_teaching("remind me to buy milk")

    def test_save_teaching(self, engine):
        tid = engine.save_teaching("user1", "when I say deploy", "push to staging first", "work")
        assert tid > 0
        teachings = engine.memory.get_teachings("user1")
        assert len(teachings) == 1


class TestDocumentIngestion:
    @pytest.fixture
    def engine(self):
        memory = Memory(":memory:")
        engine = LearningEngine(memory)
        yield engine
        memory.close()

    def test_chunk_text(self, engine):
        text = "word " * 200  # 1000 chars
        chunks = engine.chunk_text(text, chunk_size=500)
        assert len(chunks) >= 2
        # All text should be preserved
        reassembled = "".join(chunks)
        assert reassembled.strip() == text.strip()

    def test_ingest_text_document(self, engine):
        text = "Python is great. " * 50
        count = engine.ingest_document("user1", "notes.txt", "txt", text)
        assert count > 0
        docs = engine.memory.list_documents("user1")
        assert len(docs) == 1
        assert docs[0]["source_name"] == "notes.txt"


class TestFailureTracking:
    @pytest.fixture
    def engine(self):
        memory = Memory(":memory:")
        engine = LearningEngine(memory)
        yield engine
        memory.close()

    def test_log_failure(self, engine):
        fid = engine.log_failure("user1", "run_command", {"command": "ls -la"}, "permission denied")
        assert fid > 0
        failures = engine.memory.get_recent_failures("user1")
        assert len(failures) == 1

    def test_get_failure_summary(self, engine):
        engine.log_failure("user1", "web_search", {}, "timeout")
        engine.log_failure("user1", "web_search", {}, "timeout")
        engine.log_failure("user1", "run_command", {}, "not allowed")
        summary = engine.get_failure_summary("user1")
        assert "web_search" in summary
        assert "2" in summary  # appeared twice


class TestLearningJournal:
    @pytest.fixture
    def engine(self):
        memory = Memory(":memory:")
        engine = LearningEngine(memory)
        yield engine
        memory.close()

    def test_learning_journal(self, engine):
        engine.memory.save_knowledge("user1", "preference", "coffee", "latte", source="conversation")
        engine.memory.save_correction("user1", 1, "americano", "latte", "no I want latte")
        engine.memory.save_teaching("user1", "deploy = staging", "push staging first", "work")
        journal = engine.get_learning_journal("user1")
        assert "coffee" in journal or "latte" in journal
        assert "correction" in journal.lower() or "updated" in journal.lower()
        assert "teaching" in journal.lower() or "taught" in journal.lower()

    def test_empty_journal(self, engine):
        journal = engine.get_learning_journal("user1")
        assert "nothing" in journal.lower() or "no " in journal.lower()
```

**Step 2: Run tests to verify they fail**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_learning.py -v`
Expected: FAIL — module doesn't exist

**Step 3: Create the learning engine**

Create `amanclaw/learning.py`:

```python
"""
Learning Engine — orchestrates all self-learning pipelines.

Pipelines:
1. Correction detection and processing
2. Teaching processing (explicit user teachings)
3. Document ingestion (chunk + index)
4. Failure tracking
5. Behavioral pattern discovery
"""

import re
import json
import logging
from datetime import datetime, timedelta
from amanclaw.memory import Memory

logger = logging.getLogger("amanclaw.learning")

CORRECTION_PATTERNS = [
    r"\bno[,.]?\s+(i\s+)?(meant|prefer|want|like|use|need)",
    r"\bactually[,.]?\s+(it'?s|my|i)",
    r"\bwrong[,.]",
    r"\bthat'?s\s+not\s+(right|correct)",
    r"\bnot\s+\w+[,.]?\s+(it'?s|i\s+meant)",
    r"\bcorrection[:\s]",
    r"\bi\s+said\s+\w+[,.]?\s+not\s+",
]

TEACHING_PATTERNS = [
    r"\bremember\s+that\b",
    r"\balways\s+(respond|answer|reply|do|use)",
    r"\bfrom\s+now\s+on\b",
    r"\bteach:\s*",
    r"\bwhen\s+i\s+(say|ask|write|type)\b.*\b(mean|do|use|respond)",
    r"\bnever\s+(respond|answer|reply|do|use)",
    r"\bkeep\s+(answers?|responses?)\s+(short|brief|long|detailed)",
]


class LearningEngine:
    def __init__(self, memory: Memory):
        self.memory = memory

    # --- Correction Detection ---

    def is_correction(self, text: str) -> bool:
        text_lower = text.lower()
        for pattern in CORRECTION_PATTERNS:
            if re.search(pattern, text_lower):
                return True
        return False

    def process_correction(self, user_id: str, trigger_text: str,
                           knowledge_id: int, old_content: str,
                           new_content: str) -> bool:
        self.memory.update_knowledge(knowledge_id, content=new_content)
        self.memory.save_correction(user_id, knowledge_id, old_content, new_content, trigger_text)
        logger.info(f"Correction for user {user_id}: '{old_content}' -> '{new_content}'")
        return True

    # --- Teaching Detection ---

    def is_teaching(self, text: str) -> bool:
        text_lower = text.lower()
        for pattern in TEACHING_PATTERNS:
            if re.search(pattern, text_lower):
                return True
        return False

    def save_teaching(self, user_id: str, trigger_pattern: str,
                      response_guidance: str, category: str = "general") -> int:
        tid = self.memory.save_teaching(user_id, trigger_pattern, response_guidance, category)
        logger.info(f"New teaching for user {user_id}: '{trigger_pattern}'")
        return tid

    def get_matching_teachings(self, user_id: str, message: str) -> list[dict]:
        teachings = self.memory.get_teachings(user_id, active_only=True)
        matches = []
        message_lower = message.lower()
        for t in teachings:
            trigger = t["trigger_pattern"].lower()
            # Simple keyword matching — check if trigger words appear in message
            trigger_words = set(trigger.split())
            message_words = set(message_lower.split())
            overlap = trigger_words & message_words
            if len(overlap) >= max(1, len(trigger_words) // 2):
                matches.append(t)
                self.memory.increment_teaching_usage(t["id"])
        return matches

    # --- Document Ingestion ---

    def chunk_text(self, text: str, chunk_size: int = 500) -> list[str]:
        if len(text) <= chunk_size:
            return [text]
        chunks = []
        start = 0
        while start < len(text):
            end = start + chunk_size
            if end < len(text):
                # Try to break at sentence boundary
                last_period = text.rfind(".", start, end)
                last_newline = text.rfind("\n", start, end)
                break_at = max(last_period, last_newline)
                if break_at > start:
                    end = break_at + 1
            chunks.append(text[start:end])
            start = end
        return chunks

    def ingest_document(self, user_id: str, source_name: str, source_type: str,
                        text: str) -> int:
        # Delete existing chunks for this document (re-ingest)
        self.memory.delete_document(user_id, source_name)
        chunks = self.chunk_text(text)
        for i, chunk in enumerate(chunks):
            self.memory.save_document_chunk(user_id, source_name, source_type, i, chunk)
        logger.info(f"Ingested '{source_name}' for user {user_id}: {len(chunks)} chunks")
        return len(chunks)

    # --- Failure Tracking ---

    def log_failure(self, user_id: str, skill_name: str, skill_input: dict,
                    error_message: str) -> int:
        input_json = json.dumps(skill_input) if isinstance(skill_input, dict) else str(skill_input)
        return self.memory.save_failure(user_id, skill_name, input_json, error_message)

    def get_failure_summary(self, user_id: str) -> str:
        failures = self.memory.get_recent_failures(user_id, limit=50)
        if not failures:
            return "No failures recorded."
        # Group by skill
        by_skill = {}
        for f in failures:
            name = f["skill_name"]
            if name not in by_skill:
                by_skill[name] = []
            by_skill[name].append(f)
        lines = ["Recent failure summary:"]
        for skill_name, items in by_skill.items():
            unresolved = sum(1 for i in items if not i["resolved"])
            lines.append(f"- {skill_name}: {len(items)} failures ({unresolved} unresolved)")
            # Most common error
            errors = {}
            for i in items:
                e = i["error_message"][:80]
                errors[e] = errors.get(e, 0) + 1
            top_error = max(errors, key=errors.get)
            lines.append(f"  Most common: {top_error} ({errors[top_error]}x)")
        return "\n".join(lines)

    # --- Learning Journal ---

    def get_learning_journal(self, user_id: str, days: int = 7) -> str:
        sections = []

        # Recent knowledge
        knowledge = self.memory.get_active_knowledge(user_id)
        cutoff = (datetime.now() - timedelta(days=days)).strftime("%Y-%m-%d")
        recent_knowledge = [k for k in knowledge if k.get("created_at", "") >= cutoff]
        if recent_knowledge:
            lines = [f"**New knowledge learned ({len(recent_knowledge)} items):**"]
            for k in recent_knowledge[:10]:
                lines.append(f"- [{k['category']}] {k['subject']}: {k['content']}")
            sections.append("\n".join(lines))

        # Recent corrections
        corrections = self.memory.get_corrections(user_id, limit=10)
        recent_corrections = [c for c in corrections if c.get("created_at", "") >= cutoff]
        if recent_corrections:
            lines = [f"**Corrections ({len(recent_corrections)} updates):**"]
            for c in recent_corrections:
                lines.append(f"- Updated: '{c['old_content']}' -> '{c['new_content']}'")
            sections.append("\n".join(lines))

        # Active teachings
        teachings = self.memory.get_teachings(user_id, active_only=True)
        if teachings:
            lines = [f"**Active teachings ({len(teachings)} rules):**"]
            for t in teachings[:10]:
                used = f" (used {t['usage_count']}x)" if t['usage_count'] else ""
                lines.append(f"- {t['trigger_pattern']} -> {t['response_guidance']}{used}")
            sections.append("\n".join(lines))

        # Documents
        docs = self.memory.list_documents(user_id)
        if docs:
            lines = [f"**Ingested documents ({len(docs)}):**"]
            for d in docs:
                lines.append(f"- {d['source_name']} ({d['chunks']} chunks)")
            sections.append("\n".join(lines))

        # Failure summary
        failures = self.memory.get_recent_failures(user_id, limit=20)
        recent_failures = [f for f in failures if f.get("created_at", "") >= cutoff]
        if recent_failures:
            sections.append(self.get_failure_summary(user_id))

        # Behavioral patterns
        patterns = self.memory.get_behavioral_patterns(user_id, min_confidence=0.5)
        if patterns:
            lines = [f"**Observed patterns ({len(patterns)}):**"]
            for p in patterns:
                confirmed = " [confirmed]" if p["confirmed"] else ""
                lines.append(f"- {p['description']} (confidence: {p['confidence']:.0%}){confirmed}")
            sections.append("\n".join(lines))

        if not sections:
            return "No learning activity recorded yet. Talk to me, teach me, or send me documents to learn from!"

        return "\n\n".join(sections)
```

**Step 4: Run tests to verify they pass**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_learning.py -v`
Expected: All PASS

**Step 5: Commit**

```bash
git add amanclaw/learning.py tests/test_learning.py
git commit -m "feat: add learning engine with correction, teaching, document, and failure pipelines"
```

---

### Task 3: Add Teaching and Learning Skills

**Files:**
- Modify: `amanclaw/skills/remember.py`
- Test: `tests/test_learning.py`

**Step 1: Write failing tests**

Add to `tests/test_learning.py`:

```python
class TestTeachingSkill:
    @pytest.fixture(autouse=True)
    def setup(self):
        from amanclaw.memory import Memory
        from amanclaw.learning import LearningEngine
        from amanclaw.skills.remember import configure, set_current_user, set_learning_engine
        self.memory = Memory(":memory:")
        self.engine = LearningEngine(self.memory)
        configure(memory=self.memory)
        set_current_user("testuser")
        set_learning_engine(self.engine)
        yield
        self.memory.close()

    def test_teach_skill(self):
        from amanclaw.skills.remember import teach
        result = teach(rule="when I say deploy, push to staging first", category="work")
        assert "learned" in result.lower() or "got it" in result.lower()
        teachings = self.memory.get_teachings("testuser")
        assert len(teachings) == 1

    def test_learned_skill(self):
        from amanclaw.skills.remember import learned
        self.memory.save_knowledge("testuser", "preference", "coffee", "latte", source="conversation")
        result = learned()
        assert "coffee" in result or "latte" in result

    def test_forget_skill(self):
        from amanclaw.skills.remember import forget
        self.memory.save_knowledge("testuser", "preference", "coffee", "latte")
        result = forget(query="coffee")
        assert "forgot" in result.lower() or "removed" in result.lower()
```

**Step 2: Run tests to verify they fail**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_learning.py::TestTeachingSkill -v`
Expected: FAIL

**Step 3: Add new skills to remember.py**

Add to `amanclaw/skills/remember.py`:

```python
_learning_engine = None


def set_learning_engine(engine):
    global _learning_engine
    _learning_engine = engine


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
```

Also add `import re` at the top of the file if not already there.

**Step 4: Run tests to verify they pass**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_learning.py::TestTeachingSkill -v`
Expected: All PASS

**Step 5: Update skill registry test**

In `tests/test_skills.py`, update the `test_skills_registered` expected set:

```python
    def test_skills_registered(self):
        expected = {"run_command", "read_file", "write_file", "list_files",
                    "system_status", "save_fact", "get_facts", "recall",
                    "set_reminder", "list_reminders", "cancel_reminder",
                    "teach", "learned", "forget"}
        assert expected.issubset(set(REGISTRY.keys()))
```

**Step 6: Run all tests**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All PASS

**Step 7: Commit**

```bash
git add amanclaw/skills/remember.py tests/test_skills.py tests/test_learning.py
git commit -m "feat: add teach, learned, and forget skills"
```

---

### Task 4: Add Document Ingestion Skill

**Files:**
- Modify: `amanclaw/skills/documents.py`
- Test: `tests/test_learning.py`

**Step 1: Write failing tests**

Add to `tests/test_learning.py`:

```python
class TestDocumentIngestionSkill:
    @pytest.fixture(autouse=True)
    def setup(self):
        from amanclaw.memory import Memory
        from amanclaw.learning import LearningEngine
        from amanclaw.skills.documents import configure as configure_docs, set_learning_context
        import tempfile
        self.memory = Memory(":memory:")
        self.engine = LearningEngine(self.memory)
        self.tmpdir = tempfile.mkdtemp()
        configure_docs(workspace_dir=self.tmpdir)
        set_learning_context("testuser", self.engine)
        yield
        self.memory.close()

    def test_learn_document(self):
        from amanclaw.skills.documents import learn_document
        from pathlib import Path
        # Create a test file
        test_file = Path(self.tmpdir) / "notes.txt"
        test_file.write_text("Python is great for automation. Rust is great for performance.")
        result = learn_document("notes.txt")
        assert "learned" in result.lower() or "ingested" in result.lower()
        docs = self.memory.list_documents("testuser")
        assert len(docs) == 1
```

**Step 2: Run tests to verify they fail**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_learning.py::TestDocumentIngestionSkill -v`
Expected: FAIL

**Step 3: Add learn_document skill to documents.py**

Add to `amanclaw/skills/documents.py`:

```python
_current_user_id = None
_learning_engine = None


def set_learning_context(user_id: str, engine=None):
    global _current_user_id, _learning_engine
    _current_user_id = user_id
    _learning_engine = engine


@skill(
    name="learn_document",
    description="Ingest and learn from a document file in the workspace. After learning, I can answer questions about its content. Supported: TXT, MD, CSV, JSON, YAML, PDF.",
    parameters={
        "path": {
            "type": "string",
            "description": "Relative path to the document in the workspace",
        },
    },
    timeout=30,
)
def learn_document(path: str) -> str:
    if not _current_user_id or not _learning_engine:
        return "Error: Learning context not available."
    try:
        safe = _safe_path(path)
        if not safe.exists():
            return f"File not found: {path}"

        suffix = safe.suffix.lower()
        if suffix == ".pdf":
            text = _read_pdf(safe, max_chars=50000)
        elif suffix in (".txt", ".md", ".csv", ".tsv", ".json", ".yaml", ".yml", ".xml", ".html", ".log"):
            text = safe.read_text(encoding="utf-8", errors="replace")
        else:
            return f"Unsupported format: {suffix}"

        if not text or len(text) < 10:
            return "Document is empty or too short to learn from."

        source_type = suffix.lstrip(".")
        count = _learning_engine.ingest_document(_current_user_id, safe.name, source_type, text)
        return f"Learned from '{safe.name}': ingested {count} chunks. I can now answer questions about this document."
    except ValueError as e:
        return str(e)
    except Exception as e:
        return f"Error learning document: {e}"
```

**Step 4: Run tests to verify they pass**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_learning.py::TestDocumentIngestionSkill -v`
Expected: PASS

**Step 5: Commit**

```bash
git add amanclaw/skills/documents.py tests/test_learning.py
git commit -m "feat: add learn_document skill for document ingestion"
```

---

### Task 5: Integrate Learning Engine into Bot

**Files:**
- Modify: `amanclaw/bot.py`
- Modify: `amanclaw/llm.py`

**Step 1: Initialize learning engine in bot.py main()**

In `amanclaw/bot.py`, add import at the top:

```python
from amanclaw.learning import LearningEngine
from amanclaw.skills.documents import set_learning_context as set_doc_learning_context
```

Add to globals section:

```python
learning_engine: LearningEngine = None
```

In `main()`, after `configure_remember(memory=memory)`, add:

```python
    learning_engine = LearningEngine(memory)
    # Wire up learning engine to skills
    from amanclaw.skills.remember import set_learning_engine
    set_learning_engine(learning_engine)
```

**Step 2: Update handle_message to set document learning context**

In `handle_message()`, after the existing `set_*` calls, add:

```python
    set_doc_learning_context(user_id, learning_engine)
```

Do the same in `handle_photo()`.

**Step 3: Add failure tracking to skill execution**

In `amanclaw/bot.py`, modify `extract_and_save_knowledge` to also detect corrections and teachings:

Replace the existing `extract_and_save_knowledge` function:

```python
async def extract_and_save_knowledge(user_id: str, user_msg: str, assistant_reply: str):
    """Background task: extract knowledge, detect corrections and teachings."""
    try:
        # Detect corrections
        if learning_engine and learning_engine.is_correction(user_msg):
            logger.info(f"Correction detected from user {user_id}")
            # The LLM extraction will handle the actual correction via 'updates'

        # Detect teaching intent and save
        if learning_engine and learning_engine.is_teaching(user_msg):
            learning_engine.save_teaching(user_id, user_msg, assistant_reply, "conversation")
            logger.info(f"Teaching detected from user {user_id}")

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

        # Apply updates (corrections)
        for u in extracted.get("updates", []):
            kid = u.get("id")
            if kid and u.get("content"):
                # Log as correction
                if learning_engine:
                    old_entry = memory.conn.execute(
                        "SELECT content FROM knowledge WHERE id = ?", (kid,)
                    ).fetchone()
                    if old_entry:
                        learning_engine.process_correction(
                            user_id, user_msg, kid, old_entry[0], u["content"]
                        )
                else:
                    memory.update_knowledge(kid, content=u["content"])

        count = len(extracted.get("knowledge", [])) + len(extracted.get("entities", []))
        if count:
            logger.info(f"Extracted {count} knowledge items for user {user_id}")

    except Exception as e:
        logger.warning(f"Background knowledge extraction failed for {user_id}: {e}")
```

**Step 4: Enhance context building with teachings and documents**

In `build_context()`, add teachings and document search to the knowledge context:

After the existing knowledge context building, before `return`:

```python
    # Add active teachings to context
    if learning_engine:
        teachings = learning_engine.get_matching_teachings(user_id, message_text)
        if teachings:
            teaching_text = "\n\n### User-taught rules\n"
            for t in teachings:
                teaching_text += f"- {t['trigger_pattern']}: {t['response_guidance']}\n"
            knowledge_context += teaching_text

        # Search ingested documents for relevant chunks
        if message_text:
            doc_results = memory.search_documents(user_id, message_text, limit=3)
            if doc_results:
                doc_text = "\n\n### From learned documents\n"
                for d in doc_results:
                    doc_text += f"[{d['source_name']}]: {d['content'][:300]}\n"
                knowledge_context += doc_text

        # Add behavioral patterns as hints
        patterns = memory.get_behavioral_patterns(user_id, min_confidence=0.6)
        if patterns:
            pattern_text = "\n\n### Observed user preferences\n"
            for p in patterns:
                pattern_text += f"- {p['description']}\n"
            knowledge_context += pattern_text
```

**Step 5: Add /teach, /learned, /forget commands**

In `bot.py`, add command handlers:

```python
async def cmd_teach(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /teach command — enter teaching mode."""
    user_id = str(update.effective_user.id)
    if not auth_check(user_id):
        return
    if not context.args:
        await reply_with_markdown(update.message,
            "*Teaching mode*\n\n"
            "Teach me rules like:\n"
            "`/teach when I say deploy, push to staging first`\n"
            "`/teach always keep answers short about food`\n"
            "`/teach if I ask about servers, check status first`\n\n"
            "Or just tell me naturally in conversation:\n"
            "\"Remember that when I say X, I mean Y\""
        )
        return
    rule = " ".join(context.args)
    set_current_user(user_id)
    from amanclaw.skills.remember import teach
    result = teach(rule=rule)
    await reply_with_markdown(update.message, result)


async def cmd_learned(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /learned command — show learning journal."""
    user_id = str(update.effective_user.id)
    if not auth_check(user_id):
        return
    days = int(context.args[0]) if context.args else 7
    if learning_engine:
        journal = learning_engine.get_learning_journal(user_id, days=days)
    else:
        journal = "Learning engine not initialized."
    await send_long_reply(update.message, journal)


async def cmd_forget(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /forget command — remove specific knowledge."""
    user_id = str(update.effective_user.id)
    if not auth_check(user_id):
        return
    if not context.args:
        await update.message.reply_text("Usage: /forget <topic>\nExample: /forget coffee preference")
        return
    query = " ".join(context.args)
    set_current_user(user_id)
    from amanclaw.skills.remember import forget
    result = forget(query=query)
    await reply_with_markdown(update.message, result)
```

Register handlers in `main()`:

```python
    app.add_handler(CommandHandler("teach", cmd_teach))
    app.add_handler(CommandHandler("learned", cmd_learned))
    app.add_handler(CommandHandler("forget", cmd_forget))
```

Update `post_init` bot commands:

```python
        BotCommand("teach", "Teach me a rule or behavior"),
        BotCommand("learned", "Show what I've learned"),
        BotCommand("forget", "Forget specific knowledge"),
```

**Step 6: Add failure tracking to skill execution in llm.py**

In `amanclaw/llm.py`, the `_respond_native` and `_respond_fallback` methods already execute skills. We need to hook failure logging. The cleanest way is in `bot.py` after the response:

In `handle_message()`, after `response = await llm.respond(...)`, before saving exchange:

This is handled indirectly — failures from skill execution are returned as error strings. We'll track these in a simpler way by checking the response for error patterns. Add after `memory.save_exchange(...)`:

```python
    # Track skill failures in response
    if learning_engine and ("failed:" in response.lower() or "error:" in response.lower()):
        learning_engine.log_failure(user_id, "llm_response", {"message": clean_text[:200]}, response[:500])
```

**Step 7: Run all tests**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All PASS

**Step 8: Commit**

```bash
git add amanclaw/bot.py amanclaw/llm.py
git commit -m "feat: integrate learning engine into bot with /teach, /learned, /forget commands"
```

---

### Task 6: Add Proactive Check-in Job

**Files:**
- Modify: `amanclaw/learning.py`
- Modify: `amanclaw/bot.py`
- Test: `tests/test_learning.py`

**Step 1: Write failing test**

Add to `tests/test_learning.py`:

```python
class TestProactiveCheckins:
    @pytest.fixture
    def engine(self):
        memory = Memory(":memory:")
        engine = LearningEngine(memory)
        yield engine
        memory.close()

    def test_get_checkin_candidates(self, engine):
        # Add old knowledge
        engine.memory.save_knowledge("user1", "preference", "coffee", "americano",
                                     source="conversation")
        # Manually set created_at to 30 days ago
        engine.memory.conn.execute(
            "UPDATE knowledge SET created_at = datetime('now', '-30 days') WHERE subject = 'coffee'"
        )
        engine.memory.conn.commit()
        candidates = engine.get_checkin_candidates("user1", min_age_days=7)
        assert len(candidates) >= 1
        assert candidates[0]["subject"] == "coffee"

    def test_no_checkins_for_recent_knowledge(self, engine):
        engine.memory.save_knowledge("user1", "preference", "coffee", "latte")
        candidates = engine.get_checkin_candidates("user1", min_age_days=7)
        assert len(candidates) == 0

    def test_format_checkin_message(self, engine):
        engine.memory.save_knowledge("user1", "preference", "coffee", "americano")
        engine.memory.conn.execute(
            "UPDATE knowledge SET created_at = datetime('now', '-30 days') WHERE subject = 'coffee'"
        )
        engine.memory.conn.commit()
        candidates = engine.get_checkin_candidates("user1", min_age_days=7)
        msg = engine.format_checkin_message(candidates[:2])
        assert "coffee" in msg.lower()
        assert "still" in msg.lower() or "true" in msg.lower()
```

**Step 2: Run tests to verify they fail**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_learning.py::TestProactiveCheckins -v`
Expected: FAIL

**Step 3: Add checkin methods to learning.py**

Add to `LearningEngine`:

```python
    # --- Proactive Check-ins ---

    def get_checkin_candidates(self, user_id: str, min_age_days: int = 7,
                               limit: int = 5) -> list[dict]:
        cutoff = (datetime.now() - timedelta(days=min_age_days)).strftime("%Y-%m-%d %H:%M:%S")
        rows = self.memory.conn.execute(
            """SELECT id, category, subject, content, context, created_at
               FROM knowledge
               WHERE user_id = ? AND expired = 0 AND created_at <= ?
                 AND source IN ('conversation', 'explicit')
                 AND category IN ('preference', 'personal', 'routine', 'temporal')
               ORDER BY created_at ASC LIMIT ?""",
            (str(user_id), cutoff, limit)
        ).fetchall()
        return [{"id": r[0], "category": r[1], "subject": r[2], "content": r[3],
                 "context": r[4], "created_at": r[5]} for r in rows]

    def format_checkin_message(self, candidates: list[dict]) -> str:
        if not candidates:
            return ""
        lines = ["Just checking in on a few things I remember:\n"]
        for c in candidates[:2]:
            context = f" ({c['context']})" if c.get("context") else ""
            lines.append(f"- Is it still true that your {c['subject']} is \"{c['content']}\"{context}?")
        lines.append("\nLet me know if anything changed!")
        return "\n".join(lines)
```

**Step 4: Run tests to verify they pass**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_learning.py::TestProactiveCheckins -v`
Expected: All PASS

**Step 5: Add weekly check-in job to bot.py**

In `bot.py`, add the check-in job function:

```python
async def checkin_job(context: ContextTypes.DEFAULT_TYPE):
    """Weekly job to send proactive check-in messages."""
    if not learning_engine:
        return
    # Get all active users
    users = memory.list_users(status="approved")
    admin_ids = [str(uid) for uid in config.get("admin_users", {}).get("telegram", [])]
    all_user_ids = set(u["user_id"] for u in users) | set(admin_ids)

    for user_id in all_user_ids:
        candidates = learning_engine.get_checkin_candidates(user_id, min_age_days=14)
        if not candidates:
            continue
        msg = learning_engine.format_checkin_message(candidates)
        if not msg:
            continue
        try:
            await context.bot.send_message(
                chat_id=int(user_id),
                text=msg,
                parse_mode=ParseMode.MARKDOWN,
            )
            logger.info(f"Sent proactive check-in to user {user_id}")
        except Exception as e:
            logger.debug(f"Failed to send check-in to {user_id}: {e}")
```

In `main()`, register the weekly job (after the daily prune job):

```python
    # Schedule weekly proactive check-in (Sundays at 10:00 AM)
    app.job_queue.run_daily(checkin_job, time=datetime_time(hour=10, minute=0),
                            days=(6,))  # 6 = Sunday
```

**Step 6: Run all tests**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All PASS

**Step 7: Commit**

```bash
git add amanclaw/learning.py amanclaw/bot.py tests/test_learning.py
git commit -m "feat: add proactive check-in job and learning journal"
```

---

### Task 7: Enhance System Prompt with Learning Context

**Files:**
- Modify: `amanclaw/llm.py`

**Step 1: Update the system prompt to include learning awareness**

In `amanclaw/llm.py`, add to `SYSTEM_PROMPT_BASE` after the Memory section:

```python
## Learning
- You are a self-learning assistant. You improve over time by learning from conversations.
- When the user corrects you, acknowledge the correction: "Got it, updated — [old] -> [new]"
- When the user teaches you a rule, confirm: "Got it, I've learned: [rule]"
- When you use knowledge from a learned document, mention the source briefly.
- Use the 'teach' tool when the user wants to set a rule for how you behave.
- Use the 'learned' tool when the user asks what you've learned or your learning journal.
- Use the 'forget' tool when the user wants you to forget something.
- Use the 'learn_document' tool when the user sends a document and says 'learn this' or 'remember this'.
- Check the "User-taught rules" section in your context — these are high-priority instructions from the user.
```

**Step 2: Update the extraction prompt to better detect corrections**

In `amanclaw/llm.py`, update `EXTRACTION_PROMPT` rules section:

```python
Rules:
- Only extract NEW or CHANGED information. Skip greetings and small talk.
- If the user corrects a previous fact, include it in "updates" with the knowledge ID.
- Detect corrections: "no I meant X", "actually it's X", "wrong, it's X", "not X, it's Y".
- Set valid_until for temporary facts (diets, deadlines, trips).
- When the user teaches a rule ("always do X", "when I say Y, do Z"), extract as category "preference" with the rule as content.
- Return empty arrays if nothing to extract.
- Return ONLY the JSON object, no markdown fences or extra text.
```

**Step 3: Run all tests**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All PASS

**Step 4: Commit**

```bash
git add amanclaw/llm.py
git commit -m "feat: enhance system prompt and extraction for self-learning awareness"
```

---

### Task 8: Update Bot Commands Menu and Config

**Files:**
- Modify: `amanclaw/bot.py`
- Modify: `config.example.yaml`

**Step 1: Update config.example.yaml with learning settings**

Add to `config.example.yaml`:

```yaml
# Learning Engine
learning:
  enabled: true
  proactive_checkins: true
  checkin_day: 6           # 0=Monday, 6=Sunday
  checkin_hour: 10
  checkin_min_age_days: 14  # Only check facts older than this
  document_max_chars: 50000 # Max chars to ingest per document
```

**Step 2: Wire config into learning engine initialization in bot.py**

In `main()`, update learning engine init:

```python
    learning_config = config.get("learning", {})
    if learning_config.get("enabled", True):
        learning_engine = LearningEngine(memory)
        from amanclaw.skills.remember import set_learning_engine
        set_learning_engine(learning_engine)
        logger.info("Learning engine initialized")
```

Update checkin job registration to use config:

```python
    if learning_config.get("proactive_checkins", True):
        checkin_day = learning_config.get("checkin_day", 6)
        checkin_hour = learning_config.get("checkin_hour", 10)
        app.job_queue.run_daily(checkin_job, time=datetime_time(hour=checkin_hour, minute=0),
                                days=(checkin_day,))
```

**Step 3: Run all tests**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All PASS

**Step 4: Commit**

```bash
git add amanclaw/bot.py config.example.yaml
git commit -m "feat: add learning config and finalize self-learning integration"
```

---

### Task 9: Final Integration Test

**Files:**
- Test: `tests/test_learning.py`

**Step 1: Write integration test**

Add to `tests/test_learning.py`:

```python
class TestLearningIntegration:
    @pytest.fixture
    def setup_all(self):
        memory = Memory(":memory:")
        engine = LearningEngine(memory)
        yield memory, engine
        memory.close()

    def test_full_learning_lifecycle(self, setup_all):
        memory, engine = setup_all
        user = "user1"

        # 1. Bot learns from conversation
        memory.save_knowledge(user, "preference", "coffee", "americano", source="conversation")

        # 2. User corrects
        kid = memory.get_active_knowledge(user)[0]["id"]
        engine.process_correction(user, "no I prefer latte", kid, "americano", "latte")
        assert memory.get_active_knowledge(user)[0]["content"] == "latte"
        assert len(memory.get_corrections(user)) == 1

        # 3. User teaches a rule
        engine.save_teaching(user, "when I say deploy", "push to staging first", "work")
        teachings = memory.get_teachings(user)
        assert len(teachings) == 1

        # 4. User sends a document
        engine.ingest_document(user, "notes.txt", "txt", "Python is great. Rust is fast.")
        docs = memory.list_documents(user)
        assert len(docs) == 1

        # 5. A skill fails
        engine.log_failure(user, "web_search", {"query": "test"}, "timeout error")
        assert len(memory.get_recent_failures(user)) == 1

        # 6. Learning journal shows everything
        journal = engine.get_learning_journal(user)
        assert "latte" in journal
        assert "deploy" in journal or "staging" in journal
        assert "notes.txt" in journal
        assert "web_search" in journal or "failure" in journal.lower()

        # 7. Proactive check-in
        memory.conn.execute(
            "UPDATE knowledge SET created_at = datetime('now', '-30 days')"
        )
        memory.conn.commit()
        candidates = engine.get_checkin_candidates(user, min_age_days=7)
        assert len(candidates) >= 1
        msg = engine.format_checkin_message(candidates)
        assert "still true" in msg.lower() or "still" in msg.lower()
```

**Step 2: Run the integration test**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/test_learning.py::TestLearningIntegration -v`
Expected: PASS

**Step 3: Run ALL tests**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && python -m pytest tests/ -v`
Expected: All PASS

**Step 4: Final commit**

```bash
git add tests/test_learning.py
git commit -m "test: add full learning lifecycle integration test"
```
