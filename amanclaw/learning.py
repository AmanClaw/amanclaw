"""
Learning Engine — orchestrates all self-learning pipelines.

Pipelines:
1. Correction detection and processing
2. Teaching processing (explicit user teachings)
3. Document ingestion (chunk + index)
4. Failure tracking
5. Behavioral pattern discovery
"""

import re
import json
import logging
from datetime import datetime, timedelta
from amanclaw.memory import Memory

logger = logging.getLogger("amanclaw.learning")

CORRECTION_PATTERNS = [
    r"\bno[,.]?\s+(i\s+)?(meant|prefer|want|like|use|need)",
    r"\bactually[,.]?\s+(it'?s|my|i)",
    r"\bwrong[,.]",
    r"\bthat'?s\s+not\s+(right|correct)",
    r"\bnot\s+\w+[,.]?\s+(it'?s|i\s+meant)",
    r"\bcorrection[:\s]",
    r"\bi\s+said\s+\w+[,.]?\s+not\s+",
]

TEACHING_PATTERNS = [
    r"\bremember\s+that\b",
    r"\balways\s+(respond|answer|reply|do|use)",
    r"\bfrom\s+now\s+on\b",
    r"\bteach:\s*",
    r"\bwhen\s+i\s+(say|ask|write|type)\b.*\b(mean|do|use|respond)",
    r"\bnever\s+(respond|answer|reply|do|use)",
    r"\bkeep\s+(answers?|responses?)\s+(short|brief|long|detailed)",
]


