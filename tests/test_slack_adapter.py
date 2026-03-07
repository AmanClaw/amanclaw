# tests/test_slack_adapter.py
"""Tests for Slack adapter."""
import pytest
from amanclaw.channels import OutgoingMessage


class TestSlackAdapter:
    def test_import(self):
        from amanclaw.channels.slack import SlackAdapter
        assert SlackAdapter is not None

    def test_platform_name(self):
        from amanclaw.channels.slack import SlackAdapter
        adapter = SlackAdapter.__new__(SlackAdapter)
        assert adapter.platform == "slack"

    def test_split_message_short(self):
        from amanclaw.channels.slack import SlackAdapter
        adapter = SlackAdapter.__new__(SlackAdapter)
        chunks = adapter._split_message("short")
        assert chunks == ["short"]

    def test_split_message_long(self):
        from amanclaw.channels.slack import SlackAdapter
        adapter = SlackAdapter.__new__(SlackAdapter)
        long_text = "B" * 5000
        chunks = adapter._split_message(long_text)
        assert all(len(c) <= 4000 for c in chunks)
