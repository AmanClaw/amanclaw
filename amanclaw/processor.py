# amanclaw/processor.py
"""
Channel-agnostic message processing pipeline.

Extracted from bot.py to allow any channel adapter to process messages
through the same auth -> sanitize -> LLM -> learn pipeline.
"""

import asyncio
import logging
from amanclaw.channels import IncomingMessage, OutgoingMessage
from amanclaw.security import sanitize
from amanclaw.skills.remember import set_current_user
from amanclaw.skills.reminder import set_context as set_reminder_context
from amanclaw.skills.scheduled import set_context as set_scheduled_context
from amanclaw.skills.documents import set_learning_context as set_doc_learning_context

logger = logging.getLogger("amanclaw.processor")


class MessageProcessor:
    """Channel-agnostic message processing pipeline."""

    def __init__(self, config, auth, rate_limiter, memory, llm, learning=None):
        self.config = config
        self.auth = auth
        self.rate_limiter = rate_limiter
        self.memory = memory
        self.llm = llm
        self.learning = learning

    async def process(self, msg: IncomingMessage) -> OutgoingMessage | None:
        """
        Full pipeline: auth -> rate limit -> sanitize -> context -> LLM -> learn.
        Returns OutgoingMessage, or None if message should be silently dropped.
        """
        user_id = msg.user_id
        platform = msg.platform

        # --- Auth check ---
        state = self.auth.get_user_state(user_id, platform)

        if state == "blocked":
            return None

        if state == "new":
            self.memory.register_user(
                user_id=user_id,
                platform=platform,
                username=msg.username,
                first_name=msg.first_name,
            )
            return OutgoingMessage(
                chat_id=msg.chat_id,
                text="Welcome! You've been registered.\n\n"
                     "An admin needs to approve your access before you can start chatting. "
                     "Please wait for approval.",
            )

        if state == "pending":
            return OutgoingMessage(
                chat_id=msg.chat_id,
                text="Your registration is pending approval. "
                     "An admin will review your request shortly.",
            )

        # state is "admin" or "approved"

        # --- Rate limit ---
        if not self.rate_limiter.check(user_id):
            return OutgoingMessage(
                chat_id=msg.chat_id,
                text="Slow down — too many messages. Try again in a minute.",
            )

        # --- Sanitize ---
        clean_text, was_flagged = sanitize(msg.text)
        if was_flagged:
            logger.warning(f"Flagged message from {user_id} on {platform}: {msg.text[:100]}")

        # --- Set skill context ---
        set_current_user(user_id)
        set_reminder_context(user_id, msg.chat_id)
        set_scheduled_context(user_id, msg.chat_id)
        if self.learning:
            set_doc_learning_context(user_id, self.learning)

        # --- Build context ---
        history = self.memory.get_history(user_id)
        facts = self.memory.get_facts(user_id)
        summary = self.memory.get_latest_summary(user_id)

        knowledge_entries = self.memory.get_active_knowledge(user_id)
        entities = self.memory.get_entities(user_id)
        relationships = self.memory.get_relationships(user_id)

        if clean_text:
            relevant = self.memory.search_knowledge(user_id, clean_text, limit=5)
            existing_ids = {k["id"] for k in knowledge_entries}
            for r in relevant:
                if r["id"] not in existing_ids:
                    knowledge_entries.append(r)

        from amanclaw.llm import format_knowledge_context
        knowledge_context = format_knowledge_context(knowledge_entries, entities, relationships)

        # --- Auto-summarize ---
        msg_count = self.memory.get_message_count(user_id)
        summarized_count = self.memory.get_summarized_message_count(user_id)
        unsummarized = msg_count - summarized_count
        if unsummarized > 40:
            old_msgs = self.memory.get_old_messages(user_id, before_last_n=20, limit=40)
            if old_msgs:
                try:
                    new_summary = await self.llm.summarize(old_msgs, summary)
                    self.memory.save_summary(user_id, new_summary, len(old_msgs))
                    summary = new_summary
                except Exception as e:
                    logger.error(f"Summarization failed: {e}")

        # --- LLM response ---
        try:
            response = await self.llm.respond(
                clean_text, history, flagged=was_flagged,
                facts=facts, summary=summary,
                knowledge_context=knowledge_context,
            )
        except Exception as e:
            logger.error(f"LLM error: {e}")
            response = "Something went wrong talking to the AI. Try again in a moment."

        # --- Save exchange ---
        self.memory.save_exchange(user_id, platform, msg.text, response)

        # --- Background learning ---
        if self.learning:
            asyncio.create_task(self._extract_knowledge(user_id, msg.text, response))

            if "failed:" in response.lower() or "error:" in response.lower():
                self.learning.log_failure(user_id, "llm_response",
                                          {"message": clean_text[:200]}, response[:500])

        return OutgoingMessage(chat_id=msg.chat_id, text=response)

    async def _extract_knowledge(self, user_id: str, user_msg: str, response: str):
        """Background knowledge extraction (non-blocking)."""
        try:
            from amanclaw.bot import extract_and_save_knowledge
            await extract_and_save_knowledge(user_id, user_msg, response)
        except Exception as e:
            logger.debug(f"Knowledge extraction failed: {e}")
