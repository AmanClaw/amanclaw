"""Authentication — allowlist-based auth with approval flow."""

import logging
from typing import Protocol, runtime_checkable

logger = logging.getLogger("amanclaw_security.auth")


@runtime_checkable
class AuthBackend(Protocol):
    """Storage interface for user auth state."""
    def get_user_status(self, user_id: str) -> str | None: ...
    def register_user(self, user_id: str, platform: str, **kwargs) -> None: ...


class Auth:
    """DB-backed auth with admin users and approval flow."""

    def __init__(self, config: dict, memory=None):
        self.memory = memory
        self.admins = {}
        for platform, ids in config.get("admin_users", {}).items():
            self.admins[platform] = set(str(uid) for uid in (ids or []))

    def is_admin(self, user_id, platform: str) -> bool:
        return str(user_id) in self.admins.get(platform, set())

    def is_authorized(self, user_id, platform: str) -> bool:
        uid = str(user_id)
        if self.is_admin(uid, platform):
            return True
        if self.memory:
            status = self.memory.get_user_status(uid)
            return status == "approved"
        return False

    def get_user_state(self, user_id, platform: str) -> str:
        """Returns: 'admin', 'approved', 'pending', 'blocked', or 'new'."""
        uid = str(user_id)
        if self.is_admin(uid, platform):
            return "admin"
        if self.memory:
            status = self.memory.get_user_status(uid)
            return status or "new"
        return "new"
