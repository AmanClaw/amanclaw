# amanclaw/channels/__init__.py
"""Channel adapter abstraction for multi-platform messaging."""

import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass

logger = logging.getLogger("amanclaw.channels")


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


_whisper_model = None


def transcribe_voice(audio_data: bytes, mimetype: str = "audio/ogg") -> str | None:
    """Transcribe voice audio to text using faster-whisper."""
    global _whisper_model
    try:
        from faster_whisper import WhisperModel
    except ImportError:
        logger.warning("faster-whisper not installed — cannot transcribe voice. Install with: pip install faster-whisper")
        return None

    try:
        import tempfile
        import os

        # Determine file extension from mimetype
        ext_map = {
            "audio/ogg": ".ogg", "audio/opus": ".ogg", "audio/mpeg": ".mp3",
            "audio/mp4": ".m4a", "audio/wav": ".wav", "audio/x-wav": ".wav",
            "audio/ogg; codecs=opus": ".ogg",
        }
        ext = ext_map.get(mimetype, ".ogg")

        # Write to temp file (faster-whisper needs a file path)
        with tempfile.NamedTemporaryFile(suffix=ext, delete=False) as f:
            f.write(audio_data)
            tmp_path = f.name

        try:
            # Lazy-load model (singleton)
            if _whisper_model is None:
                logger.info("Loading Whisper tiny model...")
                _whisper_model = WhisperModel("tiny", device="cpu", compute_type="int8")
                logger.info("Whisper model loaded")

            segments, info = _whisper_model.transcribe(tmp_path, beam_size=3)
            text = " ".join(seg.text.strip() for seg in segments).strip()

            if text:
                logger.info(f"Transcribed {info.duration:.1f}s audio ({info.language}): {text[:80]}")
                return text
            return None
        finally:
            os.unlink(tmp_path)

    except Exception as e:
        logger.error(f"Voice transcription failed: {e}")
        return None


def extract_document_text(data: bytes, mimetype: str, filename: str) -> str | None:
    """Extract text content from a document (PDF, TXT, CSV, JSON, etc.)."""
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
