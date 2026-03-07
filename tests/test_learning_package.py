"""Tests for standalone amanclaw-learning package."""
import pytest
from amanclaw_learning import LearningEngine, MemoryBackend
from amanclaw_learning.patterns import CORRECTION_PATTERNS, TEACHING_PATTERNS


class MockMemoryBackend:
    """Minimal mock implementing MemoryBackend protocol."""

    def __init__(self):
        self._knowledge = {}
        self._corrections = []
        self._teachings = []
        self._documents = {}
        self._failures = []
        self._patterns = []
        self._next_id = 1

    def get_active_knowledge(self, user_id):
        return [k for k in self._knowledge.values() if k.get("user_id") == user_id]

    def save_knowledge(self, user_id, category, subject, content, **kwargs):
        kid = self._next_id
        self._next_id += 1
        self._knowledge[kid] = {"id": kid, "user_id": user_id, "category": category,
                                 "subject": subject, "content": content, **kwargs}
        return kid

    def update_knowledge(self, knowledge_id, **kwargs):
        if knowledge_id in self._knowledge:
            self._knowledge[knowledge_id].update(kwargs)

    def save_correction(self, user_id, knowledge_id, old_content, new_content, trigger_text):
        cid = self._next_id
        self._next_id += 1
        self._corrections.append({"id": cid, "user_id": user_id, "knowledge_id": knowledge_id,
                                   "old_content": old_content, "new_content": new_content,
                                   "trigger_text": trigger_text, "created_at": "2026-01-01"})
        return cid

    def get_corrections(self, user_id, limit=10):
        return [c for c in self._corrections if c["user_id"] == user_id][:limit]

    def save_teaching(self, user_id, trigger_pattern, response_guidance, category):
        tid = self._next_id
        self._next_id += 1
        self._teachings.append({"id": tid, "user_id": user_id, "trigger_pattern": trigger_pattern,
                                 "response_guidance": response_guidance, "category": category,
                                 "active": 1, "usage_count": 0, "created_at": "2026-01-01"})
        return tid

    def get_teachings(self, user_id, active_only=True):
        teachings = [t for t in self._teachings if t["user_id"] == user_id]
        if active_only:
            teachings = [t for t in teachings if t["active"]]
        return teachings

    def increment_teaching_usage(self, teaching_id):
        for t in self._teachings:
            if t["id"] == teaching_id:
                t["usage_count"] += 1

    def deactivate_teaching(self, teaching_id):
        for t in self._teachings:
            if t["id"] == teaching_id:
                t["active"] = 0

    def save_document_chunk(self, user_id, source_name, source_type, chunk_index, text):
        key = (user_id, source_name)
        if key not in self._documents:
            self._documents[key] = []
        self._documents[key].append({"chunk_index": chunk_index, "content": text})

    def delete_document(self, user_id, source_name):
        self._documents.pop((user_id, source_name), None)

    def list_documents(self, user_id):
        result = []
        for (uid, name), chunks in self._documents.items():
            if uid == user_id:
                result.append({"source_name": name, "source_type": "text", "chunks": len(chunks)})
        return result

    def save_failure(self, user_id, skill_name, input_json, error_message):
        fid = self._next_id
        self._next_id += 1
        self._failures.append({"id": fid, "user_id": user_id, "skill_name": skill_name,
                                "skill_input": input_json, "error_message": error_message,
                                "resolved": 0, "created_at": "2026-01-01"})
        return fid

    def get_recent_failures(self, user_id, limit=20):
        return [f for f in self._failures if f["user_id"] == user_id][:limit]

    def get_behavioral_patterns(self, user_id, min_confidence=0.5):
        return [p for p in self._patterns if p["user_id"] == user_id and p.get("confidence", 0) >= min_confidence]


class TestMemoryBackendProtocol:
    def test_mock_satisfies_protocol(self):
        backend = MockMemoryBackend()
        assert isinstance(backend, MemoryBackend)


class TestLearningEngineWithMock:
    @pytest.fixture
    def engine(self):
        backend = MockMemoryBackend()
        return LearningEngine(backend)

    def test_is_correction(self, engine):
        assert engine.is_correction("No, I meant Python")
        assert engine.is_correction("Actually, it's JavaScript")
        assert not engine.is_correction("Hello there")

    def test_is_teaching(self, engine):
        assert engine.is_teaching("Remember that I prefer dark mode")
        assert engine.is_teaching("Always respond in English")
        assert not engine.is_teaching("What's the weather?")

    def test_process_correction(self, engine):
        kid = engine.memory.save_knowledge("u1", "pref", "lang", "Python")
        result = engine.process_correction("u1", "No I meant JS", kid, "Python", "JavaScript")
        assert result is True
        assert engine.memory._knowledge[kid]["content"] == "JavaScript"

    def test_save_teaching(self, engine):
        tid = engine.save_teaching("u1", "when I say hi", "respond casually", "greeting")
        assert tid is not None
        teachings = engine.memory.get_teachings("u1")
        assert len(teachings) == 1

    def test_chunk_text(self, engine):
        text = "A" * 1200
        chunks = engine.chunk_text(text, chunk_size=500)
        assert len(chunks) == 3
        assert "".join(chunks) == text

    def test_ingest_document(self, engine):
        count = engine.ingest_document("u1", "test.txt", "text", "Hello world. " * 100)
        assert count >= 1

    def test_log_failure(self, engine):
        fid = engine.log_failure("u1", "web_search", {"q": "test"}, "timeout")
        assert fid is not None

    def test_patterns_exist(self):
        assert len(CORRECTION_PATTERNS) > 0
        assert len(TEACHING_PATTERNS) > 0
