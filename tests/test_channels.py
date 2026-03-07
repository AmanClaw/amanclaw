# tests/test_channels.py
"""Tests for channel adapter abstraction."""
import pytest
from amanclaw.channels import ChannelAdapter, IncomingMessage, OutgoingMessage


class TestIncomingMessage:
    def test_basic_creation(self):
        msg = IncomingMessage(user_id="123", chat_id="456", platform="test", text="hello")
        assert msg.user_id == "123"
        assert msg.platform == "test"
        assert msg.image_data is None

    def test_with_image(self):
        msg = IncomingMessage(user_id="123", chat_id="456", platform="test",
                             text="look at this", image_data=b"\x89PNG")
        assert msg.image_data == b"\x89PNG"


class TestOutgoingMessage:
    def test_basic_creation(self):
        msg = OutgoingMessage(chat_id="456", text="hi there")
        assert msg.parse_mode is None


class TestChannelAdapterABC:
    def test_cannot_instantiate(self):
        with pytest.raises(TypeError):
            ChannelAdapter()
