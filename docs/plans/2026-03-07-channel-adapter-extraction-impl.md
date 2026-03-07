# Channel Adapter Extraction — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extract Telegram adapter from `bot.py` into `channels/telegram.py`, move WhatsApp adapter from `amanclaw/whatsapp.py` to `channels/whatsapp.py`, and consolidate shared logic into `processor.py`.

**Architecture:** Three-phase extraction. Phase 1 enhances `processor.py` with the full `build_context` and `extract_and_save_knowledge` logic (eliminating the circular import). Phase 2 moves WhatsApp into `channels/` using `MessageProcessor`. Phase 3 extracts Telegram into `channels/telegram.py` and slims `bot.py` to orchestration only.

**Tech Stack:** Python 3.11+, python-telegram-bot 21.7, aiohttp, existing SQLite/async stack.

---

## Phase 1: Enhance MessageProcessor

### Task 1.1: Move `build_context` into MessageProcessor

**Files:**
- Modify: `amanclaw/processor.py:1-152`
- Modify: `tests/test_processor.py`

**Step 1: Write the failing test**

Add to `tests/test_processor.py`:

```python
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
```

**Step 2: Run test to verify it fails**

Run: `pytest tests/test_processor.py::TestMessageProcessor::test_process_includes_teachings -v`
Expected: FAIL — `search_documents` / `get_behavioral_patterns` not called, or `knowledge_context` missing teachings

**Step 3: Update `processor.py` with full `build_context` logic**

Replace `MessageProcessor.process()` in `amanclaw/processor.py` with the full version that includes teachings, document search, and behavioral patterns (currently only in `bot.py:build_context`):

```python
# amanclaw/processor.py
"""
Channel-agnostic message processing pipeline.

Extracted from bot.py to allow any channel adapter to process messages
through the same auth -> sanitize -> LLM -> learn pipeline.
"""

import asyncio
import logging
from amanclaw.channels import IncomingMessage, OutgoingMessage
from amanclaw.security import sanitize
from amanclaw.skills.remember import set_current_user
from amanclaw.skills.reminder import set_context as set_reminder_context
from amanclaw.skills.scheduled import set_context as set_scheduled_context
from amanclaw.skills.documents import set_learning_context as set_doc_learning_context

logger = logging.getLogger("amanclaw.processor")


class MessageProcessor:
    """Channel-agnostic message processing pipeline."""

    def __init__(self, config, auth, rate_limiter, memory, llm, learning=None):
        self.config = config
        self.auth = auth
        self.rate_limiter = rate_limiter
        self.memory = memory
        self.llm = llm
        self.learning = learning

    async def process(self, msg: IncomingMessage) -> OutgoingMessage | None:
        """
        Full pipeline: auth -> rate limit -> sanitize -> context -> LLM -> learn.
        Returns OutgoingMessage, or None if message should be silently dropped.
        """
        user_id = msg.user_id
        platform = msg.platform

        # --- Auth check ---
        state = self.auth.get_user_state(user_id, platform)

        if state == "blocked":
            return None

        if state == "new":
            self.memory.register_user(
                user_id=user_id,
                platform=platform,
                username=msg.username,
                first_name=msg.first_name,
            )
            return OutgoingMessage(
                chat_id=msg.chat_id,
                text="Welcome! You've been registered.\n\n"
                     "An admin needs to approve your access before you can start chatting. "
                     "Please wait for approval.",
            )

        if state == "pending":
            return OutgoingMessage(
                chat_id=msg.chat_id,
                text="Your registration is pending approval. "
                     "An admin will review your request shortly.",
            )

        # state is "admin" or "approved"

        # --- Rate limit ---
        if not self.rate_limiter.check(user_id):
            return OutgoingMessage(
                chat_id=msg.chat_id,
                text="Slow down — too many messages. Try again in a minute.",
            )

        # --- Sanitize ---
        clean_text, was_flagged = sanitize(msg.text)
        if was_flagged:
            logger.warning(f"Flagged message from {user_id} on {platform}: {msg.text[:100]}")

        # --- Set skill context ---
        set_current_user(user_id)
        set_reminder_context(user_id, msg.chat_id)
        set_scheduled_context(user_id, msg.chat_id)
        if self.learning:
            set_doc_learning_context(user_id, self.learning)

        # --- Build context ---
        history, facts, summary, knowledge_context = await self._build_context(user_id, clean_text)

        # --- LLM response ---
        try:
            response = await self.llm.respond(
                clean_text, history, flagged=was_flagged,
                facts=facts, summary=summary,
                knowledge_context=knowledge_context,
            )
        except Exception as e:
            logger.error(f"LLM error: {e}")
            response = "Something went wrong talking to the AI. Try again in a moment."

        # --- Save exchange ---
        self.memory.save_exchange(user_id, platform, msg.text, response)

        # --- Background learning ---
        if self.learning:
            asyncio.create_task(self._extract_knowledge(user_id, msg.text, response))

            if "failed:" in response.lower() or "error:" in response.lower():
                self.learning.log_failure(user_id, "llm_response",
                                          {"message": clean_text[:200]}, response[:500])

        return OutgoingMessage(chat_id=msg.chat_id, text=response)

    async def _build_context(self, user_id: str, message_text: str = "") -> tuple[list, dict, str, str]:
        """Build the smart context: history, facts, summary, knowledge context.
        Auto-summarize if needed."""
        history = self.memory.get_history(user_id)
        facts = self.memory.get_facts(user_id)
        summary = self.memory.get_latest_summary(user_id)

        # Build knowledge graph context
        knowledge_entries = self.memory.get_active_knowledge(user_id)
        entities = self.memory.get_entities(user_id)
        relationships = self.memory.get_relationships(user_id)

        # Search for relevant knowledge based on message
        if message_text:
            relevant = self.memory.search_knowledge(user_id, message_text, limit=5)
            existing_ids = {k["id"] for k in knowledge_entries}
            for r in relevant:
                if r["id"] not in existing_ids:
                    knowledge_entries.append(r)

        from amanclaw.llm import format_knowledge_context
        knowledge_context = format_knowledge_context(knowledge_entries, entities, relationships)

        # Auto-summarize when conversation gets long
        msg_count = self.memory.get_message_count(user_id)
        summarized_count = self.memory.get_summarized_message_count(user_id)
        unsummarized = msg_count - summarized_count
        if unsummarized > 40:
            old_msgs = self.memory.get_old_messages(user_id, before_last_n=20, limit=40)
            if old_msgs:
                try:
                    new_summary = await self.llm.summarize(old_msgs, summary)
                    if new_summary:
                        if summary:
                            new_summary = f"{summary}\n\n{new_summary}"
                        self.memory.save_summary(user_id, new_summary, len(old_msgs))
                        summary = new_summary
                        logger.info(f"Auto-summarized {len(old_msgs)} messages for user {user_id}")
                except Exception as e:
                    logger.error(f"Summarization failed: {e}")

        # Add teachings, documents, behavioral patterns if learning is enabled
        if self.learning:
            teachings = self.learning.get_matching_teachings(user_id, message_text)
            if teachings:
                teaching_text = "\n\n### User-taught rules\n"
                for t in teachings:
                    teaching_text += f"- {t['trigger_pattern']}: {t['response_guidance']}\n"
                knowledge_context += teaching_text

            if message_text:
                doc_results = self.memory.search_documents(user_id, message_text, limit=3)
                if doc_results:
                    doc_text = "\n\n### From learned documents\n"
                    for d in doc_results:
                        doc_text += f"[{d['source_name']}]: {d['content'][:300]}\n"
                    knowledge_context += doc_text

            patterns = self.memory.get_behavioral_patterns(user_id, min_confidence=0.6)
            if patterns:
                pattern_text = "\n\n### Observed user preferences\n"
                for p in patterns:
                    pattern_text += f"- {p['description']}\n"
                knowledge_context += pattern_text

        return history, facts, summary, knowledge_context

    async def _extract_knowledge(self, user_id: str, user_msg: str, assistant_reply: str):
        """Background task: extract knowledge, detect corrections and teachings."""
        try:
            # Detect corrections
            if self.learning and self.learning.is_correction(user_msg):
                logger.info(f"Correction detected from user {user_id}")

            # Detect teaching intent and save
            if self.learning and self.learning.is_teaching(user_msg):
                self.learning.save_teaching(user_id, user_msg, assistant_reply, "conversation")
                logger.info(f"Teaching detected from user {user_id}")

            # Get existing knowledge for dedup context
            existing = self.memory.get_active_knowledge(user_id)
            existing_summary = "\n".join(
                f"- [{e['category']}] {e['subject']}: {e['content']}" for e in existing[:20]
            )

            extracted = await self.llm.extract_knowledge(user_msg, assistant_reply, existing_summary)
            if not extracted:
                return

            # Save knowledge entries
            for k in extracted.get("knowledge", []):
                self.memory.save_knowledge(
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
                eid = self.memory.save_entity(
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
                    ent = self.memory.get_entity_by_name(user_id, from_name)
                    from_id = ent["id"] if ent else None
                if not to_id:
                    ent = self.memory.get_entity_by_name(user_id, to_name)
                    to_id = ent["id"] if ent else None
                if from_id and to_id:
                    self.memory.save_relationship(user_id, from_id, r.get("relation", "related_to"), to_id)

            # Apply updates (corrections)
            for u in extracted.get("updates", []):
                kid = u.get("id")
                if kid and u.get("content"):
                    if self.learning:
                        old_entry = self.memory.conn.execute(
                            "SELECT content FROM knowledge WHERE id = ?", (kid,)
                        ).fetchone()
                        if old_entry:
                            self.learning.process_correction(
                                user_id, user_msg, kid, old_entry[0], u["content"]
                            )
                    else:
                        self.memory.update_knowledge(kid, content=u["content"])

            count = len(extracted.get("knowledge", [])) + len(extracted.get("entities", []))
            if count:
                logger.info(f"Extracted {count} knowledge items for user {user_id}")

        except Exception as e:
            logger.warning(f"Background knowledge extraction failed for {user_id}: {e}")
```

