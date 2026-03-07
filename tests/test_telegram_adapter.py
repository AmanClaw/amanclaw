# tests/test_telegram_adapter.py
"""Tests for Telegram channel adapter."""
import pytest
from unittest.mock import MagicMock, AsyncMock
from amanclaw.channels.telegram import TelegramAdapter


class TestTelegramAdapter:
    @pytest.fixture
    def adapter(self):
        config = {"admin_users": {"telegram": [123]}}
        processor = MagicMock()
        processor.auth = MagicMock()
        processor.auth.is_authorized.return_value = True
        processor.rate_limiter = MagicMock()
        memory = MagicMock()
        llm = AsyncMock()
        learning = MagicMock()
        return TelegramAdapter(config, processor, memory, llm, learning)

    def test_platform(self, adapter):
        assert adapter.platform == "telegram"

    def test_split_message_short(self, adapter):
        chunks = adapter._split_long_text("short")
        assert chunks == ["short"]

    def test_split_message_long(self, adapter):
        long_text = "x" * 5000
        chunks = adapter._split_long_text(long_text)
        assert len(chunks) > 1
        assert all(len(c) <= 4000 for c in chunks)

    def test_has_register_handlers(self, adapter):
        """TelegramAdapter must have a register_handlers method."""
        assert hasattr(adapter, 'register_handlers')
        assert callable(adapter.register_handlers)

    def test_auth_check_delegates_to_processor(self, adapter):
        adapter.auth_check("123")
        adapter.processor.auth.is_authorized.assert_called_with("123", "telegram")

    def test_addskill_state_isolated(self, adapter):
        """Each adapter instance has its own addskill state."""
        assert adapter._addskill_state == {}
        adapter._addskill_state["user1"] = {"step": "describe"}
        assert "user1" in adapter._addskill_state
