# OpenClaw Parity — Design Document

Date: 2026-03-07
Status: Approved

## Goal

Close the gap between AmanClaw and OpenClaw by implementing 4 features:
1. MCP client support (unlock external tools)
2. Standalone self-learning module (publishable package)
3. Discord + Slack channels (adapter pattern)
4. Standalone security library (publishable package)

## Constraints

- Keep the "weekend-buildable" ethos — no over-engineering
- SQLite stays as the storage backend
- All features are optional — disabled by default, enabled via config
- No breaking changes to existing Telegram/WhatsApp functionality
- Python 3.11+ only

---

## Feature 1: MCP Client Support

### Overview

Add an MCP client that discovers and connects to MCP servers at startup, converting their tools into AmanClaw skill definitions so the LLM can use them natively alongside built-in skills.

### Dependencies

- `mcp` (Python MCP SDK) — handles protocol, transports, tool discovery

### New Files

- `amanclaw/mcp_client.py` — MCP client manager

### Config Changes (config.yaml)

```yaml
mcp_servers:
  # stdio transport — spawns a subprocess
  filesystem:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/docs"]

  # SSE transport — connects to a running server
  my-api:
    url: "http://localhost:8080/sse"

  # Disabled server (won't connect)
  # github:
  #   command: "npx"
  #   args: ["-y", "@modelcontextprotocol/server-github"]
  #   env:
  #     GITHUB_TOKEN: "${GITHUB_TOKEN}"
```

### Design

```
MCPManager
├── __init__(config)          # Parse mcp_servers config
├── async start()             # Connect to all configured servers
├── async stop()              # Disconnect all servers, kill subprocesses
├── get_tool_definitions()    # Return all MCP tools as OpenAI-compatible tool defs
├── async execute(name, args) # Route tool call to correct server
└── _servers: dict[str, MCPServer]

MCPServer (internal)
├── name: str
├── client: mcp.Client
├── tools: list[Tool]         # Discovered tools from this server
└── transport: stdio | sse
```

### Integration Points

**Skill registry (`skills/__init__.py`):**
- `get_tool_definitions()` merges built-in skills + MCP tools
- `execute()` checks REGISTRY first, then delegates to MCPManager for unknown tools
- MCP tool names are prefixed: `mcp_<server>_<tool>` to avoid collisions

**LLM module (`llm.py`):**
- No changes needed — it already calls `get_tool_definitions()` and `execute()`
- MCP tools appear as regular tools to the LLM

**Bot startup (`bot.py`):**
- `MCPManager` is created and started during bot init
- Stopped during graceful shutdown

### Error Handling

