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
