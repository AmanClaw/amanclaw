"""
AmanClaw — Main bot entry point.

Usage:
    python -m amanclaw

Features: Markdown rendering, photo/voice support, inline keyboards,
          reminders, conversation export, persistent typing indicator.
"""

import io
import os
import sys
import yaml
import asyncio
import logging
from pathlib import Path
from dotenv import load_dotenv

# Load .env before anything else reads env vars
load_dotenv()
from datetime import datetime, time as datetime_time
from telegram import Update, InlineKeyboardButton, InlineKeyboardMarkup, BotCommand
from telegram.ext import (
    ApplicationBuilder,
    CommandHandler,
    MessageHandler,
    CallbackQueryHandler,
    ContextTypes,
    filters,
)
from telegram.constants import ParseMode, ChatAction

from amanclaw.security import Auth, RateLimiter, sanitize
from amanclaw.memory import Memory
from amanclaw.llm import LLM
from amanclaw.skills import get_skill_list
from amanclaw.skills.shell import configure as configure_shell
from amanclaw.skills.files import configure as configure_files
from amanclaw.skills.remember import configure as configure_remember, set_current_user
from amanclaw.skills.reminder import configure as configure_reminder, set_context as set_reminder_context
from amanclaw.skills.scheduled import configure as configure_scheduled, set_context as set_scheduled_context
from amanclaw.skills.documents import configure as configure_documents
from amanclaw.whatsapp import WhatsAppAdapter


class JsonFormatter(logging.Formatter):
    """JSON log formatter for structured logging (Docker, log aggregators)."""
    def format(self, record):
        import json as _json
        log_data = {
            "ts": datetime.now().isoformat(),
            "level": record.levelname,
            "logger": record.name,
            "msg": record.getMessage(),
        }
        if record.exc_info and record.exc_info[0]:
            log_data["exception"] = self.formatException(record.exc_info)
        return _json.dumps(log_data)


def setup_logging():
    """Configure logging with console + optional rotating file output."""
    log_level = os.environ.get("LOG_LEVEL", "INFO").upper()
    log_file = os.environ.get("LOG_FILE")  # e.g. "amanclaw.log"
    log_format = os.environ.get("LOG_FORMAT", "text")  # "text" or "json"

    root = logging.getLogger()
    root.setLevel(getattr(logging, log_level, logging.INFO))

    # Console handler
    console = logging.StreamHandler()
    if log_format == "json":
        console.setFormatter(JsonFormatter())
    else:
        console.setFormatter(logging.Formatter(
            "%(asctime)s [%(name)s] %(levelname)s: %(message)s",
            datefmt="%H:%M:%S",
        ))
    root.addHandler(console)

    # Optional rotating file handler
    if log_file:
        from logging.handlers import RotatingFileHandler
        file_handler = RotatingFileHandler(
            log_file, maxBytes=10_000_000, backupCount=5, encoding="utf-8"
        )
        file_handler.setFormatter(logging.Formatter(
            "%(asctime)s [%(name)s] %(levelname)s: %(message)s",
            datefmt="%Y-%m-%d %H:%M:%S",
        ))
        root.addHandler(file_handler)

    # Quieten noisy libraries
    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.getLogger("httpcore").setLevel(logging.WARNING)
    logging.getLogger("telegram").setLevel(logging.WARNING)
    logging.getLogger("aiohttp").setLevel(logging.WARNING)


# Logging
setup_logging()
logger = logging.getLogger("amanclaw.bot")


# --- Load Config ---

def load_config(path: str = "config.yaml") -> dict:
    config_path = Path(path)
    if not config_path.exists():
        logger.error(f"Config not found: {path}")
        logger.error("Copy config.example.yaml to config.yaml and fill in your values.")
        sys.exit(1)

    with open(config_path) as f:
        return yaml.safe_load(f)


# --- Globals (initialized in main) ---

config: dict = {}
auth: Auth = None
rate_limiter: RateLimiter = None
memory: Memory = None
llm: LLM = None
whatsapp: WhatsAppAdapter = None


# --- Helpers ---

async def send_typing_periodically(context, chat_id: int, stop_event: asyncio.Event):
    """Send typing indicator every 4 seconds until stop_event is set."""
    while not stop_event.is_set():
        try:
            await context.bot.send_chat_action(chat_id=chat_id, action=ChatAction.TYPING)
        except Exception:
            break
        try:
            await asyncio.wait_for(stop_event.wait(), timeout=4.0)
        except asyncio.TimeoutError:
            continue


