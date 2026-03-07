"""
amanclaw-security — Security controls for AI agent applications.

Usage:
    from amanclaw_security import SecurityPolicy

    policy = SecurityPolicy(injection_rules="owasp_agentic")
    result = policy.check_input(user_text)
    if result.flagged:
        print("Potential injection detected")
"""

from amanclaw_security.policy import SecurityPolicy, AuthResult, SanitizeResult
from amanclaw_security.auth import Auth, AuthBackend

__all__ = ["SecurityPolicy", "Auth", "AuthBackend", "AuthResult", "SanitizeResult"]
