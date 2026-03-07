# amanclaw/channels/telegram.py
"""Telegram adapter — full-featured Telegram bot with commands, inline keyboards,
photo/voice support, typing indicators, and user skill management."""

import io
import re
import json
import asyncio
import logging
from datetime import datetime

from telegram import Update, InlineKeyboardButton, InlineKeyboardMarkup
from telegram.ext import (
    CommandHandler,
    MessageHandler,
    CallbackQueryHandler,
    ContextTypes,
    filters,
)
from telegram.constants import ParseMode, ChatAction
from telegram.helpers import escape_markdown

from amanclaw.channels import ChannelAdapter, OutgoingMessage
from amanclaw.security import sanitize
from amanclaw.skills import get_skill_list, REGISTRY
from amanclaw.skills.remember import set_current_user
from amanclaw.skills.reminder import set_context as set_reminder_context
from amanclaw.skills.scheduled import set_context as set_scheduled_context
from amanclaw.skills.documents import set_learning_context as set_doc_learning_context

logger = logging.getLogger("amanclaw.channels.telegram")

ADDSKILL_LLM_PROMPT = """You are helping create an API skill integration.
Based on the user's description, generate a complete skill config as JSON.

Rules:
- Find a suitable FREE public API if the user doesn't provide a URL
- Use {param} placeholders in URLs for dynamic parameters
- Keep the name short, lowercase, with underscores
- If an API key is typically needed, set needs_api_key to true
- For well-known services, use known free APIs like:
  - Weather: wttr.in (https://wttr.in/{city}?format=j1)
  - Currency: open.er-api.com
  - IP info: ipapi.co
  - Jokes: official-joke-api.appspot.com
  - Time: worldtimeapi.org
  - Random facts: uselessfacts.jsph.pl

Return ONLY valid JSON (no markdown fences, no explanation):
{"name": "skill_name", "description": "what it does", "url_template": "https://...", "method": "GET", "parameters": {"param_name": {"type": "string", "description": "what this param is"}}, "needs_api_key": false, "headers": {}, "query_params": {}}"""