async def reply_with_markdown(message, text: str):
    """Try to send with Markdown, fall back to plain text if parsing fails."""
    try:
        await message.reply_text(text, parse_mode=ParseMode.MARKDOWN)
    except Exception:
        # Markdown parsing failed — send as plain text
        await message.reply_text(text)


async def send_long_reply(message, response: str, with_actions: bool = False):
    """Send a response, splitting if too long for Telegram's 4096 char limit."""
    action_keyboard = None
    if with_actions and len(response) > 100:
        action_keyboard = InlineKeyboardMarkup([
            [
                InlineKeyboardButton("Simpler", callback_data="act_simpler"),
                InlineKeyboardButton("More detail", callback_data="act_detail"),
                InlineKeyboardButton("Translate BM", callback_data="act_translate_bm"),
            ]
        ])

    if len(response) <= 4096:
        try:
            await message.reply_text(response, parse_mode=ParseMode.MARKDOWN,
                                     reply_markup=action_keyboard)
        except Exception:
            await message.reply_text(response, reply_markup=action_keyboard)
    else:
        chunks = [response[i:i+4000] for i in range(0, len(response), 4000)]
        for i, chunk in enumerate(chunks):
            markup = action_keyboard if i == len(chunks) - 1 else None
            try:
                await message.reply_text(chunk, parse_mode=ParseMode.MARKDOWN,
                                         reply_markup=markup)
            except Exception:
                await message.reply_text(chunk, reply_markup=markup)


def auth_check(user_id: str) -> bool:
    return auth.is_authorized(user_id, "telegram")


async def handle_registration(update: Update, context: ContextTypes.DEFAULT_TYPE) -> bool:
    """Handle user registration flow. Returns True if user can proceed, False if blocked/pending."""
    user = update.effective_user
    user_id = str(user.id)
    state = auth.get_user_state(user_id, "telegram")

    if state == "admin" or state == "approved":
        return True

    if state == "blocked":
        return False

    if state == "pending":
        await update.message.reply_text(
            "Your registration is pending approval. "
            "An admin will review your request shortly."
        )
        return False

    # New user — register and notify admins
    memory.register_user(
        user_id=user_id,
        platform="telegram",
        username=user.username,
        first_name=user.first_name,
        last_name=user.last_name,
    )
    await update.message.reply_text(
        f"Welcome! You've been registered.\n\n"
        "An admin needs to approve your access before you can start chatting. "
        "Please wait for approval."
    )

    # Notify all admins
    admin_ids = config.get("admin_users", {}).get("telegram", [])
    name = user.first_name or user.username or user_id
    for admin_id in admin_ids:
        try:
            await context.bot.send_message(
                chat_id=int(admin_id),
                text=(
                    f"*New user registration:*\n\n"
                    f"Name: {name}\n"
                    f"Username: @{user.username or 'none'}\n"
                    f"User ID: `{user_id}`\n\n"
                    f"Use `/approve {user_id}` to approve or `/block {user_id}` to block."
                ),
                parse_mode=ParseMode.MARKDOWN,
            )
        except Exception as e:
            logger.error(f"Failed to notify admin {admin_id}: {e}")

    return False


async def build_context(user_id: str, message_text: str = "") -> tuple[list, dict, str, str]:
    """Build the smart context: history, facts, summary, knowledge context.
    Auto-summarize if needed."""
    history = memory.get_history(user_id)
    facts = memory.get_facts(user_id)  # backward compat
    summary = memory.get_latest_summary(user_id)

    # Build knowledge graph context
    knowledge_entries = memory.get_active_knowledge(user_id)
    entities = memory.get_entities(user_id)
    relationships = memory.get_relationships(user_id)

    # Also search for relevant knowledge based on message
    if message_text:
        relevant = memory.search_knowledge(user_id, message_text, limit=5)
        # Merge relevant results (deduplicate by ID)
        existing_ids = {k["id"] for k in knowledge_entries}
        for r in relevant:
            if r["id"] not in existing_ids:
                knowledge_entries.append(r)

    from amanclaw.llm import format_knowledge_context
    knowledge_context = format_knowledge_context(knowledge_entries, entities, relationships)

    # Auto-summarize when conversation gets long
    msg_count = memory.get_message_count(user_id)
    summarized_count = memory.get_summarized_message_count(user_id)
    unsummarized = msg_count - summarized_count
    if unsummarized > 40:
        old_msgs = memory.get_old_messages(user_id, before_last_n=20, limit=40)
        if old_msgs:
            new_summary = llm.summarize(old_msgs)
            if new_summary:
                if summary:
                    new_summary = f"{summary}\n\n{new_summary}"
                memory.save_summary(user_id, new_summary, len(old_msgs))
                summary = new_summary
                logger.info(f"Auto-summarized {len(old_msgs)} messages for user {user_id}")

    return history, facts, summary, knowledge_context


