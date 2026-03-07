# amanclaw/channels/discord.py
"""Discord adapter — connects to Discord via discord.py."""

import os
import logging
from amanclaw.channels import ChannelAdapter, IncomingMessage, OutgoingMessage

logger = logging.getLogger("amanclaw.channels.discord")

MAX_MESSAGE_LENGTH = 2000


class DiscordAdapter(ChannelAdapter):
    """Discord messaging adapter using discord.py."""

    def __init__(self, config: dict, processor):
        self.config = config
        self.processor = processor
        dc_config = config.get("discord", {})
        self.allowed_channels = set(str(c) for c in dc_config.get("allowed_channels", []))
        self.command_prefix = dc_config.get("command_prefix", "!")
        self._client = None

    @property
    def platform(self) -> str:
        return "discord"

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
            import discord
        except ImportError:
            logger.error("discord.py not installed. Install with: pip install amanclaw[discord]")
            return

        token = os.environ.get("DISCORD_BOT_TOKEN")
        if not token:
            logger.error("DISCORD_BOT_TOKEN not set in environment")
            return

        intents = discord.Intents.default()
        intents.message_content = True
        self._client = discord.Client(intents=intents)

        adapter = self  # capture for closure

        @self._client.event
        async def on_ready():
            logger.info(f"Discord connected as {self._client.user}")

        @self._client.event
        async def on_message(message):
            # Ignore own messages
            if message.author == self._client.user:
                return

            # Check channel restrictions
            if adapter.allowed_channels and str(message.channel.id) not in adapter.allowed_channels:
                # Allow DMs always
                if not isinstance(message.channel, discord.DMChannel):
                    return

            user_id = f"discord:{message.author.id}"
            chat_id = str(message.channel.id)
            text = message.content

            if not text:
                return

            # Handle image attachments
            image_data = None
            for attachment in message.attachments:
                if attachment.content_type and attachment.content_type.startswith("image/"):
                    image_data = await attachment.read()
                    break

            incoming = IncomingMessage(
                user_id=user_id,
                chat_id=chat_id,
                platform="discord",
                text=text,
                username=str(message.author),
                first_name=message.author.display_name,
                is_group=not isinstance(message.channel, discord.DMChannel),
                image_data=image_data,
            )

            result = await adapter.processor.process(incoming)
            if result:
                for chunk in adapter._split_message(result.text):
                    await message.channel.send(chunk)

        # Start in background (non-blocking)
        import asyncio
        asyncio.create_task(self._client.start(token))
        logger.info("Discord adapter starting...")

    async def stop(self) -> None:
        if self._client:
            await self._client.close()
            logger.info("Discord adapter stopped")

    async def send_message(self, msg: OutgoingMessage) -> None:
        if not self._client:
            return
        channel = self._client.get_channel(int(msg.chat_id))
        if channel:
            for chunk in self._split_message(msg.text):
                await channel.send(chunk)
