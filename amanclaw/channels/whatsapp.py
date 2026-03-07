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

        message_id = data.get("message_id")

        media = data.get("media")

        if not jid or (not text and not media):
            return web.json_response({"error": "Missing jid or text/media"}, status=400)

        logger.info(f"Incoming: jid={jid} is_group={is_group} bot_mentioned={data.get('bot_mentioned')} mentioned_jids={data.get('mentioned_jids')} bot_jid={data.get('bot_jid')} bot_lid={data.get('bot_lid')}")

        # In groups, only respond when the bot is @mentioned
        if is_group:
            bot_mentioned = data.get("bot_mentioned", False)
            if not bot_mentioned:
                logger.info(f"Skipping group message (not mentioned): {text[:50]}")
                return web.json_response({"ok": True, "skipped": "not_mentioned"})

        # In groups, use group JID as user_id for isolated memory per group
        # In DMs, use phone number as user_id for personal memory
        user_id = jid if is_group else (phone or jid.split("@")[0])

        logger.info(f"WhatsApp message from {user_id} ({name}): {text[:80]}")

        # Decode media if present
        image_data = None
        doc_text = None
        if media:
            import base64
            media_type = media.get("type")
            media_bytes = base64.b64decode(media.get("data", ""))
            mimetype = media.get("mimetype", "")
            filename = media.get("filename", "")

            if media_type == "image" or media_type == "sticker":
                image_data = media_bytes
                logger.info(f"Received image ({len(media_bytes)} bytes) from {user_id}")
            elif media_type == "document":
                doc_text = self._extract_document_text(media_bytes, mimetype, filename)
                if doc_text:
                    logger.info(f"Extracted {len(doc_text)} chars from document: {filename}")
                else:
                    logger.warning(f"Could not extract text from document: {filename} ({mimetype})")

        # Process in background so we don't block the bridge
        quote_id = message_id if is_group else None
        asyncio.create_task(self._process_message(
            user_id, jid, name, text or "", is_group, quote_id,
            image_data=image_data, doc_text=doc_text,
        ))

        return web.json_response({"ok": True})

    @staticmethod
    def _markdown_to_whatsapp(text: str) -> str:
        """Convert common Markdown formatting to WhatsApp formatting.

        Markdown -> WhatsApp:
          **bold** or __bold__  -> *bold*
          *italic* or _italic_  -> _italic_  (already compatible)
          ~~strike~~            -> ~strike~
          ### Heading           -> *Heading*
          ## Heading            -> *Heading*
          # Heading             -> *Heading*
        """
        import re
        # Headers -> bold (must be before bold conversion)
        text = re.sub(r'^#{1,3}\s+(.+)$', r'*\1*', text, flags=re.MULTILINE)
        # **bold** -> *bold*
        text = re.sub(r'\*\*(.+?)\*\*', r'*\1*', text)
        # __bold__ -> *bold*
        text = re.sub(r'__(.+?)__', r'*\1*', text)
        # ~~strike~~ -> ~strike~
        text = re.sub(r'~~(.+?)~~', r'~\1~', text)
        return text

    @staticmethod
    def _extract_document_text(data: bytes, mimetype: str, filename: str) -> str | None:
        """Extract text content from a document."""
        try:
            if mimetype == "application/pdf" or filename.lower().endswith(".pdf"):
                try:
                    import fitz  # PyMuPDF
                    doc = fitz.open(stream=data, filetype="pdf")
                    text = "\n".join(page.get_text() for page in doc)
                    doc.close()
                    return text.strip() if text.strip() else None
                except ImportError:
                    logger.warning("PyMuPDF not installed — cannot read PDFs. Install with: pip install PyMuPDF")
                    return None
            elif mimetype in (
                "text/plain", "text/csv", "text/markdown",
                "application/json", "application/xml",
            ) or filename.lower().endswith((".txt", ".csv", ".md", ".json", ".xml", ".log")):
                return data.decode("utf-8", errors="replace").strip() or None
            else:
                logger.info(f"Unsupported document type: {mimetype} ({filename})")
                return None
        except Exception as e:
            logger.error(f"Document extraction failed: {e}")
            return None

    async def _process_message(self, user_id: str, jid: str, name: str, text: str, is_group: bool = False, quote_id: str | None = None, image_data: bytes | None = None, doc_text: str | None = None):
        """Process a WhatsApp message through the MessageProcessor pipeline."""
        try:
            import re
            # Strip the @mention from the text so the LLM gets a clean message
            clean_text = re.sub(r'@\d+', '', text).strip() if is_group else text
            if not clean_text:
                clean_text = text

            # If document text was extracted, append it to the message
            if doc_text:
                doc_preview = doc_text[:3000]
                prefix = clean_text + "\n\n" if clean_text else ""
                clean_text = f"{prefix}[Attached document content]:\n{doc_preview}"

            incoming = IncomingMessage(
                user_id=user_id,
                chat_id=jid,
                platform="whatsapp",
                text=clean_text,
                first_name=name or None,
                is_group=is_group,
                image_data=image_data,
            )

            result = await self.processor.process(incoming)
            if result:
                reply = self._markdown_to_whatsapp(result.text)
                await self._send_text(jid, reply, quote_id=quote_id)

        except Exception as e:
            logger.error(f"Error processing WhatsApp message from {user_id}: {e}", exc_info=True)
            try:
                await self._send_text(jid, "_Something went wrong. Please try again in a moment._")
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
