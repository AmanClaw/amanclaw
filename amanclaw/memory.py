"""
Memory module — SQLite conversation history.
Keeps last N messages per user for context.
"""

import sqlite3
import json
import logging
from datetime import datetime
from pathlib import Path

logger = logging.getLogger("amanclaw.memory")


class Memory:
    def __init__(self, db_path: str = "memory.db"):
        self.db_path = db_path
        self.conn = sqlite3.connect(db_path)
        self._init_tables()
        logger.info(f"Memory initialized at {db_path}")

    def _init_tables(self):
        self.conn.executescript("""
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS facts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                source TEXT DEFAULT 'learned',
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, key)
            );

            CREATE TABLE IF NOT EXISTS summaries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                summary TEXT NOT NULL,
                message_count INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL UNIQUE,
                platform TEXT NOT NULL,
                username TEXT,
                first_name TEXT,
                last_name TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                registered_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                approved_at DATETIME
            );

            CREATE TABLE IF NOT EXISTS reminders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                chat_id TEXT NOT NULL,
                message TEXT NOT NULL,
                remind_at DATETIME NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                delivered INTEGER DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_messages_user
                ON messages(user_id, timestamp DESC);

            CREATE INDEX IF NOT EXISTS idx_reminders_pending
                ON reminders(delivered, remind_at);
        """)
        self.conn.commit()

    def get_history(self, user_id: str, last_n: int = 20) -> list[dict]:
        """Get last N messages for a user as Claude-format messages."""
        rows = self.conn.execute(
            "SELECT role, content FROM messages WHERE user_id = ? ORDER BY id DESC LIMIT ?",
            (str(user_id), last_n)
        ).fetchall()

        return [{"role": row[0], "content": row[1]} for row in reversed(rows)]

    def save_message(self, user_id: str, platform: str, role: str, content: str):
        """Save a single message."""
        self.conn.execute(
            "INSERT INTO messages (user_id, platform, role, content) VALUES (?, ?, ?, ?)",
            (str(user_id), platform, role, content)
        )
        self.conn.commit()

    def save_exchange(self, user_id: str, platform: str, user_msg: str, assistant_msg: str):
        """Save a user message + assistant reply pair."""
        self.save_message(user_id, platform, "user", user_msg)
        self.save_message(user_id, platform, "assistant", assistant_msg)

    def clear_history(self, user_id: str):
        """Clear all messages for a user."""
        self.conn.execute("DELETE FROM messages WHERE user_id = ?", (str(user_id),))
        self.conn.commit()
        logger.info(f"Cleared history for user {user_id}")

    def save_fact(self, user_id: str, key: str, value: str, source: str = "learned"):
        """Save or update a fact about the user."""
        self.conn.execute(
            """INSERT INTO facts (user_id, key, value, source, updated_at)
               VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
               ON CONFLICT(user_id, key) DO UPDATE SET
                   value = excluded.value,
                   source = excluded.source,
                   updated_at = CURRENT_TIMESTAMP""",
            (str(user_id), key, value, source)
        )
        self.conn.commit()

    def get_facts(self, user_id: str) -> dict[str, str]:
        """Get all facts about a user."""
        rows = self.conn.execute(
            "SELECT key, value FROM facts WHERE user_id = ?",
            (str(user_id),)
        ).fetchall()
        return {row[0]: row[1] for row in rows}

    def get_message_count(self, user_id: str) -> int:
        """Get total message count for a user."""
        return self.conn.execute(
            "SELECT COUNT(*) FROM messages WHERE user_id = ?",
            (str(user_id),)
        ).fetchone()[0]

    def get_old_messages(self, user_id: str, before_last_n: int = 20, limit: int = 40) -> list[dict]:
        """Get older messages (before the recent window) for summarization."""
        total = self.get_message_count(user_id)
        if total <= before_last_n:
            return []
        offset = before_last_n
        rows = self.conn.execute(
            "SELECT role, content FROM messages WHERE user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?",
            (str(user_id), limit, offset)
        ).fetchall()
        return [{"role": row[0], "content": row[1]} for row in reversed(rows)]

    def save_summary(self, user_id: str, summary: str, message_count: int):
        """Save a conversation summary."""
        self.conn.execute(
            "INSERT INTO summaries (user_id, summary, message_count) VALUES (?, ?, ?)",
            (str(user_id), summary, message_count)
        )
        self.conn.commit()

    def get_latest_summary(self, user_id: str) -> str | None:
        """Get the most recent summary for a user."""
        row = self.conn.execute(
            "SELECT summary FROM summaries WHERE user_id = ? ORDER BY id DESC LIMIT 1",
            (str(user_id),)
        ).fetchone()
        return row[0] if row else None

    def get_summarized_message_count(self, user_id: str) -> int:
        """Get the total number of messages that have been summarized."""
        row = self.conn.execute(
            "SELECT COALESCE(SUM(message_count), 0) FROM summaries WHERE user_id = ?",
            (str(user_id),)
        ).fetchone()
        return row[0]

    def get_stats(self) -> dict:
        """Get memory stats."""
        msg_count = self.conn.execute("SELECT COUNT(*) FROM messages").fetchone()[0]
        fact_count = self.conn.execute("SELECT COUNT(*) FROM facts").fetchone()[0]
        user_count = self.conn.execute("SELECT COUNT(DISTINCT user_id) FROM messages").fetchone()[0]
        summary_count = self.conn.execute("SELECT COUNT(*) FROM summaries").fetchone()[0]
        return {
            "total_messages": msg_count,
            "total_facts": fact_count,
            "unique_users": user_count,
            "total_summaries": summary_count,
        }

    def add_reminder(self, user_id: str, platform: str, chat_id: str,
                     message: str, remind_at: str):
        """Add a reminder. remind_at should be ISO format datetime string."""
        self.conn.execute(
            "INSERT INTO reminders (user_id, platform, chat_id, message, remind_at) VALUES (?, ?, ?, ?, ?)",
            (str(user_id), platform, str(chat_id), message, remind_at)
        )
        self.conn.commit()

    def get_due_reminders(self) -> list[dict]:
        """Get all reminders that are due and not yet delivered."""
        now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        rows = self.conn.execute(
            "SELECT id, user_id, platform, chat_id, message FROM reminders WHERE delivered = 0 AND remind_at <= ?",
            (now,)
        ).fetchall()
        return [{"id": r[0], "user_id": r[1], "platform": r[2], "chat_id": r[3], "message": r[4]} for r in rows]

    def mark_reminder_delivered(self, reminder_id: int):
        """Mark a reminder as delivered."""
        self.conn.execute("UPDATE reminders SET delivered = 1 WHERE id = ?", (reminder_id,))
        self.conn.commit()

    def get_user_reminders(self, user_id: str) -> list[dict]:
        """Get all pending reminders for a user."""
        rows = self.conn.execute(
            "SELECT id, message, remind_at FROM reminders WHERE user_id = ? AND delivered = 0 ORDER BY remind_at",
            (str(user_id),)
        ).fetchall()
        return [{"id": r[0], "message": r[1], "remind_at": r[2]} for r in rows]

    def delete_reminder(self, reminder_id: int, user_id: str) -> bool:
        """Delete a reminder. Returns True if deleted."""
        cursor = self.conn.execute(
            "DELETE FROM reminders WHERE id = ? AND user_id = ?",
            (reminder_id, str(user_id))
        )
        self.conn.commit()
        return cursor.rowcount > 0

    def export_history(self, user_id: str) -> str:
        """Export full conversation history as formatted text."""
        rows = self.conn.execute(
            "SELECT role, content, timestamp FROM messages WHERE user_id = ? ORDER BY id",
            (str(user_id),)
        ).fetchall()
        if not rows:
            return "No conversation history."
        lines = []
        for role, content, ts in rows:
            lines.append(f"[{ts}] {role.upper()}: {content}")
        return "\n\n".join(lines)

    # --- User Management ---

    def register_user(self, user_id: str, platform: str, username: str = None,
                      first_name: str = None, last_name: str = None) -> bool:
        """Register a new user. Returns True if newly registered, False if already exists."""
        try:
            self.conn.execute(
                """INSERT INTO users (user_id, platform, username, first_name, last_name)
                   VALUES (?, ?, ?, ?, ?)""",
                (str(user_id), platform, username, first_name, last_name)
            )
            self.conn.commit()
            logger.info(f"New user registered: {user_id} ({username or 'no username'})")
            return True
        except sqlite3.IntegrityError:
            return False

    def get_user(self, user_id: str) -> dict | None:
        """Get user record by user_id."""
        row = self.conn.execute(
            "SELECT user_id, platform, username, first_name, last_name, status, registered_at, approved_at "
            "FROM users WHERE user_id = ?",
            (str(user_id),)
        ).fetchone()
        if not row:
            return None
        return {
            "user_id": row[0], "platform": row[1], "username": row[2],
            "first_name": row[3], "last_name": row[4], "status": row[5],
            "registered_at": row[6], "approved_at": row[7],
        }

    def get_user_status(self, user_id: str) -> str | None:
        """Get user status: 'pending', 'approved', 'blocked', or None if not registered."""
        row = self.conn.execute(
            "SELECT status FROM users WHERE user_id = ?", (str(user_id),)
        ).fetchone()
        return row[0] if row else None

    def approve_user(self, user_id: str) -> bool:
        """Approve a user. Returns True if updated."""
        cursor = self.conn.execute(
            "UPDATE users SET status = 'approved', approved_at = CURRENT_TIMESTAMP WHERE user_id = ? AND status = 'pending'",
            (str(user_id),)
        )
        self.conn.commit()
        return cursor.rowcount > 0

    def block_user(self, user_id: str) -> bool:
        """Block a user. Returns True if updated."""
        cursor = self.conn.execute(
            "UPDATE users SET status = 'blocked' WHERE user_id = ? AND status != 'blocked'",
            (str(user_id),)
        )
        self.conn.commit()
        return cursor.rowcount > 0

    def list_users(self, status: str = None) -> list[dict]:
        """List all users, optionally filtered by status."""
        if status:
            rows = self.conn.execute(
                "SELECT user_id, platform, username, first_name, last_name, status, registered_at "
                "FROM users WHERE status = ? ORDER BY registered_at DESC",
                (status,)
            ).fetchall()
        else:
            rows = self.conn.execute(
                "SELECT user_id, platform, username, first_name, last_name, status, registered_at "
                "FROM users ORDER BY registered_at DESC"
            ).fetchall()
        return [
            {"user_id": r[0], "platform": r[1], "username": r[2], "first_name": r[3],
             "last_name": r[4], "status": r[5], "registered_at": r[6]}
            for r in rows
        ]

    def close(self):
        self.conn.close()