**Step 4: Run test to verify it passes**

Run: `pytest tests/test_processor.py -v`
Expected: ALL PASS

**Step 5: Commit**

```bash
git add amanclaw/processor.py tests/test_processor.py
git commit -m "refactor: consolidate build_context and extract_knowledge into MessageProcessor"
```

---

## Phase 2: Move WhatsApp to channels/

### Task 2.1: Create `channels/whatsapp.py` using MessageProcessor

**Files:**
- Create: `amanclaw/channels/whatsapp.py`
- Modify: `amanclaw/whatsapp.py` (becomes re-export)
- Modify: `amanclaw/bot.py:1587` (update WhatsApp adapter construction)

**Step 1: Write the failing test**

Create `tests/test_whatsapp_adapter.py`:

```python
# tests/test_whatsapp_adapter.py
"""Tests for WhatsApp channel adapter."""
import pytest
from unittest.mock import MagicMock, AsyncMock, patch
from amanclaw.channels import IncomingMessage, OutgoingMessage
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

    def test_backward_compat_import(self):
        """Old import path still works."""
        from amanclaw.whatsapp import WhatsAppAdapter as WA
        assert WA is WhatsAppAdapter
```

**Step 2: Run test to verify it fails**

Run: `pytest tests/test_whatsapp_adapter.py -v`
Expected: FAIL — `amanclaw.channels.whatsapp` does not exist

**Step 3: Create `amanclaw/channels/whatsapp.py`**

```python
# amanclaw/channels/whatsapp.py
"""WhatsApp adapter — connects to the Baileys bridge via REST API.

The Baileys bridge (Node.js) handles WhatsApp Web protocol.
This module receives incoming messages via HTTP callback and
sends replies via the bridge's REST API.

Uses MessageProcessor for the auth -> sanitize -> LLM pipeline.
"""

import os
import asyncio
import logging
import aiohttp
from aiohttp import web

from amanclaw.channels import ChannelAdapter, IncomingMessage, OutgoingMessage

logger = logging.getLogger("amanclaw.channels.whatsapp")

MAX_MESSAGE_LENGTH = 4096


class WhatsAppAdapter(ChannelAdapter):
    """Receives messages from the Baileys bridge and processes them through MessageProcessor."""

    def __init__(self, config: dict, processor):
        self.config = config
        self.processor = processor

        wa_config = config.get("whatsapp", {})
        self.bridge_url = (
            os.environ.get("WA_BRIDGE_URL")
            or wa_config.get("bridge_url", "http://localhost:3001")
        ).rstrip("/")
        self.listen_host = wa_config.get("listen", "0.0.0.0")
        self.listen_port = wa_config.get("port", 3002)
        self.ignore_groups = wa_config.get("ignore_groups", True)

        self._session: aiohttp.ClientSession | None = None
        self._app: web.Application | None = None
        self._runner: web.AppRunner | None = None

    @property
    def platform(self) -> str:
        return "whatsapp"

    def _get_session(self) -> aiohttp.ClientSession:
        if self._session is None or self._session.closed:
            self._session = aiohttp.ClientSession()
        return self._session

    # ------------------------------------------------------------------ #
    #  Sending messages via the bridge                                    #
    # ------------------------------------------------------------------ #

    async def send_message(self, msg: OutgoingMessage) -> None:
        """Send a message via the Baileys bridge."""
        jid = msg.chat_id if "@" in msg.chat_id else f"{msg.chat_id}@s.whatsapp.net"
        await self._send_text(jid, msg.text)

    async def _send_text(self, jid: str, text: str):
        """Send a text message via the Baileys bridge."""
        session = self._get_session()
        chunks = self._split_message(text)

        for chunk in chunks:
            try:
                async with session.post(
                    f"{self.bridge_url}/send",
                    json={"jid": jid, "text": chunk},
                    timeout=aiohttp.ClientTimeout(total=30),
                ) as resp:
                    if resp.status != 200:
                        body = await resp.text()
                        logger.error(f"Bridge send failed ({resp.status}): {body}")
            except Exception as e:
                logger.error(f"Failed to send WhatsApp message: {e}")

    async def check_health(self) -> dict | None:
        """Check bridge connection status."""
        try:
            session = self._get_session()
            async with session.get(
                f"{self.bridge_url}/health",
                timeout=aiohttp.ClientTimeout(total=5),
            ) as resp:
                if resp.status == 200:
                    return await resp.json()
        except Exception:
            pass
        return None

    @staticmethod
    def _split_message(text: str) -> list[str]:
        """Split message into chunks that fit WhatsApp's limits."""
        if len(text) <= MAX_MESSAGE_LENGTH:
            return [text]
        chunks = []
        while text:
            if len(text) <= MAX_MESSAGE_LENGTH:
                chunks.append(text)
                break
            cut = text.rfind("\n", 0, MAX_MESSAGE_LENGTH)
            if cut < MAX_MESSAGE_LENGTH // 2:
                cut = MAX_MESSAGE_LENGTH
            chunks.append(text[:cut])
            text = text[cut:].lstrip("\n")
        return chunks

    # ------------------------------------------------------------------ #
    #  Receiving messages from the bridge (HTTP callback)                 #
    # ------------------------------------------------------------------ #

    async def _handle_incoming(self, request: web.Request) -> web.Response:
        """Handle incoming message from the Baileys bridge."""
        try:
            data = await request.json()
        except Exception:
            return web.json_response({"error": "Invalid JSON"}, status=400)

        jid = data.get("jid")
        phone = data.get("from")
        text = data.get("text")
        name = data.get("name", "")
        is_group = data.get("is_group", False)

        if not jid or not text:
            return web.json_response({"error": "Missing jid or text"}, status=400)

        if is_group and self.ignore_groups:
            return web.json_response({"ok": True, "skipped": "group"})

        user_id = phone or jid.split("@")[0]

        logger.info(f"WhatsApp message from {user_id} ({name}): {text[:80]}")

        # Process in background so we don't block the bridge
        asyncio.create_task(self._process_message(user_id, jid, name, text, is_group))

        return web.json_response({"ok": True})

    async def _process_message(self, user_id: str, jid: str, name: str, text: str, is_group: bool = False):
        """Process a WhatsApp message through the MessageProcessor pipeline."""
        try:
            incoming = IncomingMessage(
                user_id=user_id,
                chat_id=jid,
                platform="whatsapp",
                text=text,
                first_name=name or None,
                is_group=is_group,
            )

            result = await self.processor.process(incoming)
            if result:
                await self._send_text(jid, result.text)

        except Exception as e:
            logger.error(f"Error processing WhatsApp message from {user_id}: {e}", exc_info=True)
            try:
                await self._send_text(jid, "Something went wrong. Try again in a moment.")
            except Exception:
                pass

    # ------------------------------------------------------------------ #
    #  Delivery of reminders / schedules for WhatsApp users               #
    # ------------------------------------------------------------------ #

    async def deliver_reminder(self, chat_id: str, message: str):
        """Deliver a reminder to a WhatsApp user."""
        jid = chat_id if "@" in chat_id else f"{chat_id}@s.whatsapp.net"
        await self._send_text(jid, f"Reminder: {message}")

    async def deliver_schedule(self, chat_id: str, message: str):
        """Deliver a scheduled task to a WhatsApp user."""
        jid = chat_id if "@" in chat_id else f"{chat_id}@s.whatsapp.net"
        await self._send_text(jid, f"Scheduled: {message}")

    # ------------------------------------------------------------------ #
    #  Server lifecycle                                                   #
    # ------------------------------------------------------------------ #

    async def start(self) -> None:
        """Start the HTTP callback server."""
        self._app = web.Application()
        self._app.router.add_post("/whatsapp/incoming", self._handle_incoming)
        self._app.router.add_get("/whatsapp/health", self._health_endpoint)

        self._runner = web.AppRunner(self._app)
        await self._runner.setup()

        site = web.TCPSite(self._runner, self.listen_host, self.listen_port)
        await site.start()

        logger.info(f"WhatsApp adapter listening on {self.listen_host}:{self.listen_port}")
        logger.info(f"Bridge URL: {self.bridge_url}")

        health = await self.check_health()
        if health:
            logger.info(f"Bridge status: {health.get('status', 'unknown')}")
        else:
            logger.warning(
                "Could not reach WhatsApp bridge. "
                "Start it with: cd bridge/whatsapp && npm start"
            )

    async def _health_endpoint(self, _request: web.Request) -> web.Response:
        """Health check endpoint."""
        bridge = await self.check_health()
        return web.json_response({
            "adapter": "running",
            "bridge": bridge,
        })

    async def stop(self) -> None:
        """Stop the HTTP server and close session."""
        if self._runner:
            await self._runner.cleanup()
        if self._session and not self._session.closed:
            await self._session.close()
        logger.info("WhatsApp adapter stopped.")
```

**Step 4: Replace `amanclaw/whatsapp.py` with re-export**

