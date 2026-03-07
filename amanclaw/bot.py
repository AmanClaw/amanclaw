"""
AmanClaw — Main bot entry point (orchestrator).

Initializes all components, creates adapters, manages lifecycle.
Telegram-specific handlers live in channels/telegram.py.
"""

import os
import sys
import yaml
import asyncio
import logging
from pathlib import Path
from dotenv import load_dotenv

load_dotenv()
from datetime import datetime, time as datetime_time
from telegram import BotCommand
from telegram.ext import ApplicationBuilder, ContextTypes
from telegram.constants import ParseMode

from amanclaw.security import Auth, RateLimiter
from amanclaw.memory import Memory
from amanclaw.llm import LLM
from amanclaw.skills.shell import configure as configure_shell
from amanclaw.skills.files import configure as configure_files
from amanclaw.skills.remember import configure as configure_remember
from amanclaw.skills.reminder import configure as configure_reminder
from amanclaw.skills.scheduled import configure as configure_scheduled
from amanclaw.skills.documents import configure as configure_documents
from amanclaw.learning import LearningEngine
from amanclaw.mcp_client import MCPManager
from amanclaw.skills import set_mcp_manager, set_user_skill_manager
from amanclaw.skills.user_skills import UserSkillManager
from amanclaw.processor import MessageProcessor
from amanclaw.channels.telegram import TelegramAdapter


class JsonFormatter(logging.Formatter):
    """JSON log formatter for structured logging (Docker, log aggregators)."""
    def format(self, record):
        import json
        log_data = {
            "ts": datetime.now().isoformat(),
            "level": record.levelname,
            "logger": record.name,
            "msg": record.getMessage(),
        }
        if record.exc_info and record.exc_info[0]:
            log_data["exception"] = self.formatException(record.exc_info)
        return json.dumps(log_data)


def setup_logging():
    """Configure logging with console + optional rotating file output."""
    log_level = os.environ.get("LOG_LEVEL", "INFO").upper()
    log_file = os.environ.get("LOG_FILE")
    log_format = os.environ.get("LOG_FORMAT", "text")

    root = logging.getLogger()
    root.setLevel(getattr(logging, log_level, logging.INFO))

    console = logging.StreamHandler()
    if log_format == "json":
        console.setFormatter(JsonFormatter())
    else:
        console.setFormatter(logging.Formatter(
            "%(asctime)s [%(name)s] %(levelname)s: %(message)s",
            datefmt="%H:%M:%S",
        ))
    root.addHandler(console)

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

    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.getLogger("httpcore").setLevel(logging.WARNING)
    logging.getLogger("telegram").setLevel(logging.WARNING)
    logging.getLogger("aiohttp").setLevel(logging.WARNING)


setup_logging()
logger = logging.getLogger("amanclaw.bot")


def load_config(path: str = "config.yaml") -> dict:
    config_path = Path(os.environ.get("CONFIG_PATH", path))
    if not config_path.exists():
        logger.error(f"Config not found: {config_path}")
        logger.error("Copy config.example.yaml to config.yaml and fill in your values.")
        sys.exit(1)
    with open(config_path) as f:
        return yaml.safe_load(f)


# --- Globals ---
config: dict = {}
memory: Memory = None
llm: LLM = None
whatsapp = None
learning_engine: LearningEngine = None
mcp_manager = None
processor: MessageProcessor = None
telegram_adapter: TelegramAdapter = None
discord_adapter = None
slack_adapter = None


# --- Jobs ---

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


async def prune_job(context: ContextTypes.DEFAULT_TYPE):
    """Daily cleanup of old messages, delivered reminders, and expired knowledge."""
    msgs = memory.prune_all_users(keep_last=200)
    reminders = memory.prune_delivered_reminders(older_than_days=30)
    expired = memory.expire_old_knowledge()
    if msgs or reminders or expired:
        logger.info(f"Pruned {msgs} old messages, {reminders} delivered reminders, {expired} expired knowledge")


async def checkin_job(context: ContextTypes.DEFAULT_TYPE):
    """Weekly job to send proactive check-in messages."""
    if not learning_engine:
        return
    users = memory.list_users(status="approved")
    admin_ids = [str(uid) for uid in config.get("admin_users", {}).get("telegram", [])]
    all_user_ids = set(u["user_id"] for u in users) | set(admin_ids)
    for user_id in all_user_ids:
        candidates = learning_engine.get_checkin_candidates(user_id, min_age_days=14)
        if not candidates:
            continue
        msg = learning_engine.format_checkin_message(candidates)
        if not msg:
            continue
        try:
            await context.bot.send_message(
                chat_id=int(user_id),
                text=msg,
                parse_mode=ParseMode.MARKDOWN,
            )
            logger.info(f"Sent proactive check-in to user {user_id}")
        except Exception as e:
            logger.debug(f"Failed to send check-in to {user_id}: {e}")


async def error_handler(update, context: ContextTypes.DEFAULT_TYPE):
    """Log errors and notify admins."""
    logger.error(f"Update {update} caused error: {context.error}", exc_info=context.error)
    admin_ids = config.get("admin_users", {}).get("telegram", [])
    error_text = f"Bot error:\n{type(context.error).__name__}: {context.error}"
    if update and hasattr(update, 'effective_user') and update.effective_user:
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


# --- Lifecycle ---

