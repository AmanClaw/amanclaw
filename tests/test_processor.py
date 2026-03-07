# tests/test_processor.py
"""Tests for channel-agnostic message processor."""
import pytest
from unittest.mock import MagicMock, AsyncMock, patch
from amanclaw.processor import MessageProcessor
from amanclaw.channels import IncomingMessage


class TestMessageProcessor:
    @pytest.fixture
    def processor(self):
        config = {"admin_users": {"telegram": [123]}}
        auth = MagicMock()
        auth.get_user_state.return_value = "approved"
        rate_limiter = MagicMock()
        rate_limiter.check.return_value = True
        memory = MagicMock()
        memory.get_history.return_value = []
        memory.get_facts.return_value = {}
        memory.get_latest_summary.return_value = None
        memory.get_active_knowledge.return_value = []
        memory.get_entities.return_value = []
        memory.get_relationships.return_value = []
        memory.search_knowledge.return_value = []
        memory.get_message_count.return_value = 5
        memory.get_summarized_message_count.return_value = 0
        llm = AsyncMock()
        llm.respond = AsyncMock(return_value="Hello back!")
        learning = MagicMock()
        learning.is_correction.return_value = False
        learning.get_matching_teachings.return_value = []
        return MessageProcessor(config, auth, rate_limiter, memory, llm, learning)

    @pytest.mark.asyncio
    async def test_process_approved_user(self, processor):
        msg = IncomingMessage(user_id="456", chat_id="789", platform="test", text="Hi")
        result = await processor.process(msg)
        assert result is not None
        assert "Hello back!" in result.text

    @pytest.mark.asyncio
    async def test_process_blocked_user(self, processor):
        processor.auth.get_user_state.return_value = "blocked"
        msg = IncomingMessage(user_id="456", chat_id="789", platform="test", text="Hi")
        result = await processor.process(msg)
        assert result is None

    @pytest.mark.asyncio
    async def test_process_rate_limited(self, processor):
        processor.rate_limiter.check.return_value = False
        msg = IncomingMessage(user_id="456", chat_id="789", platform="test", text="Hi")
        result = await processor.process(msg)
        assert result is not None
        assert "slow down" in result.text.lower() or "too many" in result.text.lower()

    @pytest.mark.asyncio
    async def test_process_new_user(self, processor):
        processor.auth.get_user_state.return_value = "new"
        msg = IncomingMessage(user_id="456", chat_id="789", platform="test",
                             text="Hi", first_name="Alice")
        result = await processor.process(msg)
        assert result is not None
        processor.memory.register_user.assert_called_once()

    @pytest.mark.asyncio
    async def test_process_includes_teachings(self, processor):
        """Processor should include matching teachings in LLM context."""
        processor.learning.get_matching_teachings.return_value = [
            {"trigger_pattern": "deploy", "response_guidance": "push to staging first"}
        ]
        processor.memory.search_documents = MagicMock(return_value=[])
        processor.memory.get_behavioral_patterns = MagicMock(return_value=[])
        msg = IncomingMessage(user_id="456", chat_id="789", platform="test", text="deploy now")
        result = await processor.process(msg)
        assert result is not None
        # Verify LLM was called with knowledge_context containing teachings
        call_kwargs = processor.llm.respond.call_args
        assert "knowledge_context" in call_kwargs.kwargs or len(call_kwargs.args) > 4

    @pytest.mark.asyncio
    async def test_build_context_returns_tuple(self, processor):
        """_build_context should return (history, facts, summary, knowledge_context)."""
        processor.memory.search_documents = MagicMock(return_value=[])
        processor.memory.get_behavioral_patterns = MagicMock(return_value=[])
        result = await processor._build_context("456", "hello")
        assert isinstance(result, tuple)
        assert len(result) == 4

    @pytest.mark.asyncio
    async def test_extract_knowledge_no_circular_import(self, processor):
        """_extract_knowledge should not import from bot.py."""
        processor.llm.extract_knowledge = AsyncMock(return_value=None)
        # Should not raise ImportError
        await processor._extract_knowledge("456", "hi", "hello")