```python
# amanclaw/whatsapp.py
"""Backward-compat re-export. Adapter now lives in amanclaw.channels.whatsapp."""
from amanclaw.channels.whatsapp import WhatsAppAdapter

__all__ = ["WhatsAppAdapter"]
```

**Step 5: Update `bot.py` WhatsApp construction (line ~1585-1588)**

Change:
```python
whatsapp = WhatsAppAdapter(config, auth, rate_limiter, memory, llm)
```
To:
```python
from amanclaw.channels.whatsapp import WhatsAppAdapter as WAAdapter
whatsapp = WAAdapter(config, processor)
```

**Step 6: Run tests**

Run: `pytest tests/test_whatsapp_adapter.py tests/test_processor.py tests/test_channels.py -v`
Expected: ALL PASS

**Step 7: Commit**

```bash
git add amanclaw/channels/whatsapp.py amanclaw/whatsapp.py amanclaw/bot.py tests/test_whatsapp_adapter.py
git commit -m "refactor: move WhatsApp adapter to channels/ with MessageProcessor integration"
```

---

## Phase 3: Extract Telegram Adapter

### Task 3.1: Create `channels/telegram.py` with TelegramAdapter class

**Files:**
- Create: `amanclaw/channels/telegram.py`
- Modify: `amanclaw/bot.py` (slim to orchestrator)

**Step 1: Write the failing test**

Create `tests/test_telegram_adapter.py`:

```python
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
```

**Step 2: Run test to verify it fails**

Run: `pytest tests/test_telegram_adapter.py -v`
Expected: FAIL — `amanclaw.channels.telegram` does not exist

**Step 3: Create `amanclaw/channels/telegram.py`**

This is the largest file. Move all Telegram-specific code from `bot.py` lines 147-1374 into `TelegramAdapter`:

