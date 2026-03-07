# tests/test_whatsapp_adapter.py
"""Tests for WhatsApp channel adapter."""
import pytest
from unittest.mock import MagicMock, AsyncMock
from amanclaw.channels import OutgoingMessage
from amanclaw.channels.whatsapp import WhatsAppAdapter


class TestWhatsAppAdapter:
    @pytest.fixture
    def adapter(self):
        config = {
            "whatsapp": {
                "bridge_url": "http://localhost:3001",
                "port": 3002,
                "ignore_groups": True,
            }
        }
        processor = MagicMock()
        processor.process = AsyncMock(return_value=OutgoingMessage(
            chat_id="123@s.whatsapp.net", text="Hello!"
        ))
        return WhatsAppAdapter(config, processor)

    def test_platform(self, adapter):
        assert adapter.platform == "whatsapp"

    def test_split_message_short(self, adapter):
        assert adapter._split_message("short") == ["short"]

    def test_split_message_long(self, adapter):
        long_text = "x" * 5000
        chunks = adapter._split_message(long_text)
        assert len(chunks) > 1
        assert all(len(c) <= 4096 for c in chunks)

    def test_bridge_url_from_config(self, adapter):
        assert adapter.bridge_url == "http://localhost:3001"

    def test_ignore_groups_default(self, adapter):
        assert adapter.ignore_groups is True

    def test_backward_compat_import(self):
        """Old import path still works."""
        from amanclaw.whatsapp import WhatsAppAdapter as WA
        assert WA is WhatsAppAdapter
