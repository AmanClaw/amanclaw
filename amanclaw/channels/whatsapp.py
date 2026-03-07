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

    async def _send_text(self, jid: str, text: str, quote_id: str | None = None):
        """Send a text message via the Baileys bridge."""
        session = self._get_session()
        chunks = self._split_message(text)

        for i, chunk in enumerate(chunks):
            try:
                payload = {"jid": jid, "text": chunk}
                # Only quote the original message on the first chunk
                if i == 0 and quote_id:
                    payload["quote_id"] = quote_id
                async with session.post(
                    f"{self.bridge_url}/send",
                    json=payload,
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

        # In groups, only respond when the bot is mentioned
        mentioned_jids = data.get("mentioned_jids", [])
        bot_jid = data.get("bot_jid", "")
        message_id = data.get("message_id")
        if is_group and not self.ignore_groups:
            bot_number = bot_jid.split(":")[0].split("@")[0] if bot_jid else ""
            is_mentioned = any(
                bot_number and bot_number in jid for jid in mentioned_jids
            )
            if not is_mentioned:
                return web.json_response({"ok": True, "skipped": "not_mentioned"})

        user_id = phone or jid.split("@")[0]

        logger.info(f"WhatsApp message from {user_id} ({name}): {text[:80]}")

        # Process in background so we don't block the bridge
        quote_id = message_id if is_group else None
        asyncio.create_task(self._process_message(user_id, jid, name, text, is_group, quote_id))

        return web.json_response({"ok": True})

    async def _process_message(self, user_id: str, jid: str, name: str, text: str, is_group: bool = False, quote_id: str | None = None):
        """Process a WhatsApp message through the MessageProcessor pipeline."""
        try:
            # Strip the @mention from the text so the LLM gets a clean message
            clean_text = text
            if is_group and quote_id:
                import re
                clean_text = re.sub(r'@\d+', '', text).strip()
                if not clean_text:
                    clean_text = text

            incoming = IncomingMessage(
                user_id=user_id,
                chat_id=jid,
                platform="whatsapp",
                text=clean_text,
                first_name=name or None,
                is_group=is_group,
            )

            result = await self.processor.process(incoming)
            if result:
                await self._send_text(jid, result.text, quote_id=quote_id)

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