```python
# amanclaw/channels/telegram.py
"""Telegram adapter — full-featured Telegram bot with commands, inline keyboards,
photo/voice support, typing indicators, and user skill management."""

import io
import re
import json
import asyncio
import logging
from datetime import datetime

from telegram import Update, InlineKeyboardButton, InlineKeyboardMarkup
from telegram.ext import (
    CommandHandler,
    MessageHandler,
    CallbackQueryHandler,
    ContextTypes,
    filters,
)
from telegram.constants import ParseMode, ChatAction
from telegram.helpers import escape_markdown

from amanclaw.channels import ChannelAdapter, OutgoingMessage
from amanclaw.security import sanitize
from amanclaw.skills import get_skill_list, REGISTRY
from amanclaw.skills.remember import set_current_user
from amanclaw.skills.reminder import set_context as set_reminder_context
from amanclaw.skills.scheduled import set_context as set_scheduled_context
from amanclaw.skills.documents import set_learning_context as set_doc_learning_context

logger = logging.getLogger("amanclaw.channels.telegram")

ADDSKILL_LLM_PROMPT = """You are helping create an API skill integration.
Based on the user's description, generate a complete skill config as JSON.

Rules:
- Find a suitable FREE public API if the user doesn't provide a URL
- Use {param} placeholders in URLs for dynamic parameters
- Keep the name short, lowercase, with underscores
- If an API key is typically needed, set needs_api_key to true
- For well-known services, use known free APIs like:
  - Weather: wttr.in (https://wttr.in/{city}?format=j1)
  - Currency: open.er-api.com
  - IP info: ipapi.co
  - Jokes: official-joke-api.appspot.com
  - Time: worldtimeapi.org
  - Random facts: uselessfacts.jsph.pl

Return ONLY valid JSON (no markdown fences, no explanation):
{"name": "skill_name", "description": "what it does", "url_template": "https://...", "method": "GET", "parameters": {"param_name": {"type": "string", "description": "what this param is"}}, "needs_api_key": false, "headers": {}, "query_params": {}}"""


class TelegramAdapter(ChannelAdapter):
    """Full-featured Telegram bot adapter."""

    def __init__(self, config: dict, processor, memory, llm, learning=None):
        self.config = config
        self.processor = processor
        self.memory = memory
        self.llm = llm
        self.learning = learning
        self._addskill_state: dict[str, dict] = {}

    @property
    def platform(self) -> str:
        return "telegram"

    def auth_check(self, user_id: str) -> bool:
        return self.processor.auth.is_authorized(user_id, "telegram")

    # ------------------------------------------------------------------ #
    #  Helpers                                                            #
    # ------------------------------------------------------------------ #

    @staticmethod
    async def _send_typing_periodically(context, chat_id: int, stop_event: asyncio.Event):
        """Send typing indicator every 4 seconds until stop_event is set."""
        while not stop_event.is_set():
            try:
                await context.bot.send_chat_action(chat_id=chat_id, action=ChatAction.TYPING)
            except Exception:
                break
            try:
                await asyncio.wait_for(stop_event.wait(), timeout=4.0)
            except asyncio.TimeoutError:
                continue

    @staticmethod
    async def _reply_with_markdown(message, text: str):
        """Try to send with Markdown, fall back to plain text if parsing fails."""
        try:
            await message.reply_text(text, parse_mode=ParseMode.MARKDOWN)
        except Exception:
            await message.reply_text(text)

    @staticmethod
    def _split_long_text(text: str) -> list[str]:
        """Split text into chunks for Telegram's 4096 char limit."""
        if len(text) <= 4000:
            return [text]
        return [text[i:i+4000] for i in range(0, len(text), 4000)]

    async def _send_long_reply(self, message, response: str, with_actions: bool = False):
        """Send a response, splitting if too long for Telegram's 4096 char limit."""
        action_keyboard = None
        if with_actions and len(response) > 100:
            action_keyboard = InlineKeyboardMarkup([
                [
                    InlineKeyboardButton("Simpler", callback_data="act_simpler"),
                    InlineKeyboardButton("More detail", callback_data="act_detail"),
                    InlineKeyboardButton("Translate BM", callback_data="act_translate_bm"),
                ]
            ])

        if len(response) <= 4096:
            try:
                await message.reply_text(response, parse_mode=ParseMode.MARKDOWN,
                                         reply_markup=action_keyboard)
            except Exception:
                await message.reply_text(response, reply_markup=action_keyboard)
        else:
            chunks = self._split_long_text(response)
            for i, chunk in enumerate(chunks):
                markup = action_keyboard if i == len(chunks) - 1 else None
                try:
                    await message.reply_text(chunk, parse_mode=ParseMode.MARKDOWN,
                                             reply_markup=markup)
                except Exception:
                    await message.reply_text(chunk, reply_markup=markup)

    # ------------------------------------------------------------------ #
    #  Registration                                                       #
    # ------------------------------------------------------------------ #

    async def _handle_registration(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> bool:
        """Handle user registration flow. Returns True if user can proceed."""
        user = update.effective_user
        user_id = str(user.id)
        state = self.processor.auth.get_user_state(user_id, "telegram")

        if state == "admin" or state == "approved":
            return True

        if state == "blocked":
            await update.message.reply_text(
                "Sorry, your access has been denied. "
                "Contact the admin if you think this is a mistake."
            )
            return False

        if state == "pending":
            await update.message.reply_text(
                "Still waiting for admin approval. Hang tight — "
                "you'll get a message as soon as you're in!\n\n"
                "Send /start anytime to check your status."
            )
            return False

        # New user — register and notify admins
        self.memory.register_user(
            user_id=user_id,
            platform="telegram",
            username=user.username,
            first_name=user.first_name,
            last_name=user.last_name,
        )
        await update.message.reply_text(
            "Welcome to AmanClaw!\n\n"
            "I'm a smart AI assistant that can remember things about you, "
            "analyze photos, set reminders, and much more.\n\n"
            "Your registration has been sent to an admin for approval. "
            "You'll be notified as soon as you're approved — usually within minutes!\n\n"
            "Send /start anytime to check your status."
        )

        # Notify all admins with inline approve/block buttons
        admin_ids = self.config.get("admin_users", {}).get("telegram", [])
        name = escape_markdown(user.first_name or user.username or user_id)
        admin_keyboard = InlineKeyboardMarkup([
            [
                InlineKeyboardButton("Approve", callback_data=f"adm_approve_{user_id}"),
                InlineKeyboardButton("Block", callback_data=f"adm_block_{user_id}"),
            ]
        ])
        for admin_id in admin_ids:
            try:
                await context.bot.send_message(
                    chat_id=int(admin_id),
                    text=(
                        f"*New user registration:*\n\n"
                        f"Name: {name}\n"
                        f"Username: @{escape_markdown(user.username or 'none')}\n"
                        f"User ID: `{user_id}`"
                    ),
                    parse_mode=ParseMode.MARKDOWN,
                    reply_markup=admin_keyboard,
                )
            except Exception as e:
                logger.error(f"Failed to notify admin {admin_id}: {e}")

        return False

    # ------------------------------------------------------------------ #
    #  Message Handlers                                                   #
    # ------------------------------------------------------------------ #

    async def handle_message(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """Handle all incoming text messages."""
        user = update.effective_user
        user_id = str(user.id)
        message_text = update.message.text

        if not await self._handle_registration(update, context):
            return

        # Check if user is in /addskill flow
        if user_id in self._addskill_state:
            await self._handle_addskill_step(update, context, user_id, message_text)
            return

        if not self.processor.rate_limiter.check(user_id):
            await update.message.reply_text("Slow down — too many messages. Try again in a minute.")
            return

        # Include quoted message context if user replied to a message
        reply = update.message.reply_to_message
        if reply and reply.text:
            quoted = reply.text[:500]
            message_text = f"[Replying to: \"{quoted}\"]\n\n{message_text}"

        clean_text, was_flagged = sanitize(message_text)
        if was_flagged:
            logger.warning(f"Flagged message from {user_id}: {message_text[:100]}")

        # Set context for skills
        set_current_user(user_id)
        set_reminder_context(user_id, str(update.effective_chat.id))
        set_scheduled_context(user_id, str(update.effective_chat.id))
        set_doc_learning_context(user_id, self.learning)

        # Start typing indicator
        stop_typing = asyncio.Event()
        typing_task = asyncio.create_task(
            self._send_typing_periodically(context, update.effective_chat.id, stop_typing)
        )

        try:
            history, facts, summary, knowledge_context = await self.processor._build_context(user_id, clean_text)
            response = await self.llm.respond(clean_text, history, flagged=was_flagged,
                                              facts=facts, summary=summary,
                                              knowledge_context=knowledge_context,
                                              user_id=user_id)
        except Exception as e:
            logger.error(f"LLM error: {e}")
            response = "Something went wrong talking to the AI. Try again in a moment."
        finally:
            stop_typing.set()
            await typing_task

        self.memory.save_exchange(user_id, "telegram", message_text, response)

        # Mark user as onboarded after first successful interaction
        if not self.memory.get_facts(user_id).get("onboarded"):
            self.memory.save_fact(user_id, "onboarded", "true")

        # Background knowledge extraction (non-blocking)
        asyncio.create_task(self.processor._extract_knowledge(user_id, message_text, response))

        # Track skill failures in response
        if self.learning and ("failed:" in response.lower() or "error:" in response.lower()):
            self.learning.log_failure(user_id, "llm_response", {"message": clean_text[:200]}, response[:500])

        await self._send_long_reply(update.message, response, with_actions=True)

        # Smart failure detection — suggest /addskill if bot lacks capability
        _capability_fail_patterns = [
            "can't access", "cannot access", "don't have access",
            "no tool", "not available", "unable to fetch",
            "can't fetch", "cannot fetch", "don't have a tool",
            "no built-in", "don't have built-in",
            "tidak dapat", "tidak boleh", "tiada akses",
        ]
        response_lower = response.lower()
        if any(p in response_lower for p in _capability_fail_patterns):
            await update.message.reply_text(
                "Want me to learn how to do this? "
                "You can add an API integration with /addskill",
            )

    async def handle_photo(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """Handle photo messages — send to vision model for analysis."""
        user = update.effective_user
        user_id = str(user.id)

        if not await self._handle_registration(update, context):
            return

        if not self.processor.rate_limiter.check(user_id):
            await update.message.reply_text("Slow down — too many messages. Try again in a minute.")
            return

        photo = update.message.photo[-1]
        caption = update.message.caption or None

        if caption:
            clean_caption, was_flagged = sanitize(caption)
        else:
            clean_caption, was_flagged = None, False

        set_current_user(user_id)
        set_reminder_context(user_id, str(update.effective_chat.id))
        set_scheduled_context(user_id, str(update.effective_chat.id))
        set_doc_learning_context(user_id, self.learning)

        stop_typing = asyncio.Event()
        typing_task = asyncio.create_task(
            self._send_typing_periodically(context, update.effective_chat.id, stop_typing)
        )

        try:
            file = await context.bot.get_file(photo.file_id)
            image_bytes = await file.download_as_bytearray()

            from amanclaw.llm import build_vision_message
            vision_msg = build_vision_message(bytes(image_bytes), clean_caption)

            history = self.memory.get_history(user_id)
            facts = self.memory.get_facts(user_id)
            summary = self.memory.get_latest_summary(user_id)

            response = await self.llm.respond(
                vision_msg, history, flagged=was_flagged,
                facts=facts, summary=summary,
                user_id=user_id,
            )
        except Exception as e:
            logger.error(f"Vision error: {e}")
            response = "Sorry, I couldn't analyze that image. Try again."
        finally:
            stop_typing.set()
            await typing_task

        user_text = f"[Photo]{f': {caption}' if caption else ''}"
        self.memory.save_exchange(user_id, "telegram", user_text, response)
        asyncio.create_task(self.processor._extract_knowledge(user_id, user_text, response))
        await self._send_long_reply(update.message, response)

    async def handle_voice(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """Handle voice messages — acknowledge and ask for text."""
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        await update.message.reply_text(
            "I can't process voice messages yet. "
            "Please type your message instead, or send a photo for image analysis."
        )

    # ------------------------------------------------------------------ #
    #  Command Handlers                                                   #
    # ------------------------------------------------------------------ #

    async def cmd_start(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not await self._handle_registration(update, context):
            return
        user = update.effective_user
        facts = self.memory.get_facts(user_id)
        is_onboarded = facts.get("onboarded") == "true"
        name = escape_markdown(facts.get("name", user.first_name or "there"))
        if is_onboarded:
            keyboard = InlineKeyboardMarkup([
                [
                    InlineKeyboardButton("Clear History", callback_data="clear"),
                    InlineKeyboardButton("Export Chat", callback_data="export"),
                ],
            ])
            await update.message.reply_text(
                f"Hey {name}! AmanClaw is ready.\n\n"
                "Just send me a message, photo, or voice note.",
                parse_mode=ParseMode.MARKDOWN,
                reply_markup=keyboard,
            )
        else:
            await self._send_approval_welcome(context, user_id)

    async def cmd_skills(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        await self._reply_with_markdown(update.message, f"*Available skills:*\n\n{get_skill_list()}")

    async def cmd_clear(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        keyboard = InlineKeyboardMarkup([
            [
                InlineKeyboardButton("Yes, clear it", callback_data="confirm_clear"),
                InlineKeyboardButton("Cancel", callback_data="cancel"),
            ]
        ])
        await update.message.reply_text(
            "Are you sure you want to clear your conversation history?",
            reply_markup=keyboard,
        )

    async def cmd_status(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        stats = self.memory.get_stats()
        facts = self.memory.get_facts(user_id)
        reminders = self.memory.get_user_reminders(user_id)
        text = (
            "*AmanClaw Status*\n\n"
            f"Messages: {stats['total_messages']}\n"
            f"Facts: {stats['total_facts']}\n"
            f"Summaries: {stats['total_summaries']}\n"
            f"Your facts: {len(facts)}\n"
            f"Pending reminders: {len(reminders)}\n"
            f"Unique users: {stats['unique_users']}"
        )
        await self._reply_with_markdown(update.message, text)

    async def cmd_export(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        export_text = self.memory.export_history(user_id)
        if export_text == "No conversation history.":
            await update.message.reply_text("No conversation history to export.")
            return
        buf = io.BytesIO(export_text.encode("utf-8"))
        buf.name = f"amanclaw_chat_{user_id}_{datetime.now().strftime('%Y%m%d_%H%M')}.txt"
        await update.message.reply_document(
            document=buf,
            caption="Here's your conversation history.",
        )

    async def cmd_myid(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        await update.message.reply_text(
            f"Your Telegram user ID: `{update.effective_user.id}`",
            parse_mode=ParseMode.MARKDOWN,
        )

    async def cmd_approve(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        admin_id = str(update.effective_user.id)
        if not self.processor.auth.is_admin(admin_id, "telegram"):
            return
        if not context.args:
            await update.message.reply_text("Usage: /approve <user_id>")
            return
        target_id = context.args[0]
        if self.memory.approve_user(target_id):
            await update.message.reply_text(f"User `{target_id}` approved.", parse_mode=ParseMode.MARKDOWN)
            await self._send_approval_welcome(context, target_id)
        else:
            user = self.memory.get_user(target_id)
            if not user:
                await update.message.reply_text(f"User `{target_id}` not found.", parse_mode=ParseMode.MARKDOWN)
            else:
                await update.message.reply_text(
                    f"User `{target_id}` is already {user['status']}.", parse_mode=ParseMode.MARKDOWN
                )

    async def cmd_block(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        admin_id = str(update.effective_user.id)
        if not self.processor.auth.is_admin(admin_id, "telegram"):
            return
        if not context.args:
            await update.message.reply_text("Usage: /block <user_id>")
            return
        target_id = context.args[0]
        if self.memory.block_user(target_id):
            await update.message.reply_text(f"User `{target_id}` blocked.", parse_mode=ParseMode.MARKDOWN)
        else:
            user = self.memory.get_user(target_id)
            if not user:
                await update.message.reply_text(f"User `{target_id}` not found.", parse_mode=ParseMode.MARKDOWN)
            else:
                await update.message.reply_text(
                    f"User `{target_id}` is already {user['status']}.", parse_mode=ParseMode.MARKDOWN
                )

    async def cmd_users(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        admin_id = str(update.effective_user.id)
        if not self.processor.auth.is_admin(admin_id, "telegram"):
            return
        status_filter = context.args[0] if context.args else None
        if status_filter and status_filter not in ("pending", "approved", "blocked"):
            await update.message.reply_text("Usage: /users [pending|approved|blocked]")
            return
        users = self.memory.list_users(status=status_filter)
        if not users:
            label = f" ({status_filter})" if status_filter else ""
            await update.message.reply_text(f"No users{label} found.")
            return
        lines = [f"*Users{(' - ' + status_filter) if status_filter else ''}:*\n"]
        for u in users:
            name = escape_markdown(u["first_name"] or u["username"] or "Unknown")
            username = f"@{escape_markdown(u['username'])}" if u["username"] else "no username"
            lines.append(f"- `{u['user_id']}` {name} ({username}) [{u['status']}]")
        await self._reply_with_markdown(update.message, "\n".join(lines))

    async def cmd_teach(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        if not context.args:
            await self._reply_with_markdown(update.message,
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
        await self._reply_with_markdown(update.message, result)

    async def cmd_learned(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        days = int(context.args[0]) if context.args else 7
        if self.learning:
            journal = self.learning.get_learning_journal(user_id, days=days)
        else:
            journal = "Learning engine not initialized."
        await self._send_long_reply(update.message, journal)

    async def cmd_forget(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        if not context.args:
            await update.message.reply_text("Usage: /forget <topic>\nExample: /forget coffee preference")
            return
        query = " ".join(context.args)
        set_current_user(user_id)
        from amanclaw.skills.remember import forget
        result = forget(query=query)
        await self._reply_with_markdown(update.message, result)

    async def cmd_myskills(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not await self._handle_registration(update, context):
            return
        skills = self.memory.get_user_skills(user_id)
        own = [s for s in skills if s["user_id"] == user_id]
        if not own:
            await update.message.reply_text(
                "You don't have any custom skills yet.\n"
                "Use /addskill to create one!"
            )
            return
        lines = ["Your Skills:\n"]
        for s in own:
            status = "private" if s["is_private"] else ("approved" if s["is_approved"] else "pending review")
            lines.append(f"- {s['name']}: {s['description']} [{status}]")
        await update.message.reply_text("\n".join(lines))

    async def cmd_delskill(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not await self._handle_registration(update, context):
            return
        if not context.args:
            await update.message.reply_text("Usage: /delskill <skill_name>")
            return
        name = context.args[0]
        if self.memory.delete_user_skill(user_id, name):
            await update.message.reply_text(f"Skill '{name}' deleted.")
        else:
            await update.message.reply_text(f"Skill '{name}' not found.")

    async def cmd_publish(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not await self._handle_registration(update, context):
            return
        if not context.args:
            await update.message.reply_text("Usage: /publish <skill_name>")
            return
        name = context.args[0]
        if self.memory.publish_user_skill(user_id, name):
            await update.message.reply_text(
                f"Skill '{name}' submitted for review!\n"
                "An admin will review it shortly."
            )
            admin_ids = self.config.get("admin_users", {}).get("telegram", [])
            skill = self.memory.get_user_skill_by_name(name, user_id)
            keyboard = InlineKeyboardMarkup([
                [
                    InlineKeyboardButton("Approve", callback_data=f"appskill_{name}_{user_id}"),
                    InlineKeyboardButton("Reject", callback_data=f"rejskill_{name}_{user_id}"),
                ]
            ])
            for admin_id in admin_ids:
                try:
                    await context.bot.send_message(
                        chat_id=int(admin_id),
                        text=(
                            f"Skill submitted for marketplace:\n\n"
                            f"Name: {name}\n"
                            f"By: {user_id}\n"
                            f"Description: {skill['description']}\n"
                            f"URL: {skill['url_template']}\n"
                            f"Method: {skill['method']}"
                        ),
                        reply_markup=keyboard,
                    )
                except Exception:
                    pass
        else:
            await update.message.reply_text(f"Skill '{name}' not found or already published.")

    async def cmd_marketplace(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not await self._handle_registration(update, context):
            return
        skills = self.memory.get_marketplace_skills()
        if not skills:
            await update.message.reply_text("No community skills available yet. Be the first — use /addskill!")
            return
        lines = ["Community Marketplace:\n"]
        for s in skills:
            lines.append(f"- {s['name']}: {s['description']}")
        lines.append("\nAll marketplace skills are automatically available to you!")
        await update.message.reply_text("\n".join(lines))

    # ------------------------------------------------------------------ #
    #  Approval Welcome                                                   #
    # ------------------------------------------------------------------ #

    async def _send_approval_welcome(self, context: ContextTypes.DEFAULT_TYPE, user_id: str):
        welcome_keyboard = InlineKeyboardMarkup([
            [
                InlineKeyboardButton("Tell me your name", callback_data="try_name"),
                InlineKeyboardButton("Analyze a photo", callback_data="try_photo"),
            ],
            [
                InlineKeyboardButton("Set a reminder", callback_data="try_reminder"),
                InlineKeyboardButton("How do I teach you?", callback_data="try_teach"),
            ],
            [
                InlineKeyboardButton("Set language", callback_data="onboard_lang"),
            ],
        ])
        try:
            await context.bot.send_message(
                chat_id=int(user_id),
                text=(
                    "You're approved! Welcome to AmanClaw.\n\n"
                    "I'm your personal AI assistant. Here's what I can do:\n\n"
                    "I remember things about you across conversations\n"
                    "Send me a photo and I'll analyze it\n"
                    "I can set reminders for you\n"
                    "Teach me custom rules and I'll follow them\n\n"
                    "Try one of these to get started, or just say hi!"
                ),
                reply_markup=welcome_keyboard,
            )
        except Exception as e:
            logger.error(f"Failed to send welcome to {user_id}: {e}")

    # ------------------------------------------------------------------ #
    #  /addskill Conversational Flow                                     #
    # ------------------------------------------------------------------ #

    async def cmd_addskill(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not await self._handle_registration(update, context):
            return
        args_text = " ".join(context.args) if context.args else ""
        if args_text.strip():
            self._addskill_state[user_id] = {"step": "generating"}
            await update.message.reply_text("Generating skill config...")
            await self._generate_skill_from_description(update, user_id, args_text.strip())
            return
        self._addskill_state[user_id] = {"step": "describe"}
        await update.message.reply_text(
            "Let's create a new skill!\n\n"
            "Just describe what you want:\n"
            "• \"Get weather for any city\"\n"
            "• \"Convert currencies\"\n"
            "• \"Get a random joke\"\n"
            "• \"Shorten URLs\"\n\n"
            "I'll find a free API and set it up for you automatically.\n\n"
            "Or do it inline: `/addskill get weather for a city`\n\n"
            "Send /cancel to stop.",
            parse_mode=ParseMode.MARKDOWN,
        )

    async def _generate_skill_from_description(self, update, user_id: str, description: str):
        try:
            result = await self.llm._call_api([
                {"role": "system", "content": ADDSKILL_LLM_PROMPT},
                {"role": "user", "content": description},
            ])
            raw = result["choices"][0]["message"]["content"]
            raw = re.sub(r'^```(?:json)?\s*', '', raw.strip())
            raw = re.sub(r'\s*```$', '', raw.strip())
            raw = re.sub(r'<think>.*?</think>', '', raw, flags=re.DOTALL).strip()
            skill_config = json.loads(raw)
        except (json.JSONDecodeError, KeyError, IndexError) as e:
            logger.warning(f"LLM skill generation failed: {e}")
            self._addskill_state.pop(user_id, None)
            msg = update.message if hasattr(update, 'message') and update.message else update
            await msg.reply_text(
                "Couldn't auto-generate the skill. Try being more specific.\n"
                "Example: `/addskill get weather forecast for a city using wttr.in`",
                parse_mode=ParseMode.MARKDOWN,
            )
            return
        except Exception as e:
            logger.error(f"LLM skill generation error: {e}")
            self._addskill_state.pop(user_id, None)
            msg = update.message if hasattr(update, 'message') and update.message else update
            await msg.reply_text(f"Error generating skill: {e}")
            return

        name = skill_config.get("name", "").lower().replace(" ", "_").replace("-", "_")
        name = re.sub(r'[^a-z0-9_]', '', name)
        if not name or len(name) < 2:
            name = re.sub(r'[^a-z0-9_]', '', description.lower().split()[0])[:20] or "custom"
        name = name[:30]
        if name in REGISTRY:
            name = f"my_{name}"

        state = {
            "step": "confirm",
            "name": name,
            "description": skill_config.get("description", description),
            "url_template": skill_config.get("url_template", ""),
            "method": skill_config.get("method", "GET"),
            "parameters": skill_config.get("parameters", {}),
            "headers": skill_config.get("headers", {}),
            "query_params": skill_config.get("query_params", {}),
            "needs_api_key": skill_config.get("needs_api_key", False),
            "api_key": None,
        }
        self._addskill_state[user_id] = state
        await self._show_addskill_confirmation(update, state)

    async def _handle_addskill_step(self, update: Update, context: ContextTypes.DEFAULT_TYPE,
                                     user_id: str, text: str):
        if text.strip().lower() == "/cancel":
            del self._addskill_state[user_id]
            await update.message.reply_text("Skill creation cancelled.")
            return
        state = self._addskill_state[user_id]
        step = state["step"]
        if step == "describe":
            state["step"] = "generating"
            await update.message.reply_text("Generating skill config...")
            await self._generate_skill_from_description(update, user_id, text.strip())
        elif step == "apikey_input":
            state["api_key"] = text.strip()
            state["step"] = "confirm"
            await self._show_addskill_confirmation(update, state)
        elif step == "edit":
            await update.message.reply_text("Regenerating with your feedback...")
            original_desc = state.get("description", "")
            await self._generate_skill_from_description(
                update, user_id, f"{original_desc}. {text.strip()}"
            )

    async def _show_addskill_confirmation(self, update_or_query, state: dict):
        params_list = ", ".join(state.get("parameters", {}).keys()) or "none"
        needs_key = state.get("needs_api_key", False)
        has_key = bool(state.get("api_key"))
        if needs_key and not has_key:
            api_key_status = "required (not set yet)"
        elif has_key:
            api_key_status = "set"
        else:
            api_key_status = "not needed"
        summary = (
            f"*Skill Preview:*\n\n"
            f"*Name:* `{state['name']}`\n"
            f"*Description:* {state['description']}\n"
            f"*URL:* `{state['url_template']}`\n"
            f"*Method:* {state.get('method', 'GET')}\n"
            f"*Parameters:* {params_list}\n"
            f"*API Key:* {api_key_status}"
        )
        buttons = []
        if needs_key and not has_key:
            buttons.append([
                InlineKeyboardButton("Set API Key", callback_data="addskill_haskey"),
                InlineKeyboardButton("Skip (no key)", callback_data="addskill_nokey"),
            ])
        buttons.append([
            InlineKeyboardButton("Create", callback_data="addskill_confirm"),
            InlineKeyboardButton("Edit", callback_data="addskill_edit"),
            InlineKeyboardButton("Cancel", callback_data="addskill_cancel"),
        ])
        keyboard = InlineKeyboardMarkup(buttons)
        msg = update_or_query.message if hasattr(update_or_query, 'message') and update_or_query.message else update_or_query
        await msg.reply_text(summary, reply_markup=keyboard, parse_mode=ParseMode.MARKDOWN)

    # ------------------------------------------------------------------ #
    #  Callback Handler                                                   #
    # ------------------------------------------------------------------ #

    async def handle_callback(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """Handle inline keyboard button presses."""
        query = update.callback_query
        await query.answer()
        user_id = str(query.from_user.id)

        # --- Admin approval callbacks ---
        if query.data.startswith("adm_approve_"):
            if not self.processor.auth.is_admin(user_id, "telegram"):
                await query.answer("Not authorized.", show_alert=True)
                return
            target_id = query.data.replace("adm_approve_", "")
            if self.memory.approve_user(target_id):
                await query.edit_message_text(
                    query.message.text + "\n\n*Approved*",
                    parse_mode=ParseMode.MARKDOWN,
                )
                await self._send_approval_welcome(context, target_id)
            else:
                await query.answer("User already processed.", show_alert=True)
            return

        if query.data.startswith("adm_block_"):
            if not self.processor.auth.is_admin(user_id, "telegram"):
                await query.answer("Not authorized.", show_alert=True)
                return
            target_id = query.data.replace("adm_block_", "")
            if self.memory.block_user(target_id):
                await query.edit_message_text(
                    query.message.text + "\n\n*Blocked*",
                    parse_mode=ParseMode.MARKDOWN,
                )
            else:
                await query.answer("User already processed.", show_alert=True)
            return

        # --- Try-me onboarding callbacks ---
        if query.data == "try_name":
            await query.edit_message_text(
                "Just send me a message telling me your name!\n\n"
                "For example: \"My name is Sarah\" or \"Call me Alex\"\n\n"
                "I'll remember it for all our future conversations."
            )
            return
        if query.data == "try_photo":
            await query.edit_message_text(
                "Send me any photo and I'll analyze it!\n\n"
                "I can describe what's in it, read text from images, "
                "identify objects, and answer questions about it.\n\n"
                "Try it now — just send a photo from your gallery."
            )
            return
        if query.data == "try_reminder":
            await query.edit_message_text(
                "Just ask me to remind you of something!\n\n"
                "For example:\n"
                "\"Remind me to call the dentist in 2 hours\"\n"
                "\"Remind me about the meeting at 3pm tomorrow\"\n\n"
                "I'll send you a message when it's time."
            )
            return
        if query.data == "try_teach":
            await query.edit_message_text(
                "You can teach me custom rules!\n\n"
                "Use /teach followed by your rule. For example:\n"
                "/teach Always reply in bullet points\n"
                "/teach When I say 'brief', keep answers under 2 sentences\n\n"
                "Use /learned to see what I've learned from you."
            )
            return

        # --- Addskill flow callbacks ---
        if query.data == "addskill_edit":
            if user_id in self._addskill_state:
                self._addskill_state[user_id]["step"] = "edit"
                await query.edit_message_text(
                    "What would you like to change? Just tell me:\n\n"
                    "Examples:\n"
                    "• \"Use a different API\"\n"
                    "• \"Change the name to my_weather\"\n"
                    "• \"Add a language parameter\"\n"
                    "• \"Use POST instead of GET\"\n\n"
                    "Or describe the whole skill differently."
                )
            return
        if query.data == "addskill_nokey":
            if user_id in self._addskill_state:
                self._addskill_state[user_id]["api_key"] = None
                self._addskill_state[user_id]["step"] = "confirm"
                await query.edit_message_text("No API key needed.")
                await self._show_addskill_confirmation(query, self._addskill_state[user_id])
            return
        if query.data == "addskill_haskey":
            if user_id in self._addskill_state:
                self._addskill_state[user_id]["step"] = "apikey_input"
                await query.edit_message_text(
                    "Send me the API key. I'll store it securely and never show it again."
                )
            return
        if query.data == "addskill_confirm":
            if user_id in self._addskill_state:
                state = self._addskill_state.pop(user_id)
                skill_data = {
                    "name": state["name"],
                    "description": state["description"],
                    "url_template": state["url_template"],
                    "method": state.get("method", "GET"),
                    "parameters": state.get("parameters", {}),
                    "api_key_encrypted": state.get("api_key"),
                    "is_private": True,
                }
                self.memory.save_user_skill(user_id, skill_data)
                await query.edit_message_text(
                    f"Skill '{state['name']}' created!\n\n"
                    f"Try it now — just ask me something that uses it.\n\n"
                    f"Commands:\n"
                    f"/myskills — view your skills\n"
                    f"/publish {state['name']} — submit to community marketplace\n"
                    f"/delskill {state['name']} — delete this skill"
                )
            return
        if query.data == "addskill_cancel":
            if user_id in self._addskill_state:
                del self._addskill_state[user_id]
            await query.edit_message_text("Skill creation cancelled.")
            return

        # --- Skill marketplace admin callbacks ---
        if query.data.startswith("appskill_"):
            if not self.processor.auth.is_admin(user_id, "telegram"):
                await query.answer("Not authorized.", show_alert=True)
                return
            parts = query.data.replace("appskill_", "").rsplit("_", 1)
            skill_name, creator_id = parts[0], parts[1]
            skill = self.memory.get_user_skill_by_name(skill_name, creator_id)
            if skill and self.memory.approve_user_skill(skill["id"]):
                await query.edit_message_text(
                    query.message.text + "\n\nApproved for marketplace"
                )
            return
        if query.data.startswith("rejskill_"):
            if not self.processor.auth.is_admin(user_id, "telegram"):
                await query.answer("Not authorized.", show_alert=True)
                return
            parts = query.data.replace("rejskill_", "").rsplit("_", 1)
            skill_name, creator_id = parts[0], parts[1]
            self.memory.delete_user_skill(creator_id, skill_name)
            await query.edit_message_text(
                query.message.text + "\n\nRejected and removed"
            )
            return

        if not self.auth_check(user_id):
            return

        if query.data == "skills":
            await query.edit_message_text(
                f"*Available skills:*\n\n{get_skill_list()}",
                parse_mode=ParseMode.MARKDOWN,
            )
        elif query.data == "status":
            stats = self.memory.get_stats()
            facts = self.memory.get_facts(user_id)
            reminders = self.memory.get_user_reminders(user_id)
            await query.edit_message_text(
                f"*AmanClaw Status*\n\n"
                f"Messages: {stats['total_messages']}\n"
                f"Facts: {stats['total_facts']}\n"
                f"Your facts: {len(facts)}\n"
                f"Pending reminders: {len(reminders)}",
                parse_mode=ParseMode.MARKDOWN,
            )
        elif query.data == "confirm_clear":
            self.memory.clear_history(user_id)
            await query.edit_message_text("Conversation history cleared.")
        elif query.data == "cancel":
            await query.edit_message_text("Cancelled.")
        elif query.data == "export":
            export_text = self.memory.export_history(user_id)
            if export_text == "No conversation history.":
                await query.edit_message_text("No conversation history to export.")
            else:
                await query.edit_message_text("Exporting your chat history...")
                buf = io.BytesIO(export_text.encode("utf-8"))
                buf.name = f"amanclaw_chat_{user_id}_{datetime.now().strftime('%Y%m%d_%H%M')}.txt"
                await context.bot.send_document(
                    chat_id=query.message.chat_id,
                    document=buf,
                    caption="Here's your conversation history.",
                )
        elif query.data == "onboard_name":
            await query.edit_message_text(
                "Just send me a message like:\n\n"
                "\"My name is [your name]\"\n\n"
                "I'll remember it for future conversations!"
            )
        elif query.data == "onboard_lang":
            lang_keyboard = InlineKeyboardMarkup([
                [
                    InlineKeyboardButton("English", callback_data="setlang_en"),
                    InlineKeyboardButton("Bahasa Melayu", callback_data="setlang_ms"),
                ],
                [
                    InlineKeyboardButton("Auto-detect", callback_data="setlang_auto"),
                ],
            ])
            await query.edit_message_text(
                "Choose your preferred language:",
                reply_markup=lang_keyboard,
            )
        elif query.data.startswith("setlang_"):
            lang_code = query.data.replace("setlang_", "")
            lang_names = {"en": "English", "ms": "Bahasa Melayu", "auto": "Auto-detect"}
            lang_name = lang_names.get(lang_code, lang_code)
            self.memory.save_fact(user_id, "preferred_language", lang_name)
            await query.edit_message_text(f"Language set to *{lang_name}*. Let's start chatting!", parse_mode=ParseMode.MARKDOWN)
        elif query.data.startswith("act_"):
            original_text = query.message.text
            if not original_text:
                await query.answer("No text to work with.")
                return
            action = query.data.replace("act_", "")
            prompts = {
                "simpler": f"Explain this more simply and briefly:\n\n{original_text}",
                "detail": f"Expand on this with more detail and examples:\n\n{original_text}",
                "translate_bm": f"Translate this to Bahasa Melayu:\n\n{original_text}",
            }
            prompt = prompts.get(action)
            if not prompt:
                return
            await query.answer("Working on it...")
            await context.bot.send_chat_action(chat_id=query.message.chat_id, action=ChatAction.TYPING)
            try:
                response = await self.llm.respond(prompt, [], facts=self.memory.get_facts(user_id), user_id=user_id)
            except Exception:
                response = "Sorry, something went wrong. Try again."
            self.memory.save_exchange(user_id, "telegram", f"[{action}]", response)
            await context.bot.send_message(
                chat_id=query.message.chat_id,
                text=response,
                parse_mode=ParseMode.MARKDOWN,
            )

    # ------------------------------------------------------------------ #
    #  ChannelAdapter interface + handler registration                    #
    # ------------------------------------------------------------------ #

    def register_handlers(self, application):
        """Register all Telegram handlers on the Application."""
        application.add_handler(CommandHandler("start", self.cmd_start))
        application.add_handler(CommandHandler("skills", self.cmd_skills))
        application.add_handler(CommandHandler("clear", self.cmd_clear))
        application.add_handler(CommandHandler("status", self.cmd_status))
        application.add_handler(CommandHandler("export", self.cmd_export))
        application.add_handler(CommandHandler("myid", self.cmd_myid))
        application.add_handler(CommandHandler("approve", self.cmd_approve))
        application.add_handler(CommandHandler("block", self.cmd_block))
        application.add_handler(CommandHandler("users", self.cmd_users))
        application.add_handler(CommandHandler("teach", self.cmd_teach))
        application.add_handler(CommandHandler("learned", self.cmd_learned))
        application.add_handler(CommandHandler("forget", self.cmd_forget))
        application.add_handler(CommandHandler("addskill", self.cmd_addskill))
        application.add_handler(CommandHandler("myskills", self.cmd_myskills))
        application.add_handler(CommandHandler("delskill", self.cmd_delskill))
        application.add_handler(CommandHandler("publish", self.cmd_publish))
        application.add_handler(CommandHandler("marketplace", self.cmd_marketplace))
        application.add_handler(CallbackQueryHandler(self.handle_callback))
        application.add_handler(MessageHandler(filters.PHOTO, self.handle_photo))
        application.add_handler(MessageHandler(filters.VOICE | filters.AUDIO, self.handle_voice))
        application.add_handler(MessageHandler(filters.TEXT & ~filters.COMMAND, self.handle_message))

    async def start(self) -> None:
        """Telegram is started by bot.py via run_polling/run_webhook — this is a no-op."""
        pass

    async def stop(self) -> None:
        """Telegram shutdown is handled by bot.py — this is a no-op."""
        pass

    async def send_message(self, msg: OutgoingMessage) -> None:
        """Send a message. Requires the bot instance to be set externally."""
        # This is used by the ABC contract. For Telegram, most sending
        # is done through update.message.reply_text in handlers.
        # This method is available for programmatic sends (reminders, etc.)
        pass
```

**Step 4: Run test to verify it passes**

Run: `pytest tests/test_telegram_adapter.py -v`
Expected: ALL PASS

**Step 5: Commit**

```bash
git add amanclaw/channels/telegram.py tests/test_telegram_adapter.py
git commit -m "feat: create TelegramAdapter in channels/telegram.py"
```

---

### Task 3.2: Slim bot.py to orchestrator

**Files:**
- Modify: `amanclaw/bot.py` (remove all extracted code, keep orchestration)

**Step 1: Rewrite `bot.py` as thin orchestrator**

Replace `amanclaw/bot.py` with:

```python
"""
AmanClaw — Main bot entry point (orchestrator).

Initializes all components, creates adapters, manages lifecycle.
Telegram-specific handlers live in channels/telegram.py.
"""

import os
import sys
import yaml
import asyncio
import logging
from pathlib import Path
from dotenv import load_dotenv

load_dotenv()
from datetime import datetime, time as datetime_time
from telegram import BotCommand
from telegram.ext import ApplicationBuilder, ContextTypes
from telegram.constants import ParseMode

from amanclaw.security import Auth, RateLimiter
from amanclaw.memory import Memory
from amanclaw.llm import LLM
from amanclaw.skills.shell import configure as configure_shell
from amanclaw.skills.files import configure as configure_files
from amanclaw.skills.remember import configure as configure_remember
from amanclaw.skills.reminder import configure as configure_reminder
from amanclaw.skills.scheduled import configure as configure_scheduled
from amanclaw.skills.documents import configure as configure_documents
from amanclaw.learning import LearningEngine
from amanclaw.mcp_client import MCPManager
from amanclaw.skills import set_mcp_manager, set_user_skill_manager
from amanclaw.skills.user_skills import UserSkillManager
from amanclaw.processor import MessageProcessor
from amanclaw.channels.telegram import TelegramAdapter


class JsonFormatter(logging.Formatter):
    """JSON log formatter for structured logging (Docker, log aggregators)."""
    def format(self, record):
        import json
        log_data = {
            "ts": datetime.now().isoformat(),
            "level": record.levelname,
            "logger": record.name,
            "msg": record.getMessage(),
        }
        if record.exc_info and record.exc_info[0]:
            log_data["exception"] = self.formatException(record.exc_info)
        return json.dumps(log_data)


def setup_logging():
    """Configure logging with console + optional rotating file output."""
    log_level = os.environ.get("LOG_LEVEL", "INFO").upper()
    log_file = os.environ.get("LOG_FILE")
    log_format = os.environ.get("LOG_FORMAT", "text")

    root = logging.getLogger()
    root.setLevel(getattr(logging, log_level, logging.INFO))

    console = logging.StreamHandler()
    if log_format == "json":
        console.setFormatter(JsonFormatter())
    else:
        console.setFormatter(logging.Formatter(
            "%(asctime)s [%(name)s] %(levelname)s: %(message)s",
            datefmt="%H:%M:%S",
        ))
    root.addHandler(console)

    if log_file:
        from logging.handlers import RotatingFileHandler
        file_handler = RotatingFileHandler(
            log_file, maxBytes=10_000_000, backupCount=5, encoding="utf-8"
        )
        file_handler.setFormatter(logging.Formatter(
            "%(asctime)s [%(name)s] %(levelname)s: %(message)s",
            datefmt="%Y-%m-%d %H:%M:%S",
        ))
        root.addHandler(file_handler)

    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.getLogger("httpcore").setLevel(logging.WARNING)
    logging.getLogger("telegram").setLevel(logging.WARNING)
    logging.getLogger("aiohttp").setLevel(logging.WARNING)


setup_logging()
logger = logging.getLogger("amanclaw.bot")


def load_config(path: str = "config.yaml") -> dict:
    config_path = Path(path)
    if not config_path.exists():
        logger.error(f"Config not found: {path}")
        logger.error("Copy config.example.yaml to config.yaml and fill in your values.")
        sys.exit(1)
    with open(config_path) as f:
        return yaml.safe_load(f)


# --- Globals ---
config: dict = {}
memory: Memory = None
llm: LLM = None
whatsapp = None
learning_engine: LearningEngine = None
mcp_manager = None
processor: MessageProcessor = None
telegram_adapter: TelegramAdapter = None
discord_adapter = None
slack_adapter = None


# --- Jobs ---

async def check_reminders(context: ContextTypes.DEFAULT_TYPE):
    """Periodic job to check and deliver due reminders."""
    due = memory.get_due_reminders()
    for r in due:
        try:
            if r["platform"] == "whatsapp" and whatsapp:
                await whatsapp.deliver_reminder(r["chat_id"], r["message"])
            else:
                await context.bot.send_message(
                    chat_id=int(r["chat_id"]),
                    text=f"*Reminder:* {r['message']}",
                    parse_mode=ParseMode.MARKDOWN,
                )
            memory.mark_reminder_delivered(r["id"])
            logger.info(f"Delivered reminder #{r['id']} to {r['platform']} user {r['user_id']}")
        except Exception as e:
            logger.error(f"Failed to deliver reminder #{r['id']}: {e}")


async def check_schedules(context: ContextTypes.DEFAULT_TYPE):
    """Periodic job to check and deliver due scheduled tasks."""
    due = memory.get_due_schedules()
    for s in due:
        try:
            if s["platform"] == "whatsapp" and whatsapp:
                await whatsapp.deliver_schedule(s["chat_id"], s["message"])
            else:
                await context.bot.send_message(
                    chat_id=int(s["chat_id"]),
                    text=f"*Scheduled:* {s['message']}",
                    parse_mode=ParseMode.MARKDOWN,
                )
            memory.mark_schedule_run(s["id"])
        except Exception as e:
            logger.error(f"Failed to deliver schedule #{s['id']}: {e}")


async def prune_job(context: ContextTypes.DEFAULT_TYPE):
    """Daily cleanup of old messages, delivered reminders, and expired knowledge."""
    msgs = memory.prune_all_users(keep_last=200)
    reminders = memory.prune_delivered_reminders(older_than_days=30)
    expired = memory.expire_old_knowledge()
    if msgs or reminders or expired:
        logger.info(f"Pruned {msgs} old messages, {reminders} delivered reminders, {expired} expired knowledge")


async def checkin_job(context: ContextTypes.DEFAULT_TYPE):
    """Weekly job to send proactive check-in messages."""
    if not learning_engine:
        return
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


async def error_handler(update, context: ContextTypes.DEFAULT_TYPE):
    """Log errors and notify admins."""
    logger.error(f"Update {update} caused error: {context.error}", exc_info=context.error)
    admin_ids = config.get("admin_users", {}).get("telegram", [])
    error_text = f"Bot error:\n{type(context.error).__name__}: {context.error}"
    if update and hasattr(update, 'effective_user') and update.effective_user:
        error_text = f"User: {update.effective_user.id}\n{error_text}"
    for admin_id in admin_ids:
        try:
            await context.bot.send_message(
                chat_id=int(admin_id),
                text=f"*AmanClaw Error*\n\n`{error_text[:1000]}`",
                parse_mode=ParseMode.MARKDOWN,
            )
        except Exception:
            pass


# --- Lifecycle ---

async def post_init(application):
    """Set bot commands menu and start adapters after initialization."""
    if whatsapp:
        try:
            await whatsapp.start()
        except Exception as e:
            logger.error(f"Failed to start WhatsApp adapter: {e}")

    commands = [
        BotCommand("start", "Welcome & quick actions"),
        BotCommand("skills", "List available skills"),
        BotCommand("status", "Memory & bot stats"),
        BotCommand("clear", "Clear conversation history"),
        BotCommand("export", "Export chat history"),
        BotCommand("myid", "Show your Telegram user ID"),
        BotCommand("teach", "Teach me a rule or behavior"),
        BotCommand("learned", "Show what I've learned"),
        BotCommand("forget", "Forget specific knowledge"),
        BotCommand("approve", "Admin: approve a user"),
        BotCommand("block", "Admin: block a user"),
        BotCommand("users", "Admin: list users"),
    ]
    await application.bot.set_my_commands(commands)


async def post_shutdown(application):
    """Clean up resources on shutdown."""
    if whatsapp:
        await whatsapp.stop()
    if discord_adapter:
        await discord_adapter.stop()
    if slack_adapter:
        await slack_adapter.stop()
    if memory:
        memory.close()
    if llm:
        await llm.close()
    logger.info("AmanClaw shut down cleanly.")


# --- Main ---

def main():
    global config, memory, llm, whatsapp, learning_engine, mcp_manager
    global processor, telegram_adapter, discord_adapter, slack_adapter

    logger.info("Starting AmanClaw...")

    config = load_config()
    webhook_config = config.get("webhook")

    # Initialize components
    db_path = os.environ.get("MEMORY_DB_PATH") or config.get("memory_db", "memory.db")
    memory = Memory(db_path)
    auth = Auth(config, memory=memory)
    rate_limiter = RateLimiter(config.get("rate_limit_per_minute", 20))
    llm = LLM(config.get("llm", {}))

    # User skill manager
    user_skill_mgr = UserSkillManager(memory)
    set_user_skill_manager(user_skill_mgr)
    logger.info("User skill manager initialized")

    # Validate admin_users
    admin_users = config.get("admin_users", {})
    has_admins = any(ids for ids in admin_users.values() if ids)
    if not has_admins:
        logger.warning(
            "No admin users configured! No one can approve new users. "
            "Add your user ID to config.yaml under admin_users."
        )

    # Configure skills
    skills_config = config.get("skills", {})
    if skills_config.get("shell_allowed_commands"):
        configure_shell(allowed_commands=skills_config["shell_allowed_commands"])
    if skills_config.get("shell_working_dir"):
        configure_shell(working_dir=skills_config["shell_working_dir"])
    if skills_config.get("workspace_dir"):
        configure_files(workspace_dir=skills_config["workspace_dir"])
        configure_documents(workspace_dir=skills_config["workspace_dir"])
    configure_remember(memory=memory)
    configure_reminder(memory=memory)
    configure_scheduled(memory=memory)

    # Learning engine
    learning_config = config.get("learning", {})
    if learning_config.get("enabled", True):
        learning_engine = LearningEngine(memory)
        from amanclaw.skills.remember import set_learning_engine
        set_learning_engine(learning_engine)
        logger.info("Learning engine initialized")

    # MCP Client
    mcp_manager = MCPManager(config)
    if config.get("mcp_servers"):
        asyncio.get_event_loop().run_until_complete(mcp_manager.start())
    set_mcp_manager(mcp_manager)
    logger.info("MCP client initialized")

    # Message Processor
    processor = MessageProcessor(config, auth, rate_limiter, memory, llm, learning_engine)

    # --- Channel Adapters ---

    # WhatsApp (optional)
    wa_config = config.get("whatsapp", {})
    if wa_config.get("enabled"):
        from amanclaw.channels.whatsapp import WhatsAppAdapter
        whatsapp = WhatsAppAdapter(config, processor)
        logger.info("WhatsApp adapter configured (will start with bot)")

    # Discord (optional)
    if config.get("discord", {}).get("enabled", False):
        from amanclaw.channels.discord import DiscordAdapter
        discord_adapter = DiscordAdapter(config, processor)
        asyncio.get_event_loop().run_until_complete(discord_adapter.start())
        logger.info("Discord adapter started")

    # Slack (optional)
    if config.get("slack", {}).get("enabled", False):
        from amanclaw.channels.slack import SlackAdapter
        slack_adapter = SlackAdapter(config, processor)
        asyncio.get_event_loop().run_until_complete(slack_adapter.start())
        logger.info("Slack adapter started")

    # Telegram
    token = config.get("telegram", {}).get("bot_token") or os.environ.get("TELEGRAM_BOT_TOKEN")
    if not token:
        logger.error("Telegram bot token not found in config.yaml or TELEGRAM_BOT_TOKEN env var.")
        sys.exit(1)

    telegram_adapter = TelegramAdapter(config, processor, memory, llm, learning_engine)

    app = ApplicationBuilder().token(token).post_init(post_init).post_shutdown(post_shutdown).build()

    # Register Telegram handlers
    telegram_adapter.register_handlers(app)

    # Error handler
    app.add_error_handler(error_handler)

    # Schedule jobs
    app.job_queue.run_repeating(check_reminders, interval=30, first=5)
    app.job_queue.run_repeating(check_schedules, interval=60, first=15)
    app.job_queue.run_daily(prune_job, time=datetime_time(hour=3, minute=0))

    if learning_config.get("proactive_checkins", True):
        checkin_day = learning_config.get("checkin_day", 6)
        checkin_hour = learning_config.get("checkin_hour", 10)
        app.job_queue.run_daily(checkin_job, time=datetime_time(hour=checkin_hour, minute=0),
                                days=(checkin_day,))

    if webhook_config and webhook_config.get("enabled"):
        webhook_url = webhook_config["url"]
        listen = webhook_config.get("listen", "0.0.0.0")
        port = webhook_config.get("port", 8443)
        secret_token = os.environ.get("WEBHOOK_SECRET") or webhook_config.get("secret_token")
        logger.info(f"Starting webhook mode on {listen}:{port}")
        app.run_webhook(
            listen=listen,
            port=port,
            url_path=f"webhook/{token[:10]}",
            webhook_url=f"{webhook_url}/webhook/{token[:10]}",
            secret_token=secret_token,
            allowed_updates=["message", "callback_query"],
        )
    else:
        logger.info("Starting polling mode")
        app.run_polling(allowed_updates=["message", "callback_query"])

    # Cleanup after run_polling returns
    if mcp_manager:
        asyncio.get_event_loop().run_until_complete(mcp_manager.stop())
    if memory:
        memory.close()


if __name__ == "__main__":
    main()
```

**Step 2: Run all tests**

Run: `pytest tests/ -v`
Expected: ALL PASS

**Step 3: Commit**

```bash
git add amanclaw/bot.py
git commit -m "refactor: slim bot.py to orchestrator — all Telegram handlers moved to channels/telegram.py"
```

---

## Phase 4: Final Verification

### Task 4.1: Run full test suite and verify imports

**Step 1: Run all tests**

Run: `pytest tests/ -v`
Expected: ALL PASS

**Step 2: Verify backward-compat imports**

```bash
python -c "from amanclaw.whatsapp import WhatsAppAdapter; print('OK:', WhatsAppAdapter)"
python -c "from amanclaw.channels.telegram import TelegramAdapter; print('OK:', TelegramAdapter)"
python -c "from amanclaw.channels.whatsapp import WhatsAppAdapter; print('OK:', WhatsAppAdapter)"
python -c "from amanclaw.processor import MessageProcessor; print('OK:', MessageProcessor)"
```

**Step 3: Verify bot.py line count reduction**

```bash
wc -l amanclaw/bot.py amanclaw/channels/telegram.py amanclaw/channels/whatsapp.py amanclaw/processor.py
```

Expected: `bot.py` ~200 lines, `telegram.py` ~900 lines, `whatsapp.py` ~150 lines, `processor.py` ~250 lines

**Step 4: Final commit**

```bash
git add -A
git commit -m "refactor: complete channel adapter extraction — Telegram and WhatsApp now in channels/"
```
