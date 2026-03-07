# amanclaw/channels/slack.py
"""Slack adapter — connects to Slack via slack-bolt with Socket Mode."""

import os
import logging
from amanclaw.channels import ChannelAdapter, IncomingMessage, OutgoingMessage

logger = logging.getLogger("amanclaw.channels.slack")

MAX_MESSAGE_LENGTH = 4000


class SlackAdapter(ChannelAdapter):
    """Slack messaging adapter using slack-bolt."""

    def __init__(self, config: dict, processor):
        self.config = config
        self.processor = processor
        slack_config = config.get("slack", {})
        self.allowed_channels = set(str(c) for c in slack_config.get("allowed_channels", []))
        self.socket_mode = slack_config.get("socket_mode", True)
        self._app = None
        self._handler = None

    @property
    def platform(self) -> str:
        return "slack"

    def _split_message(self, text: str) -> list[str]:
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

    async def start(self) -> None:
        try:
            from slack_bolt.async_app import AsyncApp
            from slack_bolt.adapter.socket_mode.async_handler import AsyncSocketModeHandler
        except ImportError:
            logger.error("slack-bolt not installed. Install with: pip install amanclaw[slack]")
            return

        bot_token = os.environ.get("SLACK_BOT_TOKEN")
        app_token = os.environ.get("SLACK_APP_TOKEN")
        if not bot_token:
            logger.error("SLACK_BOT_TOKEN not set in environment")
            return
        if self.socket_mode and not app_token:
            logger.error("SLACK_APP_TOKEN not set for socket mode")
            return

        self._app = AsyncApp(token=bot_token)
        adapter = self

        @self._app.event("message")
        async def handle_message(event, say):
            # Skip bot messages
            if event.get("bot_id") or event.get("subtype"):
                return

            channel = event.get("channel", "")
            if adapter.allowed_channels and channel not in adapter.allowed_channels:
                return

            user_id = f"slack:{event.get('user', 'unknown')}"
            text = event.get("text", "")
            if not text:
                return

            thread_ts = event.get("thread_ts") or event.get("ts")

            incoming = IncomingMessage(
                user_id=user_id,
                chat_id=channel,
                platform="slack",
                text=text,
                is_group=event.get("channel_type") != "im",
                reply_to=thread_ts,
            )

            result = await adapter.processor.process(incoming)
            if result:
                for chunk in adapter._split_message(result.text):
                    await say(text=chunk, thread_ts=thread_ts)

        @self._app.event("app_mention")
        async def handle_mention(event, say):
            # Reuse message handler for mentions
            await handle_message(event, say)

        if self.socket_mode:
            self._handler = AsyncSocketModeHandler(self._app, app_token)
            import asyncio
            asyncio.create_task(self._handler.start_async())
            logger.info("Slack adapter starting in socket mode...")
        else:
            logger.info("Slack adapter: HTTP mode not yet implemented, use socket_mode: true")

    async def stop(self) -> None:
        if self._handler:
            await self._handler.close_async()
            logger.info("Slack adapter stopped")

    async def send_message(self, msg: OutgoingMessage) -> None:
        if not self._app:
            return
        from slack_sdk.web.async_client import AsyncWebClient
        client: AsyncWebClient = self._app.client
        for chunk in self._split_message(msg.text):
            await client.chat_postMessage(
                channel=msg.chat_id,
                text=chunk,
                thread_ts=msg.reply_to,
            )
