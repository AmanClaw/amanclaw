# Channel Adapter Extraction — Design Document

Date: 2026-03-07
Status: Approved

## Goal

Complete the OpenClaw parity by extracting the Telegram adapter from `bot.py` into `channels/telegram.py` and moving the WhatsApp adapter from `amanclaw/whatsapp.py` to `channels/whatsapp.py`, both implementing the existing `ChannelAdapter` ABC.

## Constraints

- No feature regressions — all Telegram-specific features (inline keyboards, commands, callbacks, typing indicators, photo/voice, /addskill wizard) stay intact
- Backward compatibility — `from amanclaw.whatsapp import WhatsAppAdapter` still works
- No config changes
- All existing tests pass

---

## File Changes

### bot.py (~1550 → ~200 lines)

Keeps only:
- `load_config()`, `setup_logging()`, `JsonFormatter`
- Globals initialization
- `main()` — creates adapters, starts them
- `post_init` / `post_shutdown`
- Job callbacks: `check_reminders`, `check_schedules`, `prune_job`, `checkin_job`
- `error_handler`

### channels/telegram.py (NEW, ~900 lines)

All Telegram-specific code moves here:
- `TelegramAdapter(ChannelAdapter)` class
- Constructor takes: `config`, `processor`, `memory`, `llm`, `learning_engine`
- All handlers: `handle_message`, `handle_photo`, `handle_voice`
- All commands: `cmd_start`, `cmd_skills`, `cmd_clear`, `cmd_status`, `cmd_export`, `cmd_myid`, `cmd_approve`, `cmd_block`, `cmd_users`, `cmd_teach`, `cmd_learned`, `cmd_forget`, `cmd_myskills`, `cmd_delskill`, `cmd_publish`, `cmd_marketplace`, `cmd_addskill`
- Callback handler: `handle_callback`
- Helpers: `send_typing_periodically`, `reply_with_markdown`, `send_long_reply`, `send_approval_welcome`
- `/addskill` state machine

### channels/whatsapp.py (MOVED + refactored, ~150 lines)

- Moved from `amanclaw/whatsapp.py`
- Implements `ChannelAdapter` ABC
- HTTP callback server + bridge REST client (platform-specific, kept)
- Replaces duplicate auth/sanitize/LLM pipeline with `self.processor.process(IncomingMessage)`
- Keeps `deliver_reminder` / `deliver_schedule` (needed by bot.py jobs)

### processor.py (~150 → ~250 lines)

Gains from bot.py:
- `build_context()` logic merged into `MessageProcessor.process()` (teachings, documents, behavioral patterns)
- `extract_and_save_knowledge()` becomes `MessageProcessor._extract_knowledge()` (full version, replaces stub)
- Eliminates circular import: `processor.py → bot.py → whatsapp.py → bot.py`

### amanclaw/whatsapp.py (backward compat re-export)

```python
from amanclaw.channels.whatsapp import WhatsAppAdapter
```

---

## Integration Points

- `bot.py:main()` creates `TelegramAdapter` and passes it the application builder
- `bot.py:check_reminders/check_schedules` reference whatsapp adapter for delivery
- `TelegramAdapter.start()` registers all handlers on the Telegram Application
- `WhatsAppAdapter` uses `MessageProcessor.process()` instead of own pipeline

## Testing

- Existing tests continue to pass (import paths preserved)
- New test: `tests/test_telegram_adapter.py` — verify handler registration and message flow
- Updated test: `tests/test_processor.py` — covers enhanced build_context + knowledge extraction
