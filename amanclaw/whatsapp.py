"""
WhatsApp adapter — connects to the Baileys bridge via REST API.

The Baileys bridge (Node.js) handles WhatsApp Web protocol.
This module receives incoming messages via HTTP callback and
sends replies via the bridge's REST API.
"""

import os
import asyncio
import logging
import aiohttp
from aiohttp import web

from amanclaw.security import Auth, RateLimiter, sanitize
from amanclaw.memory import Memory
from amanclaw.llm import LLM
from amanclaw.skills.remember import set_current_user
from amanclaw.skills.reminder import set_context as set_reminder_context
from amanclaw.skills.scheduled import set_context as set_scheduled_context

logger = logging.getLogger("amanclaw.whatsapp")

# WhatsApp message size limit (slightly under 65536 to be safe)
MAX_MESSAGE_LENGTH = 4096


class WhatsAppAdapter:
    """Receives messages from the Baileys bridge and processes them through the LLM."""

    def __init__(
        self,
        config: dict,
        auth: Auth,
        rate_limiter: RateLimiter,
        memory: Memory,
        llm: LLM,
    ):
        self.config = config
        self.auth = auth
        self.rate_limiter = rate_limiter
        self.memory = memory
        self.llm = llm

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

    def _get_session(self) -> aiohttp.ClientSession:
        if self._session is None or self._session.closed:
            self._session = aiohttp.ClientSession()
        return self._session

    # ------------------------------------------------------------------ #
    #  Sending messages via the bridge                                    #
    # ------------------------------------------------------------------ #

    async def send_message(self, jid: str, text: str):
        """Send a text message via the Baileys bridge."""
        session = self._get_session()

        # Split long messages
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
            # Try to split at a newline
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

        # Skip group messages if configured
        if is_group and self.ignore_groups:
            return web.json_response({"ok": True, "skipped": "group"})

        # Use phone number as user_id for WhatsApp
        user_id = phone or jid.split("@")[0]

        logger.info(f"WhatsApp message from {user_id} ({name}): {text[:80]}")

        # Process in background so we don't block the bridge
        asyncio.create_task(self._process_message(user_id, jid, name, text))

        return web.json_response({"ok": True})

    async def _process_message(self, user_id: str, jid: str, name: str, text: str):
        """Process a WhatsApp message through auth, LLM, and reply."""
        try:
            # --- Auth check ---
            state = self.auth.get_user_state(user_id, "whatsapp")

            if state == "blocked":
                return

            if state == "new":
                # Auto-register
                self.memory.register_user(
                    user_id=user_id,
                    platform="whatsapp",
                    username=None,
                    first_name=name or None,
                )

                # Notify admins on Telegram
                await self._notify_admins_new_user(user_id, name)

                await self.send_message(
                    jid,
                    "Welcome! You've been registered.\n\n"
                    "An admin needs to approve your access before you can start chatting. "
                    "Please wait for approval.",
                )
                return

            if state == "pending":
                await self.send_message(
                    jid,
                    "Your registration is pending approval. "
                    "An admin will review your request shortly.",
                )
                return

            # state is "admin" or "approved" — proceed

            # --- Rate limit ---
            if not self.rate_limiter.check(user_id):
                await self.send_message(
                    jid, "Slow down — too many messages. Try again in a minute."
                )
                return

            # --- Sanitize ---
            clean_text, was_flagged = sanitize(text)
            if was_flagged:
                logger.warning(f"Flagged WhatsApp message from {user_id}: {text[:100]}")

            # --- Set skill context ---
            set_current_user(user_id)
            set_reminder_context(user_id, jid)
            set_scheduled_context(user_id, jid)

            # --- Build context and get LLM response ---
            history = self.memory.get_history(user_id)
            facts = self.memory.get_facts(user_id)
            summary = self.memory.get_latest_summary(user_id)

            response = await self.llm.respond(
                clean_text, history, flagged=was_flagged,
                facts=facts, summary=summary,
            )

            # --- Save and reply ---
            self.memory.save_exchange(user_id, "whatsapp", text, response)
            await self.send_message(jid, response)

        except Exception as e:
            logger.error(f"Error processing WhatsApp message from {user_id}: {e}", exc_info=True)
            try:
                await self.send_message(
                    jid, "Something went wrong. Try again in a moment."
                )
            except Exception:
                pass

    async def _notify_admins_new_user(self, user_id: str, name: str):
        """Notify Telegram admins about new WhatsApp user registration."""
        # This is a best-effort notification — don't fail if it doesn't work
        admin_ids = self.config.get("admin_users", {}).get("telegram", [])
        if not admin_ids:
            return

        text = (
            f"*New WhatsApp user registration:*\n\n"
            f"Name: {name or 'Unknown'}\n"
            f"Phone: `{user_id}`\n\n"
            f"Use `/approve {user_id}` to approve or `/block {user_id}` to block."
        )

        # We don't have direct access to the Telegram bot here,
        # so log the notification for the admin to see in logs
        logger.info(
            f"New WhatsApp user: {user_id} ({name}). "
            f"Approve with: /approve {user_id}"
        )

    # ------------------------------------------------------------------ #
    #  Delivery of reminders / schedules for WhatsApp users               #
    # ------------------------------------------------------------------ #

    async def deliver_reminder(self, chat_id: str, message: str):
        """Deliver a reminder to a WhatsApp user."""
        jid = chat_id if "@" in chat_id else f"{chat_id}@s.whatsapp.net"
        await self.send_message(jid, f"Reminder: {message}")

    async def deliver_schedule(self, chat_id: str, message: str):
        """Deliver a scheduled task to a WhatsApp user."""
        jid = chat_id if "@" in chat_id else f"{chat_id}@s.whatsapp.net"
        await self.send_message(jid, f"Scheduled: {message}")

    # ------------------------------------------------------------------ #
    #  Server lifecycle                                                   #
    # ------------------------------------------------------------------ #

    async def start(self):
        """Start the HTTP callback server."""
        self._app = web.Application()
        self._app.router.add_post("/whatsapp/incoming", self._handle_incoming)
        self._app.router.add_get("/whatsapp/health", self._health_endpoint)

        self._runner = web.AppRunner(self._app)
        await self._runner.setup()

        site = web.TCPSite(self._runner, self.listen_host, self.listen_port)
        await site.start()

        logger.info(
            f"WhatsApp adapter listening on {self.listen_host}:{self.listen_port}"
        )
        logger.info(f"Bridge URL: {self.bridge_url}")

        # Check bridge connectivity
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

    async def stop(self):
        """Stop the HTTP server and close session."""
        if self._runner:
            await self._runner.cleanup()
        if self._session and not self._session.closed:
            await self._session.close()
        logger.info("WhatsApp adapter stopped.")
