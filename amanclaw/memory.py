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

            CREATE TABLE IF NOT EXISTS schedules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                platform TEXT NOT NULL,
                chat_id TEXT NOT NULL,
                message TEXT NOT NULL,
                cron_hour INTEGER NOT NULL,
                cron_minute INTEGER NOT NULL,
                cron_days TEXT NOT NULL DEFAULT '0,1,2,3,4,5,6',
                enabled INTEGER DEFAULT 1,
                last_run DATE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_reminders_pending
                ON reminders(delivered, remind_at);

            CREATE TABLE IF NOT EXISTS knowledge (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                category TEXT NOT NULL,
                subject TEXT NOT NULL,
                content TEXT NOT NULL,
                context TEXT,
                valid_from DATE,
                valid_until DATE,
                confidence REAL DEFAULT 1.0,
                source TEXT DEFAULT 'conversation',
                expired INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_knowledge_user
                ON knowledge(user_id, expired);

            CREATE TABLE IF NOT EXISTS entities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                attributes TEXT DEFAULT '{}',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, name, entity_type)
            );

            CREATE TABLE IF NOT EXISTS relationships (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                from_entity_id INTEGER REFERENCES entities(id),
                relation TEXT NOT NULL,
                to_entity_id INTEGER REFERENCES entities(id),
                context TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
        """)
        self.conn.commit()

        # FTS5 index for knowledge search
        try:
            self.conn.execute("""
                CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(
                    subject, content, context,
                    content=knowledge, content_rowid=id
                )
            """)
            self.conn.commit()
        except Exception:
            pass  # FTS5 may already exist

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

    def add_schedule(self, user_id, platform, chat_id, message, hour, minute, days="0,1,2,3,4,5,6"):
        self.conn.execute(
            "INSERT INTO schedules (user_id, platform, chat_id, message, cron_hour, cron_minute, cron_days) VALUES (?,?,?,?,?,?,?)",
            (str(user_id), platform, str(chat_id), message, hour, minute, days)
        )
        self.conn.commit()

    def get_due_schedules(self):
        """Get schedules that should run now (matching hour, minute, day of week, not yet run today)."""
        from datetime import datetime
        now = datetime.now()
        today = now.strftime("%Y-%m-%d")
        dow = str(now.weekday())  # 0=Monday
        rows = self.conn.execute(
            "SELECT id, user_id, platform, chat_id, message FROM schedules "
            "WHERE enabled = 1 AND cron_hour = ? AND cron_minute = ? "
            "AND (last_run IS NULL OR last_run < ?) "
            "AND cron_days LIKE '%' || ? || '%'",
            (now.hour, now.minute, today, dow)
        ).fetchall()
        return [{"id": r[0], "user_id": r[1], "platform": r[2], "chat_id": r[3], "message": r[4]} for r in rows]

    def mark_schedule_run(self, schedule_id):
        from datetime import datetime
        self.conn.execute(
            "UPDATE schedules SET last_run = ? WHERE id = ?",
            (datetime.now().strftime("%Y-%m-%d"), schedule_id)
        )
        self.conn.commit()

    def get_user_schedules(self, user_id):
        rows = self.conn.execute(
            "SELECT id, message, cron_hour, cron_minute, cron_days, enabled FROM schedules WHERE user_id = ? ORDER BY cron_hour, cron_minute",
            (str(user_id),)
        ).fetchall()
        return [{"id": r[0], "message": r[1], "hour": r[2], "minute": r[3], "days": r[4], "enabled": r[5]} for r in rows]

    def delete_schedule(self, schedule_id, user_id):
        cursor = self.conn.execute("DELETE FROM schedules WHERE id = ? AND user_id = ?", (schedule_id, str(user_id)))
        self.conn.commit()
        return cursor.rowcount > 0

    def toggle_schedule(self, schedule_id, user_id):
        self.conn.execute(
            "UPDATE schedules SET enabled = CASE WHEN enabled = 1 THEN 0 ELSE 1 END WHERE id = ? AND user_id = ?",
            (schedule_id, str(user_id))
        )
        self.conn.commit()

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

    def prune_old_messages(self, user_id: str, keep_last: int = 100) -> int:
        """Delete messages older than the most recent `keep_last` for a user."""
        cursor = self.conn.execute(
            "DELETE FROM messages WHERE user_id = ? AND id NOT IN "
            "(SELECT id FROM messages WHERE user_id = ? ORDER BY id DESC LIMIT ?)",
            (str(user_id), str(user_id), keep_last)
        )
        self.conn.commit()
        deleted = cursor.rowcount
        if deleted:
            logger.info(f"Pruned {deleted} old messages for user {user_id}")
        return deleted

    def prune_all_users(self, keep_last: int = 100) -> int:
        """Prune old messages for all users. Returns total deleted count."""
        rows = self.conn.execute(
            "SELECT DISTINCT user_id FROM messages"
        ).fetchall()
        total = 0
        for (user_id,) in rows:
            total += self.prune_old_messages(user_id, keep_last)
        return total

    def prune_delivered_reminders(self, older_than_days: int = 30) -> int:
        """Delete delivered reminders older than N days. Returns count deleted."""
        cursor = self.conn.execute(
            "DELETE FROM reminders WHERE delivered = 1 AND remind_at <= datetime('now', ?)",
            (f"-{older_than_days} days",)
        )
        self.conn.commit()
        deleted = cursor.rowcount
        if deleted:
            logger.info(f"Pruned {deleted} delivered reminders older than {older_than_days} days")
        return deleted

    # --- Scheduled Tasks ---

    def add_schedule(self, user_id: str, platform: str, chat_id: str,
                     message: str, hour: int, minute: int, days: str = "0,1,2,3,4,5,6"):
        """Add a recurring schedule."""
        self.conn.execute(
            "INSERT INTO schedules (user_id, platform, chat_id, message, cron_hour, cron_minute, cron_days) "
            "VALUES (?, ?, ?, ?, ?, ?, ?)",
            (str(user_id), platform, str(chat_id), message, hour, minute, days)
        )
        self.conn.commit()

    def get_due_schedules(self) -> list[dict]:
        """Get schedules that should run now (matching hour, minute, day of week, not yet run today)."""
        now = datetime.now()
        today = now.strftime("%Y-%m-%d")
        dow = str(now.weekday())  # 0=Monday
        rows = self.conn.execute(
            "SELECT id, user_id, platform, chat_id, message FROM schedules "
            "WHERE enabled = 1 AND cron_hour = ? AND cron_minute = ? "
            "AND (last_run IS NULL OR last_run < ?) "
            "AND cron_days LIKE '%' || ? || '%'",
            (now.hour, now.minute, today, dow)
        ).fetchall()
        return [{"id": r[0], "user_id": r[1], "platform": r[2], "chat_id": r[3], "message": r[4]} for r in rows]

    def mark_schedule_run(self, schedule_id: int):
        """Mark a schedule as run today."""
        self.conn.execute(
            "UPDATE schedules SET last_run = ? WHERE id = ?",
            (datetime.now().strftime("%Y-%m-%d"), schedule_id)
        )
        self.conn.commit()

    def get_user_schedules(self, user_id: str) -> list[dict]:
        """Get all schedules for a user."""
        rows = self.conn.execute(
            "SELECT id, message, cron_hour, cron_minute, cron_days, enabled "
            "FROM schedules WHERE user_id = ? ORDER BY cron_hour, cron_minute",
            (str(user_id),)
        ).fetchall()
        return [{"id": r[0], "message": r[1], "hour": r[2], "minute": r[3],
                 "days": r[4], "enabled": r[5]} for r in rows]

    def delete_schedule(self, schedule_id: int, user_id: str) -> bool:
        """Delete a schedule. Returns True if deleted."""
        cursor = self.conn.execute(
            "DELETE FROM schedules WHERE id = ? AND user_id = ?",
            (schedule_id, str(user_id))
        )
        self.conn.commit()
        return cursor.rowcount > 0

    def close(self):
        self.conn.close()

    # --- Knowledge Graph ---

    def save_knowledge(self, user_id: str, category: str, subject: str, content: str,
                       context: str = None, valid_from: str = None, valid_until: str = None,
                       confidence: float = 1.0, source: str = "conversation") -> int:
        """Save a knowledge entry. Returns the knowledge ID."""
        cursor = self.conn.execute(
            """INSERT INTO knowledge (user_id, category, subject, content, context,
                   valid_from, valid_until, confidence, source)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)""",
            (str(user_id), category, subject, content, context,
             valid_from, valid_until, confidence, source)
        )
        kid = cursor.lastrowid
        # Sync FTS5 index
        try:
            self.conn.execute(
                "INSERT INTO knowledge_fts(rowid, subject, content, context) VALUES (?, ?, ?, ?)",
                (kid, subject, content, context or "")
            )
        except Exception:
            pass
        self.conn.commit()
        return kid

    def update_knowledge(self, knowledge_id: int, content: str = None, context: str = None,
                         valid_until: str = None, confidence: float = None):
        """Update fields of a knowledge entry."""
        updates = []
        params = []
        if content is not None:
            updates.append("content = ?")
            params.append(content)
        if context is not None:
            updates.append("context = ?")
            params.append(context)
        if valid_until is not None:
            updates.append("valid_until = ?")
            params.append(valid_until)
        if confidence is not None:
            updates.append("confidence = ?")
            params.append(confidence)
        if not updates:
            return
        updates.append("updated_at = CURRENT_TIMESTAMP")
        params.append(knowledge_id)
        self.conn.execute(
            f"UPDATE knowledge SET {', '.join(updates)} WHERE id = ?", params
        )
        # Sync FTS5 index
        try:
            row = self.conn.execute(
                "SELECT subject, content, context FROM knowledge WHERE id = ?",
                (knowledge_id,)
            ).fetchone()
            if row:
                self.conn.execute("DELETE FROM knowledge_fts WHERE rowid = ?", (knowledge_id,))
                self.conn.execute(
                    "INSERT INTO knowledge_fts(rowid, subject, content, context) VALUES (?, ?, ?, ?)",
                    (knowledge_id, row[0], row[1], row[2] or "")
                )
        except Exception:
            pass
        self.conn.commit()

    def get_active_knowledge(self, user_id: str) -> list[dict]:
        """Get all active (non-expired) knowledge for a user."""
        rows = self.conn.execute(
            """SELECT id, category, subject, content, context, valid_from, valid_until,
                      confidence, source, created_at, updated_at
               FROM knowledge
               WHERE user_id = ? AND expired = 0
                 AND (valid_until IS NULL OR valid_until >= date('now'))
               ORDER BY category, subject""",
            (str(user_id),)
        ).fetchall()
        return [
            {"id": r[0], "category": r[1], "subject": r[2], "content": r[3],
             "context": r[4], "valid_from": r[5], "valid_until": r[6],
             "confidence": r[7], "source": r[8], "created_at": r[9], "updated_at": r[10]}
            for r in rows
        ]

    def search_knowledge(self, user_id: str, query: str, limit: int = 10) -> list[dict]:
        """Search knowledge using FTS5 with fallback to LIKE."""
        try:
            rows = self.conn.execute(
                """SELECT k.id, k.category, k.subject, k.content, k.context,
                          k.valid_from, k.valid_until, k.confidence, k.source
                   FROM knowledge_fts fts
                   JOIN knowledge k ON k.id = fts.rowid
                   WHERE knowledge_fts MATCH ? AND k.user_id = ? AND k.expired = 0
                   LIMIT ?""",
                (query, str(user_id), limit)
            ).fetchall()
        except Exception:
            rows = []

        if not rows:
            # Fallback to LIKE search
            terms = query.split()
            conditions = []
            params = [str(user_id)]
            for term in terms:
                conditions.append("(subject LIKE ? OR content LIKE ? OR context LIKE ?)")
                params.extend([f"%{term}%", f"%{term}%", f"%{term}%"])
            where_clause = " OR ".join(conditions) if conditions else "1=1"
            rows = self.conn.execute(
                f"""SELECT id, category, subject, content, context,
                           valid_from, valid_until, confidence, source
                    FROM knowledge
                    WHERE user_id = ? AND expired = 0 AND ({where_clause})
                    LIMIT ?""",
                params + [limit]
            ).fetchall()

        return [
            {"id": r[0], "category": r[1], "subject": r[2], "content": r[3],
             "context": r[4], "valid_from": r[5], "valid_until": r[6],
             "confidence": r[7], "source": r[8]}
            for r in rows
        ]

    def save_entity(self, user_id: str, name: str, entity_type: str,
                    attributes: dict = None) -> int:
        """Save or update an entity. Returns the entity ID."""
        attrs_json = json.dumps(attributes or {})
        # Try upsert
        self.conn.execute(
            """INSERT INTO entities (user_id, name, entity_type, attributes)
               VALUES (?, ?, ?, ?)
               ON CONFLICT(user_id, name, entity_type) DO UPDATE SET
                   attributes = excluded.attributes""",
            (str(user_id), name, entity_type, attrs_json)
        )
        self.conn.commit()
        row = self.conn.execute(
            "SELECT id FROM entities WHERE user_id = ? AND name = ? AND entity_type = ?",
            (str(user_id), name, entity_type)
        ).fetchone()
        return row[0]

    def get_entities(self, user_id: str, entity_type: str = None) -> list[dict]:
        """Get all entities for a user, optionally filtered by type."""
        if entity_type:
            rows = self.conn.execute(
                "SELECT id, name, entity_type, attributes, created_at FROM entities WHERE user_id = ? AND entity_type = ?",
                (str(user_id), entity_type)
            ).fetchall()
        else:
            rows = self.conn.execute(
                "SELECT id, name, entity_type, attributes, created_at FROM entities WHERE user_id = ?",
                (str(user_id),)
            ).fetchall()
        return [
            {"id": r[0], "name": r[1], "entity_type": r[2],
             "attributes": json.loads(r[3]), "created_at": r[4]}
            for r in rows
        ]

    def get_entity_by_name(self, user_id: str, name: str) -> dict | None:
        """Get an entity by name (case-insensitive)."""
        row = self.conn.execute(
            "SELECT id, name, entity_type, attributes, created_at FROM entities WHERE user_id = ? AND name COLLATE NOCASE = ?",
            (str(user_id), name)
        ).fetchone()
        if not row:
            return None
        return {"id": row[0], "name": row[1], "entity_type": row[2],
                "attributes": json.loads(row[3]), "created_at": row[4]}

    def save_relationship(self, user_id: str, from_entity_id: int, relation: str,
                          to_entity_id: int, context: str = None):
        """Save a relationship between two entities."""
        self.conn.execute(
            "INSERT INTO relationships (user_id, from_entity_id, relation, to_entity_id, context) VALUES (?, ?, ?, ?, ?)",
            (str(user_id), from_entity_id, relation, to_entity_id, context)
        )
        self.conn.commit()

    def get_relationships(self, user_id: str, entity_id: int = None) -> list[dict]:
        """Get relationships, optionally filtered by entity (as source or target)."""
        if entity_id is not None:
            rows = self.conn.execute(
                """SELECT r.id, r.from_entity_id, e1.name as from_name, r.relation,
                          r.to_entity_id, e2.name as to_name, r.context, r.created_at
                   FROM relationships r
                   JOIN entities e1 ON r.from_entity_id = e1.id
                   JOIN entities e2 ON r.to_entity_id = e2.id
                   WHERE r.user_id = ? AND (r.from_entity_id = ? OR r.to_entity_id = ?)""",
                (str(user_id), entity_id, entity_id)
            ).fetchall()
        else:
            rows = self.conn.execute(
                """SELECT r.id, r.from_entity_id, e1.name as from_name, r.relation,
                          r.to_entity_id, e2.name as to_name, r.context, r.created_at
                   FROM relationships r
                   JOIN entities e1 ON r.from_entity_id = e1.id
                   JOIN entities e2 ON r.to_entity_id = e2.id
                   WHERE r.user_id = ?""",
                (str(user_id),)
            ).fetchall()
        return [
            {"id": r[0], "from_entity_id": r[1], "from_name": r[2], "relation": r[3],
             "to_entity_id": r[4], "to_name": r[5], "context": r[6], "created_at": r[7]}
            for r in rows
        ]

    def expire_old_knowledge(self) -> int:
        """Mark knowledge entries as expired if their valid_until date has passed. Returns count."""
        cursor = self.conn.execute(
            "UPDATE knowledge SET expired = 1 WHERE expired = 0 AND valid_until IS NOT NULL AND valid_until < date('now')"
        )
        self.conn.commit()
        return cursor.rowcount

    def migrate_facts_to_knowledge(self):
        """Copy facts rows to knowledge table, deduplicating by subject."""
        rows = self.conn.execute("SELECT user_id, key, value, source FROM facts").fetchall()
        for user_id, key, value, source in rows:
            # Check if already migrated
            existing = self.conn.execute(
                "SELECT id FROM knowledge WHERE user_id = ? AND subject = ? AND category = 'personal'",
                (user_id, key)
            ).fetchone()
            if not existing:
                self.save_knowledge(user_id, category="personal", subject=key,
                                    content=value, source=source or "migrated")