- If an MCP server fails to connect, log warning and skip it (don't block startup)
- If an MCP tool call fails, return error string (same as skill failures)
- If an MCP server disconnects mid-session, mark its tools as unavailable
- Subprocess MCP servers are killed on bot shutdown (SIGTERM, then SIGKILL after 5s)

### Security

- MCP tool outputs go through `sanitize_skill_output()` (same as built-in skills)
- MCP servers run as configured — no additional sandboxing (user's responsibility)
- Environment variables in config support `${VAR}` substitution from .env

---

## Feature 2: Standalone Self-Learning Module

### Overview

Extract `learning.py` and its memory dependencies into a reusable package that can work with any storage backend.

### Package Structure

```
packages/amanclaw-learning/
├── pyproject.toml
├── README.md
└── src/
    └── amanclaw_learning/
        ├── __init__.py           # Public API: LearningEngine, MemoryBackend
        ├── engine.py             # LearningEngine (moved from learning.py)
        ├── patterns.py           # CORRECTION_PATTERNS, TEACHING_PATTERNS
        └── backend.py            # MemoryBackend protocol
```

### MemoryBackend Protocol

```python
from typing import Protocol, runtime_checkable

@runtime_checkable
class MemoryBackend(Protocol):
    """Storage interface for the learning engine."""

    # Knowledge CRUD
    def get_active_knowledge(self, user_id: str) -> list[dict]: ...
    def save_knowledge(self, user_id: str, category: str, subject: str,
                       content: str, **kwargs) -> int: ...
    def update_knowledge(self, knowledge_id: int, **kwargs) -> None: ...

    # Corrections
    def save_correction(self, user_id: str, knowledge_id: int,
                        old_content: str, new_content: str,
                        trigger_text: str) -> int: ...
    def get_corrections(self, user_id: str, limit: int = 10) -> list[dict]: ...

    # Teachings
    def save_teaching(self, user_id: str, trigger_pattern: str,
                      response_guidance: str, category: str) -> int: ...
    def get_teachings(self, user_id: str, active_only: bool = True) -> list[dict]: ...
    def increment_teaching_usage(self, teaching_id: int) -> None: ...
    def deactivate_teaching(self, teaching_id: int) -> None: ...

    # Documents
    def save_document_chunk(self, user_id: str, source_name: str,
                            source_type: str, chunk_index: int, text: str) -> None: ...
    def delete_document(self, user_id: str, source_name: str) -> None: ...
    def list_documents(self, user_id: str) -> list[dict]: ...

    # Failures
    def save_failure(self, user_id: str, skill_name: str,
                     input_json: str, error_message: str) -> int: ...
    def get_recent_failures(self, user_id: str, limit: int = 20) -> list[dict]: ...

    # Behavioral patterns
    def get_behavioral_patterns(self, user_id: str,
                                min_confidence: float = 0.5) -> list[dict]: ...
```

### Integration

- Main project adds `amanclaw-learning` as a path dependency in `pyproject.toml`:
  ```toml
  [tool.setuptools.packages.find]
  where = [".", "packages/amanclaw-learning/src"]
  ```
- `amanclaw/memory.py` (Memory class) already satisfies `MemoryBackend` — no changes needed
- `amanclaw/learning.py` becomes a thin re-export:
  ```python
  from amanclaw_learning import LearningEngine, MemoryBackend
  ```
- Existing imports throughout the project remain unchanged

### What Moves vs What Stays

| Component | Moves to package | Stays in main |
|-----------|-----------------|---------------|
| `LearningEngine` class | Yes | Re-exported |
| `CORRECTION_PATTERNS` | Yes | No |
| `TEACHING_PATTERNS` | Yes | No |
| `MemoryBackend` protocol | Yes (new) | No |
| `Memory` class (SQLite impl) | No | Yes (implements protocol) |
| Learning skills (`/teach`, etc.) | No | Yes |
| Bot integration (check-ins) | No | Yes |

---

## Feature 3: Discord + Slack Channels

### Overview

Refactor bot.py to separate channel-specific code from core message processing, then add Discord and Slack adapters.

### New Directory

```
amanclaw/channels/
├── __init__.py       # ChannelAdapter ABC, IncomingMessage, OutgoingMessage
├── telegram.py       # TelegramAdapter (extracted from bot.py)
├── whatsapp.py       # WhatsAppAdapter (moved from whatsapp.py)
├── discord.py        # NEW: DiscordAdapter
└── slack.py          # NEW: SlackAdapter
```

### ChannelAdapter ABC

```python
from abc import ABC, abstractmethod
from dataclasses import dataclass

@dataclass
class IncomingMessage:
    user_id: str
    chat_id: str
    platform: str
    text: str
    username: str | None = None
    first_name: str | None = None
    is_group: bool = False
    image_data: bytes | None = None   # For vision support
    reply_to: str | None = None       # For threaded replies

@dataclass
class OutgoingMessage:
    chat_id: str
    text: str
    parse_mode: str | None = None     # "markdown", "html", or None
    reply_to: str | None = None

class ChannelAdapter(ABC):
    @abstractmethod
    async def start(self) -> None: ...

    @abstractmethod
    async def stop(self) -> None: ...

    @abstractmethod
    async def send_message(self, msg: OutgoingMessage) -> None: ...

    @property
    @abstractmethod
    def platform(self) -> str: ...
```

### Core Message Processor

Extract from `bot.py` into a new `amanclaw/processor.py`:

```python
class MessageProcessor:
    """Channel-agnostic message processing pipeline."""

    def __init__(self, config, auth, rate_limiter, memory, llm, learning):
        ...

    async def process(self, msg: IncomingMessage) -> str | None:
        """
        Full pipeline: auth -> rate limit -> sanitize -> context -> LLM -> learn.
        Returns response text, or None if message was rejected.
        """
        ...
```

This is the core logic currently in `bot.py:handle_message()` and `whatsapp.py:_process_message()` — deduplicated into one place.

### Discord Adapter

```yaml
# config.yaml
discord:
  enabled: false
  # token loaded from DISCORD_BOT_TOKEN env var
  allowed_channels: []    # Empty = all channels; list channel IDs to restrict
  command_prefix: "!"     # Optional prefix for commands
```

- Uses `discord.py` library (async, well-maintained)
- Listens for messages in allowed channels or DMs
- Supports file attachments for vision
- Maps Discord user ID to AmanClaw user_id as `discord:<user_id>`
- Message length limit: 2000 chars (auto-split)

### Slack Adapter

```yaml
# config.yaml
slack:
  enabled: false
  # tokens loaded from SLACK_BOT_TOKEN and SLACK_APP_TOKEN env vars
  socket_mode: true       # Recommended for personal use (no public URL needed)
  allowed_channels: []
```

- Uses `slack-bolt` library with Socket Mode (no public URL required)
- Listens for app_mention and direct messages
- Maps Slack user ID to AmanClaw user_id as `slack:<user_id>`
- Supports threaded replies
- Message length limit: 4000 chars (auto-split)

### Bot.py Refactor

**Before:** `bot.py` is ~400 lines mixing Telegram-specific handlers with core logic.

**After:**
- `bot.py` becomes the orchestrator — creates adapters, starts/stops them
- `processor.py` holds the shared message processing pipeline
- `channels/telegram.py` holds Telegram-specific handler code
- Telegram-specific features (inline keyboards, commands, photo handling) stay in the adapter

**Migration strategy:**
1. Create `processor.py` with `MessageProcessor` extracted from `bot.py`
2. Create `channels/__init__.py` with ABC
3. Create `channels/telegram.py` wrapping existing Telegram code
4. Move `whatsapp.py` to `channels/whatsapp.py`
5. Add `channels/discord.py` and `channels/slack.py`
6. Slim down `bot.py` to orchestration only

### Dependencies

- `discord.py >= 2.3` (optional, only if discord enabled)
- `slack-bolt >= 1.18` (optional, only if slack enabled)

Optional deps handled via extras in pyproject.toml:
```toml
[project.optional-dependencies]
discord = ["discord.py>=2.3"]
slack = ["slack-bolt>=1.18", "slack-sdk>=3.27"]
```

---

## Feature 4: Standalone Security Library

### Overview

Extract and expand `security.py` into a reusable package with configurable security policies.

### Package Structure

```
packages/amanclaw-security/
├── pyproject.toml
├── README.md
└── src/
    └── amanclaw_security/
        ├── __init__.py           # Public API
        ├── auth.py               # Auth class (allowlist, approval flow)
        ├── rate_limit.py         # RateLimiter (sliding window)
        ├── injection.py          # Injection detection (expanded patterns)
        ├── sanitize.py           # Output sanitization
        ├── policy.py             # SecurityPolicy (bundles everything)
        └── rules/
            ├── __init__.py
            ├── owasp_agentic.py  # OWASP Agentic Top 10 rule set
            └── default.py        # Default rules (current patterns)
```

### SecurityPolicy Class

```python
class SecurityPolicy:
    """Configurable security policy for AI agent applications."""

    def __init__(
        self,
        auth_backend: AuthBackend | None = None,
        rate_limit: int = 20,        # msgs per minute, 0 = disabled
        injection_rules: str = "default",  # "default", "owasp_agentic", or custom
        sanitize_output: bool = True,
    ):
        ...

    def check_auth(self, user_id: str, platform: str) -> AuthResult: ...
    def check_rate(self, user_id: str) -> bool: ...
    def check_input(self, text: str) -> SanitizeResult: ...
    def sanitize_output(self, output: str) -> str: ...
```

### Expanded Injection Rules (OWASP Agentic Top 10)

Current: 10 patterns focused on prompt injection.

Expanded rule sets:
- **default** — current 10 patterns (backward compatible)
- **owasp_agentic** — covers all OWASP Agentic Top 10 categories:
  - Prompt injection (existing + expanded)
  - Tool misuse patterns (requests to chain dangerous tools)
  - Excessive agency indicators (requests for autonomous action loops)
  - Data exfiltration patterns (requests to send data to external URLs)
  - Privilege escalation patterns (requests to modify own permissions)

### AuthBackend Protocol

```python
@runtime_checkable
class AuthBackend(Protocol):
    def get_user_status(self, user_id: str) -> str | None: ...
    def register_user(self, user_id: str, platform: str, **kwargs) -> None: ...
```

### Integration

Same pattern as learning module:
- Path dependency in main project
- `amanclaw/security.py` becomes a thin re-export
- `Memory` class satisfies `AuthBackend` protocol (already has `get_user_status`)

---

## Config.yaml — Final Shape

```yaml
llm:
  base_url: "https://..."
  model: "Qwen/Qwen3-VL-30B-A3B-Instruct"
  max_tokens: 4096
  temperature: 0.7

admin_users:
  telegram: [38403796]

rate_limit_per_minute: 20
memory_db: memory.db

# --- Channels (all optional) ---
whatsapp:
  enabled: true
  bridge_url: "http://localhost:3001"
  port: 3002
  ignore_groups: false

discord:
  enabled: false
  allowed_channels: []
  command_prefix: "!"

slack:
  enabled: false
  socket_mode: true
  allowed_channels: []

# --- MCP Servers (optional) ---
mcp_servers:
  # filesystem:
  #   command: "npx"
  #   args: ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/docs"]

# --- Skills ---
skills:
  shell_allowed_commands: [ls, cat, head, tail, wc, grep, find, which, df, du, free, uptime, date, ps, whoami, hostname, pwd, tree]
  shell_working_dir: "~"
  workspace_dir: "~/amanclaw-workspace"
  skill_timeout_seconds: 30

# --- Security (optional overrides) ---
security:
  injection_rules: "default"     # "default" or "owasp_agentic"
  sanitize_output: true

# --- Learning (optional overrides) ---
learning:
  proactive_checkin: true
  checkin_interval_days: 7
  document_chunk_size: 500
```

---

## Implementation Order

| Phase | Feature | Depends On | New Deps |
|-------|---------|------------|----------|
| 1 | MCP Client | None | `mcp` |
| 2 | Standalone Learning | None | None |
| 3 | Channel Adapters + Discord + Slack | None (parallel with 1-2) | `discord.py`, `slack-bolt` |
| 4 | Standalone Security | None (parallel with 1-3) | None |

Phases 1-4 are independent and can be implemented in parallel by separate agents.

---

## Testing Strategy

| Feature | Test Approach |
|---------|--------------|
| MCP Client | Mock MCP server, test tool discovery + execution + error handling |
| Learning Package | Existing `test_learning.py` + new tests with mock MemoryBackend |
| Channel Adapters | Mock each platform SDK, test message routing through processor |
| Security Package | Existing `test_security.py` + new tests for OWASP rules + policy class |
| Integration | End-to-end test: message in -> through processor -> response out |

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| MCP SDK is young / API may change | Pin version, wrap in our MCPManager abstraction |
| Bot.py refactor breaks Telegram | Extract incrementally, keep all Telegram tests passing at each step |
| Optional deps cause import errors | Lazy imports with helpful error messages ("pip install amanclaw[discord]") |
| SQLite contention with multiple channels | Already single-threaded writes; add WAL mode if needed |
| Package extraction breaks imports | Re-export from original locations for backward compat |
