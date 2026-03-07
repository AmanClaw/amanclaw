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
        history, facts, summary, knowledge_context = await self._build_context(user_id, clean_text)

        # --- Build LLM message (text or vision) ---
        if msg.image_data:
            llm_message = self.llm.build_vision_message(msg.image_data, caption=clean_text or None)
        else:
            llm_message = clean_text

        # --- LLM response ---
        try:
            response = await self.llm.respond(
                llm_message, history, flagged=was_flagged,
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

    async def _build_context(self, user_id: str, message_text: str = "") -> tuple[list, dict, str, str]:
        """Build the smart context: history, facts, summary, knowledge context.
        Auto-summarize if needed."""
        history = self.memory.get_history(user_id)
        facts = self.memory.get_facts(user_id)
        summary = self.memory.get_latest_summary(user_id)

        # Build knowledge graph context
        knowledge_entries = self.memory.get_active_knowledge(user_id)
        entities = self.memory.get_entities(user_id)
        relationships = self.memory.get_relationships(user_id)

        # Search for relevant knowledge based on message
        if message_text:
            relevant = self.memory.search_knowledge(user_id, message_text, limit=5)
            existing_ids = {k["id"] for k in knowledge_entries}
            for r in relevant:
                if r["id"] not in existing_ids:
                    knowledge_entries.append(r)

        from amanclaw.llm import format_knowledge_context
        knowledge_context = format_knowledge_context(knowledge_entries, entities, relationships)

        # Auto-summarize when conversation gets long
        msg_count = self.memory.get_message_count(user_id)
        summarized_count = self.memory.get_summarized_message_count(user_id)
        unsummarized = msg_count - summarized_count
        if unsummarized > 40:
            old_msgs = self.memory.get_old_messages(user_id, before_last_n=20, limit=40)
            if old_msgs:
                try:
                    new_summary = await self.llm.summarize(old_msgs, summary)
                    if new_summary:
                        if summary:
                            new_summary = f"{summary}\n\n{new_summary}"
                        self.memory.save_summary(user_id, new_summary, len(old_msgs))
                        summary = new_summary
                        logger.info(f"Auto-summarized {len(old_msgs)} messages for user {user_id}")
                except Exception as e:
                    logger.error(f"Summarization failed: {e}")

        # Add teachings, documents, behavioral patterns if learning is enabled
        if self.learning:
            teachings = self.learning.get_matching_teachings(user_id, message_text)
            if teachings:
                teaching_text = "\n\n### User-taught rules\n"
                for t in teachings:
                    teaching_text += f"- {t['trigger_pattern']}: {t['response_guidance']}\n"
                knowledge_context += teaching_text

            if message_text:
                doc_results = self.memory.search_documents(user_id, message_text, limit=3)
                if doc_results:
                    doc_text = "\n\n### From learned documents\n"
                    for d in doc_results:
                        doc_text += f"[{d['source_name']}]: {d['content'][:300]}\n"
                    knowledge_context += doc_text

            patterns = self.memory.get_behavioral_patterns(user_id, min_confidence=0.6)
            if patterns:
                pattern_text = "\n\n### Observed user preferences\n"
                for p in patterns:
                    pattern_text += f"- {p['description']}\n"
                knowledge_context += pattern_text

        return history, facts, summary, knowledge_context

    async def _extract_knowledge(self, user_id: str, user_msg: str, assistant_reply: str):
        """Background task: extract knowledge, detect corrections and teachings."""
        try:
            # Detect corrections
            if self.learning and self.learning.is_correction(user_msg):
                logger.info(f"Correction detected from user {user_id}")

            # Detect teaching intent and save
            if self.learning and self.learning.is_teaching(user_msg):
                self.learning.save_teaching(user_id, user_msg, assistant_reply, "conversation")
                logger.info(f"Teaching detected from user {user_id}")

            # Get existing knowledge for dedup context
            existing = self.memory.get_active_knowledge(user_id)
            existing_summary = "\n".join(
                f"- [{e['category']}] {e['subject']}: {e['content']}" for e in existing[:20]
            )

            extracted = await self.llm.extract_knowledge(user_msg, assistant_reply, existing_summary)
            if not extracted:
                return

            # Save knowledge entries
            for k in extracted.get("knowledge", []):
                self.memory.save_knowledge(
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
                eid = self.memory.save_entity(
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
                from_id = entity_name_to_id.get(from_name)
                to_id = entity_name_to_id.get(to_name)
                if not from_id:
                    ent = self.memory.get_entity_by_name(user_id, from_name)
                    from_id = ent["id"] if ent else None
                if not to_id:
                    ent = self.memory.get_entity_by_name(user_id, to_name)
                    to_id = ent["id"] if ent else None
                if from_id and to_id:
                    self.memory.save_relationship(user_id, from_id, r.get("relation", "related_to"), to_id)

            # Apply updates (corrections)
            for u in extracted.get("updates", []):
                kid = u.get("id")
                if kid and u.get("content"):
                    if self.learning:
                        old_entry = self.memory.conn.execute(
                            "SELECT content FROM knowledge WHERE id = ?", (kid,)
                        ).fetchone()
                        if old_entry:
                            self.learning.process_correction(
                                user_id, user_msg, kid, old_entry[0], u["content"]
                            )
                    else:
                        self.memory.update_knowledge(kid, content=u["content"])

            count = len(extracted.get("knowledge", [])) + len(extracted.get("entities", []))
            if count:
                logger.info(f"Extracted {count} knowledge items for user {user_id}")

        except Exception as e:
            logger.warning(f"Background knowledge extraction failed for {user_id}: {e}")
