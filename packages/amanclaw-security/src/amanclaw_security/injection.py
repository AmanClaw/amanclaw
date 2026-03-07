"""Injection detection using configurable rule sets."""

import re
import logging
from amanclaw_security.rules import RULE_SETS

logger = logging.getLogger("amanclaw_security.injection")


def check_injection(text: str, rules: str = "default") -> tuple[str, bool]:
    """
    Check text for injection patterns.
    Returns (text, was_flagged).
    """
    patterns = RULE_SETS.get(rules, RULE_SETS["default"])
    compiled = [re.compile(p, re.IGNORECASE) for p in patterns]

    for pattern in compiled:
        if pattern.search(text):
            logger.warning(f"Injection pattern detected: {pattern.pattern}")
            return text, True
    return text, False
