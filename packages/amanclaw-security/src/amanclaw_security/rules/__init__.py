"""Security rule sets."""

from amanclaw_security.rules.default import DEFAULT_PATTERNS
from amanclaw_security.rules.owasp_agentic import OWASP_AGENTIC_PATTERNS

RULE_SETS = {
    "default": DEFAULT_PATTERNS,
    "owasp_agentic": OWASP_AGENTIC_PATTERNS,
}
