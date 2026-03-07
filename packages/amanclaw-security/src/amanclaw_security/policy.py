"""SecurityPolicy — unified security policy for AI agent applications."""

from dataclasses import dataclass
from amanclaw_security.auth import Auth, AuthBackend
from amanclaw_security.rate_limit import RateLimiter
from amanclaw_security.injection import check_injection
from amanclaw_security.sanitize import sanitize_output


@dataclass
class AuthResult:
    authorized: bool
    state: str  # "admin", "approved", "pending", "blocked", "new"


@dataclass
class SanitizeResult:
    text: str
    flagged: bool


class SecurityPolicy:
    """Configurable security policy for AI agent applications."""

    def __init__(
        self,
        auth_backend: AuthBackend | None = None,
        admin_users: dict | None = None,
        rate_limit: int = 20,
        injection_rules: str = "default",
        do_sanitize_output: bool = True,
    ):
        config = {"admin_users": admin_users or {}}
        self._auth = Auth(config, memory=auth_backend)
        self._rate_limiter = RateLimiter(max_per_minute=rate_limit) if rate_limit > 0 else None
        self._injection_rules = injection_rules
        self._do_sanitize_output = do_sanitize_output

    def check_auth(self, user_id: str, platform: str) -> AuthResult:
        state = self._auth.get_user_state(user_id, platform)
        authorized = state in ("admin", "approved")
        return AuthResult(authorized=authorized, state=state)

    def check_rate(self, user_id: str) -> bool:
        if not self._rate_limiter:
            return True
        return self._rate_limiter.check(user_id)

    def check_input(self, text: str) -> SanitizeResult:
        _, flagged = check_injection(text, rules=self._injection_rules)
        return SanitizeResult(text=text, flagged=flagged)

    def sanitize_tool_output(self, output: str) -> str:
        if not self._do_sanitize_output:
            return output
        return sanitize_output(output)