async def extract_and_save_knowledge(user_id: str, user_msg: str, assistant_reply: str):
    """Background task: extract knowledge from exchange and save to DB."""
    try:
        # Get existing knowledge for dedup context
        existing = memory.get_active_knowledge(user_id)
        existing_summary = "\n".join(
            f"- [{e['category']}] {e['subject']}: {e['content']}" for e in existing[:20]
        )

        extracted = await llm.extract_knowledge(user_msg, assistant_reply, existing_summary)
        if not extracted:
            return

        # Save knowledge entries
        for k in extracted.get("knowledge", []):
            memory.save_knowledge(
                user_id,
                category=k.get("category", "personal"),
                subject=k.get("subject", ""),
                content=k.get("content", ""),
                context=k.get("context"),
                valid_until=k.get("valid_until"),
                source="conversation",
            )

        # Save entities
        entity_name_to_id = {}
        for e in extracted.get("entities", []):
            eid = memory.save_entity(
                user_id,
                name=e.get("name", ""),
                entity_type=e.get("type", "person"),
                attributes=e.get("attributes", {}),
            )
            entity_name_to_id[e.get("name", "")] = eid

        # Save relationships
        for r in extracted.get("relationships", []):
            from_name = r.get("from", "")
            to_name = r.get("to", "")
            # Resolve entity IDs
            from_id = entity_name_to_id.get(from_name)
            to_id = entity_name_to_id.get(to_name)
            if not from_id:
                ent = memory.get_entity_by_name(user_id, from_name)
                from_id = ent["id"] if ent else None
            if not to_id:
                ent = memory.get_entity_by_name(user_id, to_name)
                to_id = ent["id"] if ent else None
            if from_id and to_id:
                memory.save_relationship(user_id, from_id, r.get("relation", "related_to"), to_id)

        # Apply updates to existing knowledge
        for u in extracted.get("updates", []):
            kid = u.get("id")
            if kid and u.get("content"):
                memory.update_knowledge(kid, content=u["content"])

        count = len(extracted.get("knowledge", [])) + len(extracted.get("entities", []))
        if count:
            logger.info(f"Extracted {count} knowledge items for user {user_id}")

    except Exception as e:
        logger.warning(f"Background knowledge extraction failed for {user_id}: {e}")


# --- Telegram Handlers ---

