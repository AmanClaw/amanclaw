# tests/test_discord_adapter.py
"""Tests for Discord adapter."""
import pytest
from unittest.mock import MagicMock, AsyncMock, patch
from amanclaw.channels import OutgoingMessage


class TestDiscordAdapter:
    def test_import(self):
        from amanclaw.channels.discord import DiscordAdapter
        assert DiscordAdapter is not None

    def test_platform_name(self):
        from amanclaw.channels.discord import DiscordAdapter
        adapter = DiscordAdapter.__new__(DiscordAdapter)
        assert adapter.platform == "discord"

    def test_split_message_short(self):
        from amanclaw.channels.discord import DiscordAdapter
        adapter = DiscordAdapter.__new__(DiscordAdapter)
        chunks = adapter._split_message("short message")
        assert chunks == ["short message"]

    def test_split_message_long(self):
        from amanclaw.channels.discord import DiscordAdapter
        adapter = DiscordAdapter.__new__(DiscordAdapter)
        long_text = "A" * 2500
        chunks = adapter._split_message(long_text)
        assert len(chunks) == 2
        assert all(len(c) <= 2000 for c in chunks)
