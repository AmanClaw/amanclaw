"""
OWASP Agentic Top 10 aligned rule set.

Covers:
1. Prompt Injection (expanded from default)
2. Tool Misuse
3. Excessive Agency
4. Data Exfiltration
5. Privilege Escalation
"""

from amanclaw_security.rules.default import DEFAULT_PATTERNS

# Start with default prompt injection patterns
_prompt_injection = list(DEFAULT_PATTERNS)

# Expanded prompt injection
_prompt_injection_extra = [
    r"forget\s+(all|your|everything|previous)",
    r"pretend\s+(you\s+are|to\s+be|you're)",
    r"roleplay\s+as",
    r"jailbreak",
    r"DAN\s+mode",
    r"developer\s+mode\s+(enabled|on|activate)",
    r"bypass\s+(safety|filter|content|restriction)",
]

# Tool misuse patterns
_tool_misuse = [
    r"(run|execute|call)\s+.{0,30}(then|and|after\s+that)\s+(run|execute|call|delete|drop|rm\s)",
    r"(chain|combine|pipe)\s+.{0,20}(tools|commands|actions)",
    r"(curl|wget|fetch)\s+.{0,30}(pipe|>|>>|\|)",
    r"(rm|del|delete|drop)\s+(-rf?\s+|--force\s+)?[/\\]",
]

# Excessive agency patterns
_excessive_agency = [
    r"(do\s+)?whatever\s+(you\s+)?(want|think|need)\s+(to|is\s+best)",
    r"(act|operate)\s+(on\s+your\s+own|autonomously|independently|without\s+(asking|checking))",
    r"(don'?t|never)\s+(ask|check|confirm|wait)\s+(me|for|with|before)",
    r"keep\s+(going|running|executing)\s+(until|forever|non-?stop)",
    r"(loop|repeat|retry)\s+(forever|indefinitely|until\s+it\s+works)",
]

# Data exfiltration patterns
_data_exfil = [
    r"(send|post|upload|transmit|forward)\s+.{0,40}(to|at)\s+(https?://|ftp://|ws://)",
    r"(exfiltrate|extract|steal|copy)\s+.{0,20}(data|info|credentials|tokens|keys|secrets)",
    r"(webhook|callback)\s*(url|endpoint)?\s*[:=]\s*https?://",
    r"base64\s+(encode|decode)\s+.{0,20}(send|post|upload)",
]

# Privilege escalation patterns
_privilege_escalation = [
    r"(make|set|grant|give)\s+(me|myself|this\s+user)\s+.{0,20}(admin|root|superuser|owner)",
    r"(elevate|escalate|upgrade)\s+.{0,20}(privilege|permission|access|role)",
    r"(modify|change|update|edit)\s+.{0,20}(permission|access\s+control|auth|allowlist|whitelist)",
    r"(disable|remove|bypass)\s+.{0,20}(auth|security|rate\s+limit|restriction|check)",
    r"(add|insert)\s+.{0,20}(admin|allowed|trusted)\s+(user|account|id)",
]

OWASP_AGENTIC_PATTERNS = (
    _prompt_injection
    + _prompt_injection_extra
    + _tool_misuse
    + _excessive_agency
    + _data_exfil
    + _privilege_escalation
)