async def handle_message(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle all incoming text messages."""
    user = update.effective_user
    user_id = str(user.id)
    message_text = update.message.text

    if not await handle_registration(update, context):
        return

    if not rate_limiter.check(user_id):
        await update.message.reply_text("Slow down — too many messages. Try again in a minute.")
        return

    # Include quoted message context if user replied to a message
    reply = update.message.reply_to_message
    if reply and reply.text:
        quoted = reply.text[:500]
        message_text = f"[Replying to: \"{quoted}\"]\n\n{message_text}"

    clean_text, was_flagged = sanitize(message_text)
    if was_flagged:
        logger.warning(f"Flagged message from {user_id}: {message_text[:100]}")

    # Set context for skills
    set_current_user(user_id)
    set_reminder_context(user_id, str(update.effective_chat.id))
    set_scheduled_context(user_id, str(update.effective_chat.id))

    # Start typing indicator
    stop_typing = asyncio.Event()
    typing_task = asyncio.create_task(
        send_typing_periodically(context, update.effective_chat.id, stop_typing)
    )

    try:
        history, facts, summary, knowledge_context = await build_context(user_id, clean_text)
        response = await llm.respond(clean_text, history, flagged=was_flagged,
                                     facts=facts, summary=summary,
                                     knowledge_context=knowledge_context)
    except Exception as e:
        logger.error(f"LLM error: {e}")
        response = "Something went wrong talking to the AI. Try again in a moment."
    finally:
        stop_typing.set()
        await typing_task

    memory.save_exchange(user_id, "telegram", message_text, response)

    # Background knowledge extraction (non-blocking)
    asyncio.create_task(extract_and_save_knowledge(user_id, message_text, response))

    await send_long_reply(update.message, response, with_actions=True)


async def handle_photo(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle photo messages — send to vision model for analysis."""
    user = update.effective_user
    user_id = str(user.id)

    if not await handle_registration(update, context):
        return

    if not rate_limiter.check(user_id):
        await update.message.reply_text("Slow down — too many messages. Try again in a minute.")
        return

    # Get the highest resolution photo
    photo = update.message.photo[-1]
    caption = update.message.caption or None

    if caption:
        clean_caption, was_flagged = sanitize(caption)
    else:
        clean_caption, was_flagged = None, False

    set_current_user(user_id)
    set_reminder_context(user_id, str(update.effective_chat.id))
    set_scheduled_context(user_id, str(update.effective_chat.id))

    # Start typing indicator
    stop_typing = asyncio.Event()
    typing_task = asyncio.create_task(
        send_typing_periodically(context, update.effective_chat.id, stop_typing)
    )

    try:
        # Download the photo
        file = await context.bot.get_file(photo.file_id)
        image_bytes = await file.download_as_bytearray()

        # Build vision message
        vision_msg = llm.build_vision_message(bytes(image_bytes), clean_caption)

        history, facts, summary, knowledge_context = await build_context(user_id)
        response = await llm.respond(vision_msg, history, flagged=was_flagged,
                                     facts=facts, summary=summary,
                                     knowledge_context=knowledge_context)
    except Exception as e:
        logger.error(f"Vision error: {e}")
        response = "I couldn't process that image. Try again or send a text message instead."
    finally:
        stop_typing.set()
        await typing_task

    save_text = f"[Photo: {clean_caption or 'no caption'}]"
    memory.save_exchange(user_id, "telegram", save_text, response)

    asyncio.create_task(extract_and_save_knowledge(user_id, save_text, response))

    await send_long_reply(update.message, response, with_actions=True)


async def handle_voice(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle voice messages — acknowledge and ask for text."""
    user_id = str(update.effective_user.id)
    if not auth_check(user_id):
        return

    await update.message.reply_text(
        "I can't process voice messages yet. "
        "Please type your message instead, or send a photo for image analysis."
    )


# --- Command Handlers ---

async def cmd_start(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle /start command — triggers registration for new users."""
    user_id = str(update.effective_user.id)

    if not await handle_registration(update, context):
        return

    user = update.effective_user
    facts = memory.get_facts(user_id)
    name = facts.get("name", user.first_name or "there")

    keyboard = InlineKeyboardMarkup([
        [
            InlineKeyboardButton("Skills", callback_data="skills"),
            InlineKeyboardButton("Status", callback_data="status"),
        ],
        [
            InlineKeyboardButton("Clear History", callback_data="clear"),
            InlineKeyboardButton("Export Chat", callback_data="export"),
        ],
    ])

    await update.message.reply_text(
        f"Hey {name}! AmanClaw is ready.\n\n"
        "Just send me a message, photo, or use the buttons below.\n\n"
        "*Commands:*\n"
        "/skills — available skills\n"
        "/clear — clear conversation history\n"
        "/status — memory stats\n"
        "/export — export conversation\n"
        "/myid — your Telegram user ID",
        parse_mode=ParseMode.MARKDOWN,
        reply_markup=keyboard,
    )


async def cmd_skills(update: Update, context: ContextTypes.DEFAULT_TYPE):
    user_id = str(update.effective_user.id)
    if not auth_check(user_id):
        return
    await reply_with_markdown(update.message, f"*Available skills:*\n\n{get_skill_list()}")


async def cmd_clear(update: Update, context: ContextTypes.DEFAULT_TYPE):
    user_id = str(update.effective_user.id)
    if not auth_check(user_id):
        return

    keyboard = InlineKeyboardMarkup([
        [
            InlineKeyboardButton("Yes, clear it", callback_data="confirm_clear"),
            InlineKeyboardButton("Cancel", callback_data="cancel"),
        ]
    ])
    await update.message.reply_text(
        "Are you sure you want to clear your conversation history?",
        reply_markup=keyboard,
    )


async def cmd_status(update: Update, context: ContextTypes.DEFAULT_TYPE):
    user_id = str(update.effective_user.id)
    if not auth_check(user_id):
        return

    stats = memory.get_stats()
    facts = memory.get_facts(user_id)
    reminders = memory.get_user_reminders(user_id)

    text = (
        "*AmanClaw Status*\n\n"
        f"Messages: {stats['total_messages']}\n"
        f"Facts: {stats['total_facts']}\n"
        f"Summaries: {stats['total_summaries']}\n"
        f"Your facts: {len(facts)}\n"
        f"Pending reminders: {len(reminders)}\n"
        f"Unique users: {stats['unique_users']}"
    )
    await reply_with_markdown(update.message, text)


async def cmd_export(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Export conversation history as a text file."""
    user_id = str(update.effective_user.id)
    if not auth_check(user_id):
        return

    export_text = memory.export_history(user_id)
    if export_text == "No conversation history.":
        await update.message.reply_text("No conversation history to export.")
        return

    # Send as a document
    buf = io.BytesIO(export_text.encode("utf-8"))
    buf.name = f"amanclaw_chat_{user_id}_{datetime.now().strftime('%Y%m%d_%H%M')}.txt"
    await update.message.reply_document(
        document=buf,
        caption="Here's your conversation history.",
    )


async def cmd_myid(update: Update, context: ContextTypes.DEFAULT_TYPE):
    await update.message.reply_text(
        f"Your Telegram user ID: `{update.effective_user.id}`",
        parse_mode=ParseMode.MARKDOWN,
    )


async def cmd_approve(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Admin command: /approve <user_id>"""
    admin_id = str(update.effective_user.id)
    if not auth.is_admin(admin_id, "telegram"):
        return

    if not context.args:
        await update.message.reply_text("Usage: /approve <user_id>")
        return

    target_id = context.args[0]
    if memory.approve_user(target_id):
        await update.message.reply_text(f"User `{target_id}` approved.", parse_mode=ParseMode.MARKDOWN)
        # Send onboarding message to the user
        try:
            onboard_keyboard = InlineKeyboardMarkup([
                [
                    InlineKeyboardButton("Set my name", callback_data="onboard_name"),
                    InlineKeyboardButton("Set language", callback_data="onboard_lang"),
                ],
                [
                    InlineKeyboardButton("What can you do?", callback_data="skills"),
                ],
            ])
            await context.bot.send_message(
                chat_id=int(target_id),
                text=(
                    "Your access has been approved! Welcome to AmanClaw.\n\n"
                    "Let's get you set up. You can:\n"
                    "- Tell me your name so I remember you\n"
                    "- Choose your preferred language\n"
                    "- Or just start chatting right away!\n\n"
                    "Use the buttons below to get started."
                ),
                reply_markup=onboard_keyboard,
            )
        except Exception:
            pass
    else:
        user = memory.get_user(target_id)
        if not user:
            await update.message.reply_text(f"User `{target_id}` not found.", parse_mode=ParseMode.MARKDOWN)
        else:
            await update.message.reply_text(
                f"User `{target_id}` is already {user['status']}.", parse_mode=ParseMode.MARKDOWN
            )


async def cmd_block(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Admin command: /block <user_id>"""
    admin_id = str(update.effective_user.id)
    if not auth.is_admin(admin_id, "telegram"):
        return

    if not context.args:
        await update.message.reply_text("Usage: /block <user_id>")
        return

    target_id = context.args[0]
    if memory.block_user(target_id):
        await update.message.reply_text(f"User `{target_id}` blocked.", parse_mode=ParseMode.MARKDOWN)
    else:
        user = memory.get_user(target_id)
        if not user:
            await update.message.reply_text(f"User `{target_id}` not found.", parse_mode=ParseMode.MARKDOWN)
        else:
            await update.message.reply_text(
                f"User `{target_id}` is already {user['status']}.", parse_mode=ParseMode.MARKDOWN
            )


async def cmd_users(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Admin command: /users [pending|approved|blocked]"""
    admin_id = str(update.effective_user.id)
    if not auth.is_admin(admin_id, "telegram"):
        return

    status_filter = context.args[0] if context.args else None
    if status_filter and status_filter not in ("pending", "approved", "blocked"):
        await update.message.reply_text("Usage: /users [pending|approved|blocked]")
        return

    users = memory.list_users(status=status_filter)
    if not users:
        label = f" ({status_filter})" if status_filter else ""
        await update.message.reply_text(f"No users{label} found.")
        return

    lines = [f"*Users{(' - ' + status_filter) if status_filter else ''}:*\n"]
    for u in users:
        name = u["first_name"] or u["username"] or "Unknown"
        username = f"@{u['username']}" if u["username"] else "no username"
        lines.append(f"- `{u['user_id']}` {name} ({username}) [{u['status']}]")

    await reply_with_markdown(update.message, "\n".join(lines))


# --- Inline Keyboard Callback ---

async def handle_callback(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Handle inline keyboard button presses."""
    query = update.callback_query
    await query.answer()

    user_id = str(query.from_user.id)
    if not auth_check(user_id):
        return

    if query.data == "skills":
        await query.edit_message_text(
            f"*Available skills:*\n\n{get_skill_list()}",
            parse_mode=ParseMode.MARKDOWN,
        )
    elif query.data == "status":
        stats = memory.get_stats()
        facts = memory.get_facts(user_id)
        reminders = memory.get_user_reminders(user_id)
        await query.edit_message_text(
            f"*AmanClaw Status*\n\n"
            f"Messages: {stats['total_messages']}\n"
            f"Facts: {stats['total_facts']}\n"
            f"Your facts: {len(facts)}\n"
            f"Pending reminders: {len(reminders)}",
            parse_mode=ParseMode.MARKDOWN,
        )
    elif query.data == "confirm_clear":
        memory.clear_history(user_id)
        await query.edit_message_text("Conversation history cleared.")
    elif query.data == "cancel":
        await query.edit_message_text("Cancelled.")
    elif query.data == "export":
        export_text = memory.export_history(user_id)
        if export_text == "No conversation history.":
            await query.edit_message_text("No conversation history to export.")
        else:
            await query.edit_message_text("Exporting your chat history...")
            buf = io.BytesIO(export_text.encode("utf-8"))
            buf.name = f"amanclaw_chat_{user_id}_{datetime.now().strftime('%Y%m%d_%H%M')}.txt"
            await context.bot.send_document(
                chat_id=query.message.chat_id,
                document=buf,
                caption="Here's your conversation history.",
            )
    # --- Onboarding callbacks ---
    elif query.data == "onboard_name":
        await query.edit_message_text(
            "Just send me a message like:\n\n"
            "\"My name is [your name]\"\n\n"
            "I'll remember it for future conversations!"
        )
    elif query.data == "onboard_lang":
        lang_keyboard = InlineKeyboardMarkup([
            [
                InlineKeyboardButton("English", callback_data="setlang_en"),
                InlineKeyboardButton("Bahasa Melayu", callback_data="setlang_ms"),
            ],
            [
                InlineKeyboardButton("Auto-detect", callback_data="setlang_auto"),
            ],
        ])
        await query.edit_message_text(
            "Choose your preferred language:",
            reply_markup=lang_keyboard,
        )
    elif query.data.startswith("setlang_"):
        lang_code = query.data.replace("setlang_", "")
        lang_names = {"en": "English", "ms": "Bahasa Melayu", "auto": "Auto-detect"}
        lang_name = lang_names.get(lang_code, lang_code)
        memory.save_fact(user_id, "preferred_language", lang_name)
        await query.edit_message_text(f"Language set to *{lang_name}*. Let's start chatting!", parse_mode=ParseMode.MARKDOWN)
    # --- Response action callbacks ---
    elif query.data.startswith("act_"):
        original_text = query.message.text
        if not original_text:
            await query.answer("No text to work with.")
            return

        action = query.data.replace("act_", "")
        prompts = {
            "simpler": f"Explain this more simply and briefly:\n\n{original_text}",
            "detail": f"Expand on this with more detail and examples:\n\n{original_text}",
            "translate_bm": f"Translate this to Bahasa Melayu:\n\n{original_text}",
        }
        prompt = prompts.get(action)
        if not prompt:
            return

        await query.answer("Working on it...")
        await context.bot.send_chat_action(chat_id=query.message.chat_id, action=ChatAction.TYPING)

        try:
            response = await llm.respond(prompt, [], facts=memory.get_facts(user_id))
        except Exception:
            response = "Sorry, something went wrong. Try again."

        memory.save_exchange(user_id, "telegram", f"[{action}]", response)
        await context.bot.send_message(
            chat_id=query.message.chat_id,
            text=response,
            parse_mode=ParseMode.MARKDOWN,
        )


# --- Reminder Checker ---

async def check_reminders(context: ContextTypes.DEFAULT_TYPE):
    """Periodic job to check and deliver due reminders."""
    due = memory.get_due_reminders()
    for r in due:
        try:
            if r["platform"] == "whatsapp" and whatsapp:
                await whatsapp.deliver_reminder(r["chat_id"], r["message"])
            else:
                await context.bot.send_message(
                    chat_id=int(r["chat_id"]),
                    text=f"*Reminder:* {r['message']}",
                    parse_mode=ParseMode.MARKDOWN,
                )
            memory.mark_reminder_delivered(r["id"])
            logger.info(f"Delivered reminder #{r['id']} to {r['platform']} user {r['user_id']}")
        except Exception as e:
            logger.error(f"Failed to deliver reminder #{r['id']}: {e}")


# --- Schedule Checker ---

async def check_schedules(context: ContextTypes.DEFAULT_TYPE):
    """Periodic job to check and deliver due scheduled tasks."""
    due = memory.get_due_schedules()
    for s in due:
        try:
            if s["platform"] == "whatsapp" and whatsapp:
                await whatsapp.deliver_schedule(s["chat_id"], s["message"])
            else:
                await context.bot.send_message(
                    chat_id=int(s["chat_id"]),
                    text=f"*Scheduled:* {s['message']}",
                    parse_mode=ParseMode.MARKDOWN,
                )
            memory.mark_schedule_run(s["id"])
        except Exception as e:
            logger.error(f"Failed to deliver schedule #{s['id']}: {e}")


# --- Bot Menu ---

async def post_init(application):
    """Set bot commands menu and start WhatsApp adapter after initialization."""
    # Start WhatsApp adapter if configured
    if whatsapp:
        try:
            await whatsapp.start()
        except Exception as e:
            logger.error(f"Failed to start WhatsApp adapter: {e}")

    commands = [
        BotCommand("start", "Welcome & quick actions"),
        BotCommand("skills", "List available skills"),
        BotCommand("status", "Memory & bot stats"),
        BotCommand("clear", "Clear conversation history"),
        BotCommand("export", "Export chat history"),
        BotCommand("myid", "Show your Telegram user ID"),
        BotCommand("approve", "Admin: approve a user"),
        BotCommand("block", "Admin: block a user"),
        BotCommand("users", "Admin: list users"),
    ]
    await application.bot.set_my_commands(commands)


async def post_shutdown(application):
    """Clean up resources on shutdown."""
    if whatsapp:
        await whatsapp.stop()
    if memory:
        memory.close()
    if llm:
        await llm.close()
    logger.info("AmanClaw shut down cleanly.")


# --- Error Handler ---

async def error_handler(update: Update, context: ContextTypes.DEFAULT_TYPE):
    """Log errors and notify admins."""
    logger.error(f"Update {update} caused error: {context.error}", exc_info=context.error)

    # Notify admins
    admin_ids = config.get("admin_users", {}).get("telegram", [])
    error_text = f"Bot error:\n{type(context.error).__name__}: {context.error}"
    if update and update.effective_user:
        error_text = f"User: {update.effective_user.id}\n{error_text}"

    for admin_id in admin_ids:
        try:
            await context.bot.send_message(
                chat_id=int(admin_id),
                text=f"*AmanClaw Error*\n\n`{error_text[:1000]}`",
                parse_mode=ParseMode.MARKDOWN,
            )
        except Exception:
            pass


# --- Pruning Job ---

async def prune_job(context: ContextTypes.DEFAULT_TYPE):
    """Daily cleanup of old messages, delivered reminders, and expired knowledge."""
    msgs = memory.prune_all_users(keep_last=200)
    reminders = memory.prune_delivered_reminders(older_than_days=30)
    expired = memory.expire_old_knowledge()
    if msgs or reminders or expired:
        logger.info(f"Pruned {msgs} old messages, {reminders} delivered reminders, {expired} expired knowledge")


# --- Main ---

def main():
    global config, auth, rate_limiter, memory, llm, whatsapp

    logger.info("Starting AmanClaw...")

    # Load config
    config = load_config()
    webhook_config = config.get("webhook")

    # Initialize components — env vars override config values
    db_path = os.environ.get("MEMORY_DB_PATH") or config.get("memory_db", "memory.db")
    memory = Memory(db_path)
    auth = Auth(config, memory=memory)
    rate_limiter = RateLimiter(config.get("rate_limit_per_minute", 20))
    llm = LLM(config.get("llm", {}))

    # Validate admin_users
    admin_users = config.get("admin_users", {})
    has_admins = any(ids for ids in admin_users.values() if ids)
    if not has_admins:
        logger.warning(
            "No admin users configured! No one can approve new users. "
            "Add your user ID to config.yaml under admin_users."
        )

    # Configure skills from config
    skills_config = config.get("skills", {})
    if skills_config.get("shell_allowed_commands"):
        configure_shell(allowed_commands=skills_config["shell_allowed_commands"])
    if skills_config.get("shell_working_dir"):
        configure_shell(working_dir=skills_config["shell_working_dir"])
    if skills_config.get("workspace_dir"):
        configure_files(workspace_dir=skills_config["workspace_dir"])
        configure_documents(workspace_dir=skills_config["workspace_dir"])
    configure_remember(memory=memory)
    configure_reminder(memory=memory)
    configure_scheduled(memory=memory)

    # --- WhatsApp (optional) ---
    wa_config = config.get("whatsapp", {})
    if wa_config.get("enabled"):
        whatsapp = WhatsAppAdapter(config, auth, rate_limiter, memory, llm)
        logger.info("WhatsApp adapter configured (will start with bot)")

    # Get Telegram token
    token = config.get("telegram", {}).get("bot_token") or os.environ.get("TELEGRAM_BOT_TOKEN")
    if not token:
        logger.error("Telegram bot token not found in config.yaml or TELEGRAM_BOT_TOKEN env var.")
        sys.exit(1)

    # Build Telegram bot
    app = ApplicationBuilder().token(token).post_init(post_init).post_shutdown(post_shutdown).build()

    # Register handlers
    app.add_handler(CommandHandler("start", cmd_start))
    app.add_handler(CommandHandler("skills", cmd_skills))
    app.add_handler(CommandHandler("clear", cmd_clear))
    app.add_handler(CommandHandler("status", cmd_status))
    app.add_handler(CommandHandler("export", cmd_export))
    app.add_handler(CommandHandler("myid", cmd_myid))
    app.add_handler(CommandHandler("approve", cmd_approve))
    app.add_handler(CommandHandler("block", cmd_block))
    app.add_handler(CommandHandler("users", cmd_users))
    app.add_handler(CallbackQueryHandler(handle_callback))
    app.add_handler(MessageHandler(filters.PHOTO, handle_photo))
    app.add_handler(MessageHandler(filters.VOICE | filters.AUDIO, handle_voice))
    app.add_handler(MessageHandler(filters.TEXT & ~filters.COMMAND, handle_message))

    # Error handler
    app.add_error_handler(error_handler)

    # Schedule reminder checker every 30 seconds
    app.job_queue.run_repeating(check_reminders, interval=30, first=5)

    # Schedule recurring task checker every 60 seconds
    app.job_queue.run_repeating(check_schedules, interval=60, first=15)

    # Schedule daily pruning at 3:00 AM
    app.job_queue.run_daily(prune_job, time=datetime_time(hour=3, minute=0))

    if webhook_config and webhook_config.get("enabled"):
        webhook_url = webhook_config["url"]
        listen = webhook_config.get("listen", "0.0.0.0")
        port = webhook_config.get("port", 8443)
        secret_token = os.environ.get("WEBHOOK_SECRET") or webhook_config.get("secret_token")

        logger.info(f"Starting webhook mode on {listen}:{port}")
        app.run_webhook(
            listen=listen,
            port=port,
            url_path=f"webhook/{token[:10]}",  # Use part of token as path for obscurity
            webhook_url=f"{webhook_url}/webhook/{token[:10]}",
            secret_token=secret_token,
            allowed_updates=Update.ALL_TYPES,
        )
    else:
        logger.info("Starting polling mode")
        app.run_polling(allowed_updates=Update.ALL_TYPES)

    # Cleanup after run_polling returns (shutdown)
    if memory:
        memory.close()


if __name__ == "__main__":
    main()
