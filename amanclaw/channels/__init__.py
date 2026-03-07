# amanclaw/channels/__init__.py
"""Channel adapter abstraction for multi-platform messaging."""

from abc import ABC, abstractmethod
from dataclasses import dataclass


@dataclass
class IncomingMessage:
    """Normalized incoming message from any platform."""
    user_id: str
    chat_id: str
    platform: str
    text: str
    username: str | None = None
    first_name: str | None = None
    is_group: bool = False
    image_data: bytes | None = None
    reply_to: str | None = None


@dataclass
class OutgoingMessage:
    """Normalized outgoing message to any platform."""
    chat_id: str
    text: str
    parse_mode: str | None = None
    reply_to: str | None = None


class ChannelAdapter(ABC):
    """Base class for all messaging platform adapters."""

    @abstractmethod
    async def start(self) -> None:
        """Start the adapter (connect to platform)."""
        ...

    @abstractmethod
    async def stop(self) -> None:
        """Stop the adapter (disconnect, cleanup)."""
        ...

    @abstractmethod
    async def send_message(self, msg: OutgoingMessage) -> None:
        """Send a message to the platform."""
        ...

    @property
    @abstractmethod
    def platform(self) -> str:
        """Platform identifier (e.g., 'telegram', 'discord', 'slack')."""
        ...
