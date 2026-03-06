"""
Document skill — extract text from PDF and common document formats.
Uses only Python stdlib + lightweight packages (no heavy deps).
"""

import os
import logging
from pathlib import Path
from amanclaw.skills import skill

logger = logging.getLogger("amanclaw.skills.documents")

WORKSPACE = Path.home() / "amanclaw-workspace"


def configure(workspace_dir: str = None):
    global WORKSPACE
    if workspace_dir:
        WORKSPACE = Path(workspace_dir).expanduser().resolve()


def _safe_path(filepath: str) -> Path:
    resolved = (WORKSPACE / filepath).resolve()
    if not str(resolved).startswith(str(WORKSPACE)):
        raise ValueError(f"Path escapes workspace: {filepath}")
    return resolved


@skill(
    name="read_document",
    description="Extract and read text from a document file (PDF, TXT, CSV, JSON, Markdown) in the workspace. Send a file to Telegram first, then use this to analyze it.",
    parameters={
        "path": {
            "type": "string",
            "description": "Relative path to the document in the workspace",
        },
        "max_chars": {
            "type": "integer",
            "description": "Maximum characters to return (default 5000)",
            "optional": True,
        },
    },
    timeout=30,
)
def read_document(path: str, max_chars: int = 5000) -> str:
    try:
        safe = _safe_path(path)
        if not safe.exists():
            return f"File not found: {path}"

        suffix = safe.suffix.lower()

        if suffix == ".pdf":
            return _read_pdf(safe, max_chars)
        elif suffix in (".txt", ".md", ".csv", ".tsv", ".json", ".yaml", ".yml", ".xml", ".html", ".log"):
            text = safe.read_text(encoding="utf-8", errors="replace")
            if len(text) > max_chars:
                return text[:max_chars] + f"\n\n[Truncated — {len(text)} total chars]"
            return text
        else:
            return f"Unsupported format: {suffix}. Supported: PDF, TXT, MD, CSV, JSON, YAML, XML, HTML, LOG"
    except ValueError as e:
        return str(e)
    except Exception as e:
        return f"Error reading document: {e}"


def _read_pdf(path: Path, max_chars: int) -> str:
    """Extract text from PDF. Tries pypdf first, falls back to basic extraction."""
    try:
        import pypdf
        reader = pypdf.PdfReader(str(path))
        pages = []
        total_chars = 0
        for i, page in enumerate(reader.pages):
            text = page.extract_text() or ""
            if total_chars + len(text) > max_chars:
                text = text[:max_chars - total_chars]
                pages.append(f"--- Page {i+1} ---\n{text}")
                pages.append(f"\n[Truncated at page {i+1} of {len(reader.pages)}]")
                break
            pages.append(f"--- Page {i+1} ---\n{text}")
            total_chars += len(text)
        return "\n\n".join(pages) if pages else "PDF has no extractable text."
    except ImportError:
        return "PDF support requires pypdf. Install with: pip install pypdf"
    except Exception as e:
        return f"Error reading PDF: {e}"
