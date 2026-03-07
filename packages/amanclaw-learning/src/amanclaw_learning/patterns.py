"""Detection patterns for corrections and teachings."""

CORRECTION_PATTERNS = [
    r"\bno[,.]?\s+(i\s+)?(meant|prefer|want|like|use|need)",
    r"\bactually[,.]?\s+(it'?s|my|i)",
    r"\bwrong[,.]",
    r"\bthat'?s\s+not\s+(right|correct)",
    r"\bnot\s+\w+[,.]?\s+(it'?s|i\s+meant)",
    r"\bcorrection[:\s]",
    r"\bi\s+said\s+\w+[,.]?\s+not\s+",
]

TEACHING_PATTERNS = [
    r"\bremember\s+that\b",
    r"\balways\s+(respond|answer|reply|do|use)",
    r"\bfrom\s+now\s+on\b",
    r"\bteach:\s*",
    r"\bwhen\s+i\s+(say|ask|write|type)\b.*\b(mean|do|use|respond)",
    r"\bnever\s+(respond|answer|reply|do|use)",
    r"\bkeep\s+(answers?|responses?)\s+(short|brief|long|detailed)",
]
