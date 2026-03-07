"""
Security module — re-exported from standalone amanclaw-security package.

All imports from amanclaw.security continue to work.
"""

from amanclaw_security.auth import Auth, AuthBackend
from amanclaw_security.rate_limit import RateLimiter
from amanclaw_security.injection import check_injection
from amanclaw_security.sanitize import sanitize_output


def sanitize(text: str) -> tuple[str, bool]:
    """Check text for injection patterns. Backward-compatible wrapper."""
    return check_injection(text, rules="default")


def sanitize_skill_output(output: str) -> str:
    """Sanitize skill output. Backward-compatible wrapper."""
    return sanitize_output(output)


__all__ = ["Auth", "AuthBackend", "RateLimiter", "sanitize", "sanitize_skill_output"]
