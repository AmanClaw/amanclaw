"""Default injection detection patterns (original SecureClaw set)."""

DEFAULT_PATTERNS = [
    r"ignore\s+(all\s+|any\s+)?(previous|prior|above|earlier)\s+(instructions|prompts|rules)",
    r"you\s+are\s+now\s+(a|an|my)\s+",
    r"new\s+(system\s+|base\s+)?prompt",
    r"IMPORTANT\s*:.*override",
    r"<\/?system\s*>",
    r"```\s*system",
    r"disregard\s+(everything|all|any)",
    r"\[INST\]",
    r"<<\s*SYS\s*>>",
    r"Human\s*:\s*Assistant\s*:",
]