class LearningEngine:
    def __init__(self, memory: Memory):
        self.memory = memory

    # --- Correction Detection ---

    def is_correction(self, text: str) -> bool:
        text_lower = text.lower()
        for pattern in CORRECTION_PATTERNS:
            if re.search(pattern, text_lower):
                return True
        return False

    def process_correction(self, user_id: str, trigger_text: str,
                           knowledge_id: int, old_content: str,
                           new_content: str) -> bool:
        self.memory.update_knowledge(knowledge_id, content=new_content)
        self.memory.save_correction(user_id, knowledge_id, old_content, new_content, trigger_text)
        logger.info(f"Correction for user {user_id}: '{old_content}' -> '{new_content}'")
        return True

    # --- Teaching Detection ---

    def is_teaching(self, text: str) -> bool:
        text_lower = text.lower()
        for pattern in TEACHING_PATTERNS:
            if re.search(pattern, text_lower):
                return True
        return False

    def save_teaching(self, user_id: str, trigger_pattern: str,
                      response_guidance: str, category: str = "general") -> int:
        tid = self.memory.save_teaching(user_id, trigger_pattern, response_guidance, category)
        logger.info(f"New teaching for user {user_id}: '{trigger_pattern}'")
        return tid

    def get_matching_teachings(self, user_id: str, message: str) -> list[dict]:
        teachings = self.memory.get_teachings(user_id, active_only=True)
        matches = []
        message_lower = message.lower()
        for t in teachings:
            trigger = t["trigger_pattern"].lower()
            # Simple keyword matching — check if trigger words appear in message
            trigger_words = set(trigger.split())
            message_words = set(message_lower.split())
            overlap = trigger_words & message_words
            if len(overlap) >= max(1, len(trigger_words) // 2):
                matches.append(t)
                self.memory.increment_teaching_usage(t["id"])
        return matches

    # --- Document Ingestion ---

    def chunk_text(self, text: str, chunk_size: int = 500) -> list[str]:
        if len(text) <= chunk_size:
            return [text]
        chunks = []
        start = 0
        while start < len(text):
            end = start + chunk_size
            if end < len(text):
                # Try to break at sentence boundary
                last_period = text.rfind(".", start, end)
                last_newline = text.rfind("\n", start, end)
                break_at = max(last_period, last_newline)
                if break_at > start:
                    end = break_at + 1
            chunks.append(text[start:end])
            start = end
        return chunks

    def ingest_document(self, user_id: str, source_name: str, source_type: str,
                        text: str) -> int:
        # Delete existing chunks for this document (re-ingest)
        self.memory.delete_document(user_id, source_name)
        chunks = self.chunk_text(text)
        for i, chunk in enumerate(chunks):
            self.memory.save_document_chunk(user_id, source_name, source_type, i, chunk)
        logger.info(f"Ingested '{source_name}' for user {user_id}: {len(chunks)} chunks")
        return len(chunks)

    # --- Failure Tracking ---

    def log_failure(self, user_id: str, skill_name: str, skill_input: dict,
                    error_message: str) -> int:
        input_json = json.dumps(skill_input) if isinstance(skill_input, dict) else str(skill_input)
        return self.memory.save_failure(user_id, skill_name, input_json, error_message)

    def get_failure_summary(self, user_id: str) -> str:
        failures = self.memory.get_recent_failures(user_id, limit=50)
        if not failures:
            return "No failures recorded."
        # Group by skill
        by_skill = {}
        for f in failures:
            name = f["skill_name"]
            if name not in by_skill:
                by_skill[name] = []
            by_skill[name].append(f)
        lines = ["Recent failure summary:"]
        for skill_name, items in by_skill.items():
            unresolved = sum(1 for i in items if not i["resolved"])
            lines.append(f"- {skill_name}: {len(items)} failures ({unresolved} unresolved)")
            # Most common error
            errors = {}
            for i in items:
                e = i["error_message"][:80]
                errors[e] = errors.get(e, 0) + 1
            top_error = max(errors, key=errors.get)
            lines.append(f"  Most common: {top_error} ({errors[top_error]}x)")
        return "\n".join(lines)

    # --- Learning Journal ---

    def get_learning_journal(self, user_id: str, days: int = 7) -> str:
        sections = []

        # Recent knowledge
        knowledge = self.memory.get_active_knowledge(user_id)
        cutoff = (datetime.now() - timedelta(days=days)).strftime("%Y-%m-%d")
        recent_knowledge = [k for k in knowledge if k.get("created_at", "") >= cutoff]
        if recent_knowledge:
            lines = [f"**New knowledge learned ({len(recent_knowledge)} items):**"]
            for k in recent_knowledge[:10]:
                lines.append(f"- [{k['category']}] {k['subject']}: {k['content']}")
            sections.append("\n".join(lines))

        # Recent corrections
        corrections = self.memory.get_corrections(user_id, limit=10)
        recent_corrections = [c for c in corrections if c.get("created_at", "") >= cutoff]
        if recent_corrections:
            lines = [f"**Corrections ({len(recent_corrections)} updates):**"]
            for c in recent_corrections:
                lines.append(f"- Updated: '{c['old_content']}' -> '{c['new_content']}'")
            sections.append("\n".join(lines))

        # Active teachings
        teachings = self.memory.get_teachings(user_id, active_only=True)
        if teachings:
            lines = [f"**Active teachings ({len(teachings)} rules):**"]
            for t in teachings[:10]:
                used = f" (used {t['usage_count']}x)" if t['usage_count'] else ""
                lines.append(f"- {t['trigger_pattern']} -> {t['response_guidance']}{used}")
            sections.append("\n".join(lines))

        # Documents
        docs = self.memory.list_documents(user_id)
        if docs:
            lines = [f"**Ingested documents ({len(docs)}):**"]
            for d in docs:
                lines.append(f"- {d['source_name']} ({d['chunks']} chunks)")
            sections.append("\n".join(lines))

        # Failure summary
        failures = self.memory.get_recent_failures(user_id, limit=20)
        recent_failures = [f for f in failures if f.get("created_at", "") >= cutoff]
        if recent_failures:
            sections.append(self.get_failure_summary(user_id))

        # Behavioral patterns
        patterns = self.memory.get_behavioral_patterns(user_id, min_confidence=0.5)
        if patterns:
            lines = [f"**Observed patterns ({len(patterns)}):**"]
            for p in patterns:
                confirmed = " [confirmed]" if p["confirmed"] else ""
                lines.append(f"- {p['description']} (confidence: {p['confidence']:.0%}){confirmed}")
            sections.append("\n".join(lines))

        if not sections:
            return "No learning activity recorded yet. Talk to me, teach me, or send me documents to learn from!"

        return "\n\n".join(sections)