async def post_init(application):
    """Set bot commands menu and start adapters after initialization."""
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
        BotCommand("teach", "Teach me a rule or behavior"),
        BotCommand("learned", "Show what I've learned"),
        BotCommand("forget", "Forget specific knowledge"),
        BotCommand("approve", "Admin: approve a user"),
        BotCommand("block", "Admin: block a user"),
        BotCommand("users", "Admin: list users"),
    ]
    await application.bot.set_my_commands(commands)


async def post_shutdown(application):
    """Clean up resources on shutdown."""
    if whatsapp:
        await whatsapp.stop()
    if discord_adapter:
        await discord_adapter.stop()
    if slack_adapter:
        await slack_adapter.stop()
    if memory:
        memory.close()
    if llm:
        await llm.close()
    logger.info("AmanClaw shut down cleanly.")


# --- Main ---

def main():
    global config, memory, llm, whatsapp, learning_engine, mcp_manager
    global processor, telegram_adapter, discord_adapter, slack_adapter

    logger.info("Starting AmanClaw...")

    config = load_config()
    webhook_config = config.get("webhook")

    # Initialize components
    db_path = os.environ.get("MEMORY_DB_PATH") or config.get("memory_db", "memory.db")
    memory = Memory(db_path)
    auth = Auth(config, memory=memory)
    rate_limiter = RateLimiter(config.get("rate_limit_per_minute", 20))
    llm = LLM(config.get("llm", {}))

    # User skill manager
    user_skill_mgr = UserSkillManager(memory)
    set_user_skill_manager(user_skill_mgr)
    logger.info("User skill manager initialized")

    # Validate admin_users
    admin_users = config.get("admin_users", {})
    has_admins = any(ids for ids in admin_users.values() if ids)
    if not has_admins:
        logger.warning(
            "No admin users configured! No one can approve new users. "
            "Add your user ID to config.yaml under admin_users."
        )

    # Configure skills
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

    # Learning engine
    learning_config = config.get("learning", {})
    if learning_config.get("enabled", True):
        learning_engine = LearningEngine(memory)
        from amanclaw.skills.remember import set_learning_engine
        set_learning_engine(learning_engine)
        logger.info("Learning engine initialized")

    # MCP Client
    mcp_manager = MCPManager(config)
    if config.get("mcp_servers"):
        asyncio.get_event_loop().run_until_complete(mcp_manager.start())
    set_mcp_manager(mcp_manager)
    logger.info("MCP client initialized")

    # Message Processor
    processor = MessageProcessor(config, auth, rate_limiter, memory, llm, learning_engine)

    # --- Channel Adapters ---

    # WhatsApp (optional)
    wa_config = config.get("whatsapp", {})
    if wa_config.get("enabled"):
        from amanclaw.channels.whatsapp import WhatsAppAdapter
        whatsapp = WhatsAppAdapter(config, processor)
        logger.info("WhatsApp adapter configured (will start with bot)")

    # Discord (optional)
    if config.get("discord", {}).get("enabled", False):
        from amanclaw.channels.discord import DiscordAdapter
        discord_adapter = DiscordAdapter(config, processor)
        asyncio.get_event_loop().run_until_complete(discord_adapter.start())
        logger.info("Discord adapter started")

    # Slack (optional)
    if config.get("slack", {}).get("enabled", False):
        from amanclaw.channels.slack import SlackAdapter
        slack_adapter = SlackAdapter(config, processor)
        asyncio.get_event_loop().run_until_complete(slack_adapter.start())
        logger.info("Slack adapter started")

    # Telegram
    token = config.get("telegram", {}).get("bot_token") or os.environ.get("TELEGRAM_BOT_TOKEN")
    if not token:
        logger.error("Telegram bot token not found in config.yaml or TELEGRAM_BOT_TOKEN env var.")
        sys.exit(1)

    telegram_adapter = TelegramAdapter(config, processor, memory, llm, learning_engine)

    app = ApplicationBuilder().token(token).post_init(post_init).post_shutdown(post_shutdown).build()

    # Register Telegram handlers
    telegram_adapter.register_handlers(app)

    # Error handler
    app.add_error_handler(error_handler)

    # Schedule jobs
    app.job_queue.run_repeating(check_reminders, interval=30, first=5)
    app.job_queue.run_repeating(check_schedules, interval=60, first=15)
    app.job_queue.run_daily(prune_job, time=datetime_time(hour=3, minute=0))

    if learning_config.get("proactive_checkins", True):
        checkin_day = learning_config.get("checkin_day", 6)
        checkin_hour = learning_config.get("checkin_hour", 10)
        app.job_queue.run_daily(checkin_job, time=datetime_time(hour=checkin_hour, minute=0),
                                days=(checkin_day,))

    if webhook_config and webhook_config.get("enabled"):
        webhook_url = webhook_config["url"]
        listen = webhook_config.get("listen", "0.0.0.0")
        port = webhook_config.get("port", 8443)
        secret_token = os.environ.get("WEBHOOK_SECRET") or webhook_config.get("secret_token")
        logger.info(f"Starting webhook mode on {listen}:{port}")
        app.run_webhook(
            listen=listen,
            port=port,
            url_path=f"webhook/{token[:10]}",
            webhook_url=f"{webhook_url}/webhook/{token[:10]}",
            secret_token=secret_token,
            allowed_updates=["message", "callback_query"],
        )
    else:
        logger.info("Starting polling mode")
        app.run_polling(allowed_updates=["message", "callback_query"])

    # Cleanup after run_polling returns
    if mcp_manager:
        asyncio.get_event_loop().run_until_complete(mcp_manager.stop())
    if memory:
        memory.close()


if __name__ == "__main__":
    main()