class TelegramAdapter(ChannelAdapter):
    """Full-featured Telegram bot adapter."""

    def __init__(self, config: dict, processor, memory, llm, learning=None):
        self.config = config
        self.processor = processor
        self.memory = memory
        self.llm = llm
        self.learning = learning
        self._addskill_state: dict[str, dict] = {}

    @property
    def platform(self) -> str:
        return "telegram"

    def auth_check(self, user_id: str) -> bool:
        return self.processor.auth.is_authorized(user_id, "telegram")

    # ------------------------------------------------------------------ #
    #  Helpers                                                            #
    # ------------------------------------------------------------------ #

    @staticmethod
    async def _send_typing_periodically(context, chat_id: int, stop_event: asyncio.Event):
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

    @staticmethod
    async def _reply_with_markdown(message, text: str):
        """Try to send with Markdown, fall back to plain text if parsing fails."""
        try:
            await message.reply_text(text, parse_mode=ParseMode.MARKDOWN)
        except Exception:
            await message.reply_text(text)

    @staticmethod
    def _split_long_text(text: str) -> list[str]:
        """Split text into chunks for Telegram's 4096 char limit."""
        if len(text) <= 4000:
            return [text]
        return [text[i:i+4000] for i in range(0, len(text), 4000)]

    async def _send_long_reply(self, message, response: str, with_actions: bool = False):
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
            chunks = self._split_long_text(response)
            for i, chunk in enumerate(chunks):
                markup = action_keyboard if i == len(chunks) - 1 else None
                try:
                    await message.reply_text(chunk, parse_mode=ParseMode.MARKDOWN,
                                             reply_markup=markup)
                except Exception:
                    await message.reply_text(chunk, reply_markup=markup)

    # ------------------------------------------------------------------ #
    #  Registration                                                       #
    # ------------------------------------------------------------------ #

    async def _handle_registration(self, update: Update, context: ContextTypes.DEFAULT_TYPE) -> bool:
        """Handle user registration flow. Returns True if user can proceed."""
        user = update.effective_user
        user_id = str(user.id)
        state = self.processor.auth.get_user_state(user_id, "telegram")

        if state == "admin" or state == "approved":
            return True

        if state == "blocked":
            await update.message.reply_text(
                "Sorry, your access has been denied. "
                "Contact the admin if you think this is a mistake."
            )
            return False

        if state == "pending":
            await update.message.reply_text(
                "Still waiting for admin approval. Hang tight — "
                "you'll get a message as soon as you're in!\n\n"
                "Send /start anytime to check your status."
            )
            return False

        # New user — register and notify admins
        self.memory.register_user(
            user_id=user_id,
            platform="telegram",
            username=user.username,
            first_name=user.first_name,
            last_name=user.last_name,
        )
        await update.message.reply_text(
            "Welcome to AmanClaw!\n\n"
            "I'm a smart AI assistant that can remember things about you, "
            "analyze photos, set reminders, and much more.\n\n"
            "Your registration has been sent to an admin for approval. "
            "You'll be notified as soon as you're approved — usually within minutes!\n\n"
            "Send /start anytime to check your status."
        )

        # Notify all admins with inline approve/block buttons
        admin_ids = self.config.get("admin_users", {}).get("telegram", [])
        name = escape_markdown(user.first_name or user.username or user_id)
        admin_keyboard = InlineKeyboardMarkup([
            [
                InlineKeyboardButton("Approve", callback_data=f"adm_approve_{user_id}"),
                InlineKeyboardButton("Block", callback_data=f"adm_block_{user_id}"),
            ]
        ])
        for admin_id in admin_ids:
            try:
                await context.bot.send_message(
                    chat_id=int(admin_id),
                    text=(
                        f"*New user registration:*\n\n"
                        f"Name: {name}\n"
                        f"Username: @{escape_markdown(user.username or 'none')}\n"
                        f"User ID: `{user_id}`"
                    ),
                    parse_mode=ParseMode.MARKDOWN,
                    reply_markup=admin_keyboard,
                )
            except Exception as e:
                logger.error(f"Failed to notify admin {admin_id}: {e}")

        return False

    # ------------------------------------------------------------------ #
    #  Message Handlers                                                   #
    # ------------------------------------------------------------------ #

    async def handle_message(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """Handle all incoming text messages."""
        user = update.effective_user
        user_id = str(user.id)
        message_text = update.message.text

        if not await self._handle_registration(update, context):
            return

        # Check if user is in /addskill flow
        if user_id in self._addskill_state:
            await self._handle_addskill_step(update, context, user_id, message_text)
            return

        if not self.processor.rate_limiter.check(user_id):
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
        set_doc_learning_context(user_id, self.learning)

        # Start typing indicator
        stop_typing = asyncio.Event()
        typing_task = asyncio.create_task(
            self._send_typing_periodically(context, update.effective_chat.id, stop_typing)
        )

        try:
            history, facts, summary, knowledge_context = await self.processor._build_context(user_id, clean_text)
            response = await self.llm.respond(clean_text, history, flagged=was_flagged,
                                              facts=facts, summary=summary,
                                              knowledge_context=knowledge_context,
                                              user_id=user_id)
        except Exception as e:
            logger.error(f"LLM error: {e}")
            response = "Something went wrong talking to the AI. Try again in a moment."
        finally:
            stop_typing.set()
            await typing_task

        self.memory.save_exchange(user_id, "telegram", message_text, response)

        # Mark user as onboarded after first successful interaction
        if not self.memory.get_facts(user_id).get("onboarded"):
            self.memory.save_fact(user_id, "onboarded", "true")

        # Background knowledge extraction (non-blocking)
        asyncio.create_task(self.processor._extract_knowledge(user_id, message_text, response))

        # Track skill failures in response
        if self.learning and ("failed:" in response.lower() or "error:" in response.lower()):
            self.learning.log_failure(user_id, "llm_response", {"message": clean_text[:200]}, response[:500])

        await self._send_long_reply(update.message, response, with_actions=True)

        # Smart failure detection — suggest /addskill if bot lacks capability
        _capability_fail_patterns = [
            "can't access", "cannot access", "don't have access",
            "no tool", "not available", "unable to fetch",
            "can't fetch", "cannot fetch", "don't have a tool",
            "no built-in", "don't have built-in",
            "tidak dapat", "tidak boleh", "tiada akses",
        ]
        response_lower = response.lower()
        if any(p in response_lower for p in _capability_fail_patterns):
            await update.message.reply_text(
                "Want me to learn how to do this? "
                "You can add an API integration with /addskill",
            )

    async def handle_photo(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """Handle photo messages — send to vision model for analysis."""
        user = update.effective_user
        user_id = str(user.id)

        if not await self._handle_registration(update, context):
            return

        if not self.processor.rate_limiter.check(user_id):
            await update.message.reply_text("Slow down — too many messages. Try again in a minute.")
            return

        photo = update.message.photo[-1]
        caption = update.message.caption or None

        if caption:
            clean_caption, was_flagged = sanitize(caption)
        else:
            clean_caption, was_flagged = None, False

        set_current_user(user_id)
        set_reminder_context(user_id, str(update.effective_chat.id))
        set_scheduled_context(user_id, str(update.effective_chat.id))
        set_doc_learning_context(user_id, self.learning)

        stop_typing = asyncio.Event()
        typing_task = asyncio.create_task(
            self._send_typing_periodically(context, update.effective_chat.id, stop_typing)
        )

        try:
            file = await context.bot.get_file(photo.file_id)
            image_bytes = await file.download_as_bytearray()

            from amanclaw.llm import build_vision_message
            vision_msg = build_vision_message(bytes(image_bytes), clean_caption)

            history = self.memory.get_history(user_id)
            facts = self.memory.get_facts(user_id)
            summary = self.memory.get_latest_summary(user_id)

            response = await self.llm.respond(
                vision_msg, history, flagged=was_flagged,
                facts=facts, summary=summary,
                user_id=user_id,
            )
        except Exception as e:
            logger.error(f"Vision error: {e}")
            response = "Sorry, I couldn't analyze that image. Try again."
        finally:
            stop_typing.set()
            await typing_task

        user_text = f"[Photo]{f': {caption}' if caption else ''}"
        self.memory.save_exchange(user_id, "telegram", user_text, response)
        asyncio.create_task(self.processor._extract_knowledge(user_id, user_text, response))
        await self._send_long_reply(update.message, response)

    async def handle_voice(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """Handle voice messages — acknowledge and ask for text."""
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        await update.message.reply_text(
            "I can't process voice messages yet. "
            "Please type your message instead, or send a photo for image analysis."
        )

    # ------------------------------------------------------------------ #
    #  Command Handlers                                                   #
    # ------------------------------------------------------------------ #

    async def cmd_start(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not await self._handle_registration(update, context):
            return
        user = update.effective_user
        facts = self.memory.get_facts(user_id)
        is_onboarded = facts.get("onboarded") == "true"
        name = escape_markdown(facts.get("name", user.first_name or "there"))
        if is_onboarded:
            keyboard = InlineKeyboardMarkup([
                [
                    InlineKeyboardButton("Clear History", callback_data="clear"),
                    InlineKeyboardButton("Export Chat", callback_data="export"),
                ],
            ])
            await update.message.reply_text(
                f"Hey {name}! AmanClaw is ready.\n\n"
                "Just send me a message, photo, or voice note.",
                parse_mode=ParseMode.MARKDOWN,
                reply_markup=keyboard,
            )
        else:
            await self._send_approval_welcome(context, user_id)

    async def cmd_skills(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        await self._reply_with_markdown(update.message, f"*Available skills:*\n\n{get_skill_list()}")

    async def cmd_clear(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
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

    async def cmd_status(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        stats = self.memory.get_stats()
        facts = self.memory.get_facts(user_id)
        reminders = self.memory.get_user_reminders(user_id)
        text = (
            "*AmanClaw Status*\n\n"
            f"Messages: {stats['total_messages']}\n"
            f"Facts: {stats['total_facts']}\n"
            f"Summaries: {stats['total_summaries']}\n"
            f"Your facts: {len(facts)}\n"
            f"Pending reminders: {len(reminders)}\n"
            f"Unique users: {stats['unique_users']}"
        )
        await self._reply_with_markdown(update.message, text)

    async def cmd_export(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        export_text = self.memory.export_history(user_id)
        if export_text == "No conversation history.":
            await update.message.reply_text("No conversation history to export.")
            return
        buf = io.BytesIO(export_text.encode("utf-8"))
        buf.name = f"amanclaw_chat_{user_id}_{datetime.now().strftime('%Y%m%d_%H%M')}.txt"
        await update.message.reply_document(
            document=buf,
            caption="Here's your conversation history.",
        )

    async def cmd_myid(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        await update.message.reply_text(
            f"Your Telegram user ID: `{update.effective_user.id}`",
            parse_mode=ParseMode.MARKDOWN,
        )

    async def cmd_approve(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        admin_id = str(update.effective_user.id)
        if not self.processor.auth.is_admin(admin_id, "telegram"):
            return
        if not context.args:
            await update.message.reply_text("Usage: /approve <user_id>")
            return
        target_id = context.args[0]
        if self.memory.approve_user(target_id):
            await update.message.reply_text(f"User `{target_id}` approved.", parse_mode=ParseMode.MARKDOWN)
            await self._send_approval_welcome(context, target_id)
        else:
            user = self.memory.get_user(target_id)
            if not user:
                await update.message.reply_text(f"User `{target_id}` not found.", parse_mode=ParseMode.MARKDOWN)
            else:
                await update.message.reply_text(
                    f"User `{target_id}` is already {user['status']}.", parse_mode=ParseMode.MARKDOWN
                )

    async def cmd_block(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        admin_id = str(update.effective_user.id)
        if not self.processor.auth.is_admin(admin_id, "telegram"):
            return
        if not context.args:
            await update.message.reply_text("Usage: /block <user_id>")
            return
        target_id = context.args[0]
        if self.memory.block_user(target_id):
            await update.message.reply_text(f"User `{target_id}` blocked.", parse_mode=ParseMode.MARKDOWN)
        else:
            user = self.memory.get_user(target_id)
            if not user:
                await update.message.reply_text(f"User `{target_id}` not found.", parse_mode=ParseMode.MARKDOWN)
            else:
                await update.message.reply_text(
                    f"User `{target_id}` is already {user['status']}.", parse_mode=ParseMode.MARKDOWN
                )

    async def cmd_users(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        admin_id = str(update.effective_user.id)
        if not self.processor.auth.is_admin(admin_id, "telegram"):
            return
        status_filter = context.args[0] if context.args else None
        if status_filter and status_filter not in ("pending", "approved", "blocked"):
            await update.message.reply_text("Usage: /users [pending|approved|blocked]")
            return
        users = self.memory.list_users(status=status_filter)
        if not users:
            label = f" ({status_filter})" if status_filter else ""
            await update.message.reply_text(f"No users{label} found.")
            return
        lines = [f"*Users{(' - ' + status_filter) if status_filter else ''}:*\n"]
        for u in users:
            name = escape_markdown(u["first_name"] or u["username"] or "Unknown")
            username = f"@{escape_markdown(u['username'])}" if u["username"] else "no username"
            lines.append(f"- `{u['user_id']}` {name} ({username}) [{u['status']}]")
        await self._reply_with_markdown(update.message, "\n".join(lines))

    async def cmd_teach(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        if not context.args:
            await self._reply_with_markdown(update.message,
                "*Teaching mode*\n\n"
                "Teach me rules like:\n"
                "`/teach when I say deploy, push to staging first`\n"
                "`/teach always keep answers short about food`\n"
                "`/teach if I ask about servers, check status first`\n\n"
                "Or just tell me naturally in conversation:\n"
                "\"Remember that when I say X, I mean Y\""
            )
            return
        rule = " ".join(context.args)
        set_current_user(user_id)
        from amanclaw.skills.remember import teach
        result = teach(rule=rule)
        await self._reply_with_markdown(update.message, result)

    async def cmd_learned(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        days = int(context.args[0]) if context.args else 7
        if self.learning:
            journal = self.learning.get_learning_journal(user_id, days=days)
        else:
            journal = "Learning engine not initialized."
        await self._send_long_reply(update.message, journal)

    async def cmd_forget(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not self.auth_check(user_id):
            return
        if not context.args:
            await update.message.reply_text("Usage: /forget <topic>\nExample: /forget coffee preference")
            return
        query = " ".join(context.args)
        set_current_user(user_id)
        from amanclaw.skills.remember import forget
        result = forget(query=query)
        await self._reply_with_markdown(update.message, result)

    async def cmd_myskills(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not await self._handle_registration(update, context):
            return
        skills = self.memory.get_user_skills(user_id)
        own = [s for s in skills if s["user_id"] == user_id]
        if not own:
            await update.message.reply_text(
                "You don't have any custom skills yet.\n"
                "Use /addskill to create one!"
            )
            return
        lines = ["Your Skills:\n"]
        for s in own:
            status = "private" if s["is_private"] else ("approved" if s["is_approved"] else "pending review")
            lines.append(f"- {s['name']}: {s['description']} [{status}]")
        await update.message.reply_text("\n".join(lines))

    async def cmd_delskill(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not await self._handle_registration(update, context):
            return
        if not context.args:
            await update.message.reply_text("Usage: /delskill <skill_name>")
            return
        name = context.args[0]
        if self.memory.delete_user_skill(user_id, name):
            await update.message.reply_text(f"Skill '{name}' deleted.")
        else:
            await update.message.reply_text(f"Skill '{name}' not found.")

    async def cmd_publish(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not await self._handle_registration(update, context):
            return
        if not context.args:
            await update.message.reply_text("Usage: /publish <skill_name>")
            return
        name = context.args[0]
        if self.memory.publish_user_skill(user_id, name):
            await update.message.reply_text(
                f"Skill '{name}' submitted for review!\n"
                "An admin will review it shortly."
            )
            admin_ids = self.config.get("admin_users", {}).get("telegram", [])
            skill = self.memory.get_user_skill_by_name(name, user_id)
            keyboard = InlineKeyboardMarkup([
                [
                    InlineKeyboardButton("Approve", callback_data=f"appskill_{name}_{user_id}"),
                    InlineKeyboardButton("Reject", callback_data=f"rejskill_{name}_{user_id}"),
                ]
            ])
            for admin_id in admin_ids:
                try:
                    await context.bot.send_message(
                        chat_id=int(admin_id),
                        text=(
                            f"Skill submitted for marketplace:\n\n"
                            f"Name: {name}\n"
                            f"By: {user_id}\n"
                            f"Description: {skill['description']}\n"
                            f"URL: {skill['url_template']}\n"
                            f"Method: {skill['method']}"
                        ),
                        reply_markup=keyboard,
                    )
                except Exception:
                    pass
        else:
            await update.message.reply_text(f"Skill '{name}' not found or already published.")

    async def cmd_marketplace(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not await self._handle_registration(update, context):
            return
        skills = self.memory.get_marketplace_skills()
        if not skills:
            await update.message.reply_text("No community skills available yet. Be the first — use /addskill!")
            return
        lines = ["Community Marketplace:\n"]
        for s in skills:
            lines.append(f"- {s['name']}: {s['description']}")
        lines.append("\nAll marketplace skills are automatically available to you!")
        await update.message.reply_text("\n".join(lines))

    # ------------------------------------------------------------------ #
    #  Approval Welcome                                                   #
    # ------------------------------------------------------------------ #

    async def _send_approval_welcome(self, context: ContextTypes.DEFAULT_TYPE, user_id: str):
        welcome_keyboard = InlineKeyboardMarkup([
            [
                InlineKeyboardButton("Tell me your name", callback_data="try_name"),
                InlineKeyboardButton("Analyze a photo", callback_data="try_photo"),
            ],
            [
                InlineKeyboardButton("Set a reminder", callback_data="try_reminder"),
                InlineKeyboardButton("How do I teach you?", callback_data="try_teach"),
            ],
            [
                InlineKeyboardButton("Set language", callback_data="onboard_lang"),
            ],
        ])
        try:
            await context.bot.send_message(
                chat_id=int(user_id),
                text=(
                    "You're approved! Welcome to AmanClaw.\n\n"
                    "I'm your personal AI assistant. Here's what I can do:\n\n"
                    "I remember things about you across conversations\n"
                    "Send me a photo and I'll analyze it\n"
                    "I can set reminders for you\n"
                    "Teach me custom rules and I'll follow them\n\n"
                    "Try one of these to get started, or just say hi!"
                ),
                reply_markup=welcome_keyboard,
            )
        except Exception as e:
            logger.error(f"Failed to send welcome to {user_id}: {e}")

    # ------------------------------------------------------------------ #
    #  /addskill Conversational Flow                                     #
    # ------------------------------------------------------------------ #

    async def cmd_addskill(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        user_id = str(update.effective_user.id)
        if not await self._handle_registration(update, context):
            return
        args_text = " ".join(context.args) if context.args else ""
        if args_text.strip():
            self._addskill_state[user_id] = {"step": "generating"}
            await update.message.reply_text("Generating skill config...")
            await self._generate_skill_from_description(update, user_id, args_text.strip())
            return
        self._addskill_state[user_id] = {"step": "describe"}
        await update.message.reply_text(
            "Let's create a new skill!\n\n"
            "Just describe what you want:\n"
            "\u2022 \"Get weather for any city\"\n"
            "\u2022 \"Convert currencies\"\n"
            "\u2022 \"Get a random joke\"\n"
            "\u2022 \"Shorten URLs\"\n\n"
            "I'll find a free API and set it up for you automatically.\n\n"
            "Or do it inline: `/addskill get weather for a city`\n\n"
            "Send /cancel to stop.",
            parse_mode=ParseMode.MARKDOWN,
        )

    async def _generate_skill_from_description(self, update, user_id: str, description: str):
        try:
            result = await self.llm._call_api([
                {"role": "system", "content": ADDSKILL_LLM_PROMPT},
                {"role": "user", "content": description},
            ])
            raw = result["choices"][0]["message"]["content"]
            raw = re.sub(r'^```(?:json)?\s*', '', raw.strip())
            raw = re.sub(r'\s*```$', '', raw.strip())
            raw = re.sub(r'<think>.*?</think>', '', raw, flags=re.DOTALL).strip()
            skill_config = json.loads(raw)
        except (json.JSONDecodeError, KeyError, IndexError) as e:
            logger.warning(f"LLM skill generation failed: {e}")
            self._addskill_state.pop(user_id, None)
            msg = update.message if hasattr(update, 'message') and update.message else update
            await msg.reply_text(
                "Couldn't auto-generate the skill. Try being more specific.\n"
                "Example: `/addskill get weather forecast for a city using wttr.in`",
                parse_mode=ParseMode.MARKDOWN,
            )
            return
        except Exception as e:
            logger.error(f"LLM skill generation error: {e}")
            self._addskill_state.pop(user_id, None)
            msg = update.message if hasattr(update, 'message') and update.message else update
            await msg.reply_text(f"Error generating skill: {e}")
            return

        name = skill_config.get("name", "").lower().replace(" ", "_").replace("-", "_")
        name = re.sub(r'[^a-z0-9_]', '', name)
        if not name or len(name) < 2:
            name = re.sub(r'[^a-z0-9_]', '', description.lower().split()[0])[:20] or "custom"
        name = name[:30]
        if name in REGISTRY:
            name = f"my_{name}"

        state = {
            "step": "confirm",
            "name": name,
            "description": skill_config.get("description", description),
            "url_template": skill_config.get("url_template", ""),
            "method": skill_config.get("method", "GET"),
            "parameters": skill_config.get("parameters", {}),
            "headers": skill_config.get("headers", {}),
            "query_params": skill_config.get("query_params", {}),
            "needs_api_key": skill_config.get("needs_api_key", False),
            "api_key": None,
        }
        self._addskill_state[user_id] = state
        await self._show_addskill_confirmation(update, state)

    async def _handle_addskill_step(self, update: Update, context: ContextTypes.DEFAULT_TYPE,
                                     user_id: str, text: str):
        if text.strip().lower() == "/cancel":
            del self._addskill_state[user_id]
            await update.message.reply_text("Skill creation cancelled.")
            return
        state = self._addskill_state[user_id]
        step = state["step"]
        if step == "describe":
            state["step"] = "generating"
            await update.message.reply_text("Generating skill config...")
            await self._generate_skill_from_description(update, user_id, text.strip())
        elif step == "apikey_input":
            state["api_key"] = text.strip()
            state["step"] = "confirm"
            await self._show_addskill_confirmation(update, state)
        elif step == "edit":
            await update.message.reply_text("Regenerating with your feedback...")
            original_desc = state.get("description", "")
            await self._generate_skill_from_description(
                update, user_id, f"{original_desc}. {text.strip()}"
            )

    async def _show_addskill_confirmation(self, update_or_query, state: dict):
        params_list = ", ".join(state.get("parameters", {}).keys()) or "none"
        needs_key = state.get("needs_api_key", False)
        has_key = bool(state.get("api_key"))
        if needs_key and not has_key:
            api_key_status = "required (not set yet)"
        elif has_key:
            api_key_status = "set"
        else:
            api_key_status = "not needed"
        summary = (
            f"*Skill Preview:*\n\n"
            f"*Name:* `{state['name']}`\n"
            f"*Description:* {state['description']}\n"
            f"*URL:* `{state['url_template']}`\n"
            f"*Method:* {state.get('method', 'GET')}\n"
            f"*Parameters:* {params_list}\n"
            f"*API Key:* {api_key_status}"
        )
        buttons = []
        if needs_key and not has_key:
            buttons.append([
                InlineKeyboardButton("Set API Key", callback_data="addskill_haskey"),
                InlineKeyboardButton("Skip (no key)", callback_data="addskill_nokey"),
            ])
        buttons.append([
            InlineKeyboardButton("Create", callback_data="addskill_confirm"),
            InlineKeyboardButton("Edit", callback_data="addskill_edit"),
            InlineKeyboardButton("Cancel", callback_data="addskill_cancel"),
        ])
        keyboard = InlineKeyboardMarkup(buttons)
        msg = update_or_query.message if hasattr(update_or_query, 'message') and update_or_query.message else update_or_query
        await msg.reply_text(summary, reply_markup=keyboard, parse_mode=ParseMode.MARKDOWN)

    # ------------------------------------------------------------------ #
    #  Callback Handler                                                   #
    # ------------------------------------------------------------------ #

    async def handle_callback(self, update: Update, context: ContextTypes.DEFAULT_TYPE):
        """Handle inline keyboard button presses."""
        query = update.callback_query
        await query.answer()
        user_id = str(query.from_user.id)

        # --- Admin approval callbacks ---
        if query.data.startswith("adm_approve_"):
            if not self.processor.auth.is_admin(user_id, "telegram"):
                await query.answer("Not authorized.", show_alert=True)
                return
            target_id = query.data.replace("adm_approve_", "")
            if self.memory.approve_user(target_id):
                await query.edit_message_text(
                    query.message.text + "\n\n✅ Approved",
                )
                await self._send_approval_welcome(context, target_id)
            else:
                await query.answer("User already processed.", show_alert=True)
            return

        if query.data.startswith("adm_block_"):
            if not self.processor.auth.is_admin(user_id, "telegram"):
                await query.answer("Not authorized.", show_alert=True)
                return
            target_id = query.data.replace("adm_block_", "")
            if self.memory.block_user(target_id):
                await query.edit_message_text(
                    query.message.text + "\n\n🚫 Blocked",
                )
            else:
                await query.answer("User already processed.", show_alert=True)
            return

        # --- Try-me onboarding callbacks ---
        if query.data == "try_name":
            await query.edit_message_text(
                "Just send me a message telling me your name!\n\n"
                "For example: \"My name is Sarah\" or \"Call me Alex\"\n\n"
                "I'll remember it for all our future conversations."
            )
            return
        if query.data == "try_photo":
            await query.edit_message_text(
                "Send me any photo and I'll analyze it!\n\n"
                "I can describe what's in it, read text from images, "
                "identify objects, and answer questions about it.\n\n"
                "Try it now — just send a photo from your gallery."
            )
            return
        if query.data == "try_reminder":
            await query.edit_message_text(
                "Just ask me to remind you of something!\n\n"
                "For example:\n"
                "\"Remind me to call the dentist in 2 hours\"\n"
                "\"Remind me about the meeting at 3pm tomorrow\"\n\n"
                "I'll send you a message when it's time."
            )
            return
        if query.data == "try_teach":
            await query.edit_message_text(
                "You can teach me custom rules!\n\n"
                "Use /teach followed by your rule. For example:\n"
                "/teach Always reply in bullet points\n"
                "/teach When I say 'brief', keep answers under 2 sentences\n\n"
                "Use /learned to see what I've learned from you."
            )
            return

        # --- Addskill flow callbacks ---
        if query.data == "addskill_edit":
            if user_id in self._addskill_state:
                self._addskill_state[user_id]["step"] = "edit"
                await query.edit_message_text(
                    "What would you like to change? Just tell me:\n\n"
                    "Examples:\n"
                    "\u2022 \"Use a different API\"\n"
                    "\u2022 \"Change the name to my_weather\"\n"
                    "\u2022 \"Add a language parameter\"\n"
                    "\u2022 \"Use POST instead of GET\"\n\n"
                    "Or describe the whole skill differently."
                )
            return
        if query.data == "addskill_nokey":
            if user_id in self._addskill_state:
                self._addskill_state[user_id]["api_key"] = None
                self._addskill_state[user_id]["step"] = "confirm"
                await query.edit_message_text("No API key needed.")
                await self._show_addskill_confirmation(query, self._addskill_state[user_id])
            return
        if query.data == "addskill_haskey":
            if user_id in self._addskill_state:
                self._addskill_state[user_id]["step"] = "apikey_input"
                await query.edit_message_text(
                    "Send me the API key. I'll store it securely and never show it again."
                )
            return
        if query.data == "addskill_confirm":
            if user_id in self._addskill_state:
                state = self._addskill_state.pop(user_id)
                skill_data = {
                    "name": state["name"],
                    "description": state["description"],
                    "url_template": state["url_template"],
                    "method": state.get("method", "GET"),
                    "parameters": state.get("parameters", {}),
                    "api_key_encrypted": state.get("api_key"),
                    "is_private": True,
                }
                self.memory.save_user_skill(user_id, skill_data)
                await query.edit_message_text(
                    f"Skill '{state['name']}' created!\n\n"
                    f"Try it now — just ask me something that uses it.\n\n"
                    f"Commands:\n"
                    f"/myskills — view your skills\n"
                    f"/publish {state['name']} — submit to community marketplace\n"
                    f"/delskill {state['name']} — delete this skill"
                )
            return
        if query.data == "addskill_cancel":
            if user_id in self._addskill_state:
                del self._addskill_state[user_id]
            await query.edit_message_text("Skill creation cancelled.")
            return

        # --- Skill marketplace admin callbacks ---
        if query.data.startswith("appskill_"):
            if not self.processor.auth.is_admin(user_id, "telegram"):
                await query.answer("Not authorized.", show_alert=True)
                return
            parts = query.data.replace("appskill_", "").rsplit("_", 1)
            skill_name, creator_id = parts[0], parts[1]
            skill = self.memory.get_user_skill_by_name(skill_name, creator_id)
            if skill and self.memory.approve_user_skill(skill["id"]):
                await query.edit_message_text(
                    query.message.text + "\n\nApproved for marketplace"
                )
            return
        if query.data.startswith("rejskill_"):
            if not self.processor.auth.is_admin(user_id, "telegram"):
                await query.answer("Not authorized.", show_alert=True)
                return
            parts = query.data.replace("rejskill_", "").rsplit("_", 1)
            skill_name, creator_id = parts[0], parts[1]
            self.memory.delete_user_skill(creator_id, skill_name)
            await query.edit_message_text(
                query.message.text + "\n\nRejected and removed"
            )
            return

        if not self.auth_check(user_id):
            return

        if query.data == "skills":
            await query.edit_message_text(
                f"*Available skills:*\n\n{get_skill_list()}",
                parse_mode=ParseMode.MARKDOWN,
            )
        elif query.data == "status":
            stats = self.memory.get_stats()
            facts = self.memory.get_facts(user_id)
            reminders = self.memory.get_user_reminders(user_id)
            await query.edit_message_text(
                f"*AmanClaw Status*\n\n"
                f"Messages: {stats['total_messages']}\n"
                f"Facts: {stats['total_facts']}\n"
                f"Your facts: {len(facts)}\n"
                f"Pending reminders: {len(reminders)}",
                parse_mode=ParseMode.MARKDOWN,
            )
        elif query.data == "confirm_clear":
            self.memory.clear_history(user_id)
            await query.edit_message_text("Conversation history cleared.")
        elif query.data == "cancel":
            await query.edit_message_text("Cancelled.")
        elif query.data == "export":
            export_text = self.memory.export_history(user_id)
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
            self.memory.save_fact(user_id, "preferred_language", lang_name)
            await query.edit_message_text(f"Language set to *{lang_name}*. Let's start chatting!", parse_mode=ParseMode.MARKDOWN)
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
                response = await self.llm.respond(prompt, [], facts=self.memory.get_facts(user_id), user_id=user_id)
            except Exception:
                response = "Sorry, something went wrong. Try again."
            self.memory.save_exchange(user_id, "telegram", f"[{action}]", response)
            await context.bot.send_message(
                chat_id=query.message.chat_id,
                text=response,
                parse_mode=ParseMode.MARKDOWN,
            )

    # ------------------------------------------------------------------ #
    #  ChannelAdapter interface + handler registration                    #
    # ------------------------------------------------------------------ #

    def register_handlers(self, application):
        """Register all Telegram handlers on the Application."""
        application.add_handler(CommandHandler("start", self.cmd_start))
        application.add_handler(CommandHandler("skills", self.cmd_skills))
        application.add_handler(CommandHandler("clear", self.cmd_clear))
        application.add_handler(CommandHandler("status", self.cmd_status))
        application.add_handler(CommandHandler("export", self.cmd_export))
        application.add_handler(CommandHandler("myid", self.cmd_myid))
        application.add_handler(CommandHandler("approve", self.cmd_approve))
        application.add_handler(CommandHandler("block", self.cmd_block))
        application.add_handler(CommandHandler("users", self.cmd_users))
        application.add_handler(CommandHandler("teach", self.cmd_teach))
        application.add_handler(CommandHandler("learned", self.cmd_learned))
        application.add_handler(CommandHandler("forget", self.cmd_forget))
        application.add_handler(CommandHandler("addskill", self.cmd_addskill))
        application.add_handler(CommandHandler("myskills", self.cmd_myskills))
        application.add_handler(CommandHandler("delskill", self.cmd_delskill))
        application.add_handler(CommandHandler("publish", self.cmd_publish))
        application.add_handler(CommandHandler("marketplace", self.cmd_marketplace))
        application.add_handler(CallbackQueryHandler(self.handle_callback))
        application.add_handler(MessageHandler(filters.PHOTO, self.handle_photo))
        application.add_handler(MessageHandler(filters.VOICE | filters.AUDIO, self.handle_voice))
        application.add_handler(MessageHandler(filters.TEXT & ~filters.COMMAND, self.handle_message))

    async def start(self) -> None:
        """Telegram is started by bot.py via run_polling/run_webhook — this is a no-op."""
        pass

    async def stop(self) -> None:
        """Telegram shutdown is handled by bot.py — this is a no-op."""
        pass

    async def send_message(self, msg: OutgoingMessage) -> None:
        """Send a message via the ChannelAdapter ABC contract.
        For Telegram, most sending is done through update.message.reply_text in handlers."""
        pass
