"""Per-user sliding window rate limiter."""

import time


class RateLimiter:
    def __init__(self, max_per_minute: int = 20):
        self.max_per_minute = max_per_minute
        self.windows: dict[str, list[float]] = {}

    def check(self, user_id: str) -> bool:
        now = time.time()
        key = str(user_id)
        if key not in self.windows:
            self.windows[key] = []
        self.windows[key] = [t for t in self.windows[key] if now - t < 60]
        if len(self.windows[key]) >= self.max_per_minute:
            return False
        self.windows[key].append(now)
        return True
