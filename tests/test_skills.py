"""Tests for skill execution — shell, files, and skill registry."""

import os
import json
import tempfile
import pytest
from pathlib import Path
from amanclaw.skills import execute, get_tool_definitions, get_skill_list, REGISTRY
from amanclaw.skills.shell import run_command
from amanclaw.skills.files import read_file, write_file, list_files, configure as configure_files


# --- Skill Registry ---

class TestSkillRegistry:
    def test_skills_registered(self):
        expected = {"run_command", "read_file", "write_file", "list_files",
                    "system_status", "save_fact", "get_facts", "recall",
                    "set_reminder", "list_reminders", "cancel_reminder"}
        assert expected.issubset(set(REGISTRY.keys()))

    def test_tool_definitions_format(self):
        tools = get_tool_definitions()
        assert len(tools) > 0
        for tool in tools:
            assert "name" in tool
            assert "description" in tool
            assert "input_schema" in tool
            assert tool["input_schema"]["type"] == "object"

    def test_skill_list_readable(self):
        text = get_skill_list()
        assert "run_command" in text
        assert "read_file" in text

    def test_unknown_skill(self):
        result = execute("nonexistent_skill", {})
        assert "Unknown skill" in result


# --- Shell Skill ---

class TestShellSkill:
    def test_allowed_command(self):
        result = run_command("whoami")
        assert result and result.strip()

    def test_blocked_command(self):
        result = run_command("rm -rf /")
        assert "not allowed" in result

    def test_dangerous_chars_pipe(self):
        result = run_command("ls | grep test")
        assert "dangerous characters" in result

    def test_dangerous_chars_semicolon(self):
        result = run_command("ls; rm -rf /")
        assert "dangerous characters" in result

    def test_dangerous_chars_backtick(self):
        result = run_command("echo `whoami`")
        assert "dangerous characters" in result

    def test_dangerous_chars_dollar(self):
        result = run_command("echo $HOME")
        assert "dangerous characters" in result

    def test_dangerous_chars_redirect(self):
        result = run_command("ls > /tmp/out")
        assert "dangerous characters" in result

    def test_empty_command(self):
        result = run_command("")
        assert "Empty command" in result or "Invalid" in result

    def test_path_traversal_passwd(self):
        result = run_command("cat ../../etc/passwd")
        assert "blocked" in result.lower() or "not allowed" in result.lower()

    def test_ls_runs(self):
        result = run_command("ls /tmp")
        assert result  # Should return something

    def test_date_runs(self):
        result = run_command("date")
        assert result and len(result) > 0


# --- File Skill ---

class TestFileSkill:
    @pytest.fixture(autouse=True)
    def setup_workspace(self, tmp_path):
        """Use a temp directory as workspace for tests."""
        configure_files(workspace_dir=str(tmp_path))
        self.workspace = tmp_path

    def test_write_and_read(self):
        result = write_file("test.txt", "hello world")
        assert "11 characters" in result

        content = read_file("test.txt")
        assert content == "hello world"

    def test_read_nonexistent(self):
        result = read_file("nope.txt")
        assert "not found" in result.lower()

    def test_write_nested(self):
        result = write_file("sub/dir/file.txt", "nested content")
        assert "nested content" in read_file("sub/dir/file.txt")

    def test_list_files(self):
        write_file("a.txt", "aaa")
        write_file("b.txt", "bbb")
        result = list_files(".")
        assert "a.txt" in result
        assert "b.txt" in result

    def test_list_empty_dir(self):
        (self.workspace / "empty").mkdir()
        result = list_files("empty")
        assert "empty" in result.lower()

    def test_path_escape(self):
        result = read_file("../../etc/passwd")
        assert "escapes workspace" in result.lower() or "not found" in result.lower()

    def test_write_path_escape(self):
        result = write_file("../../evil.txt", "bad")
        assert "escapes workspace" in result.lower()

    def test_large_file_blocked(self):
        big_file = self.workspace / "big.txt"
        big_file.write_text("x" * 200_000)
        result = read_file("big.txt")
        assert "too large" in result.lower()

    def test_read_output_capped(self):
        large = "x" * 10_000
        write_file("large.txt", large)
        content = read_file("large.txt")
        assert len(content) <= 5000


# --- Memory Tests ---

class TestMemory:
    @pytest.fixture
    def memory(self):
        from amanclaw.memory import Memory
        m = Memory(":memory:")
        yield m
        m.close()

    def test_save_and_get_history(self, memory):
        memory.save_exchange("user1", "telegram", "hello", "hi there")
        history = memory.get_history("user1")
        assert len(history) == 2
        assert history[0]["role"] == "user"
        assert history[0]["content"] == "hello"
        assert history[1]["role"] == "assistant"
        assert history[1]["content"] == "hi there"

    def test_history_limit(self, memory):
        for i in range(30):
            memory.save_message("user1", "telegram", "user", f"msg {i}")
        history = memory.get_history("user1", last_n=10)
        assert len(history) == 10

    def test_clear_history(self, memory):
        memory.save_exchange("user1", "telegram", "hello", "hi")
        memory.clear_history("user1")
        history = memory.get_history("user1")
        assert len(history) == 0

    def test_user_isolation(self, memory):
        memory.save_exchange("user1", "telegram", "hello", "hi")
        memory.save_exchange("user2", "telegram", "hey", "yo")
        assert len(memory.get_history("user1")) == 2
        assert len(memory.get_history("user2")) == 2

    def test_save_and_get_facts(self, memory):
        memory.save_fact("user1", "name", "Alice")
        memory.save_fact("user1", "language", "Python")
        facts = memory.get_facts("user1")
        assert facts == {"name": "Alice", "language": "Python"}

    def test_fact_upsert(self, memory):
        memory.save_fact("user1", "name", "Alice")
        memory.save_fact("user1", "name", "Bob")
        facts = memory.get_facts("user1")
        assert facts["name"] == "Bob"

    def test_facts_user_isolation(self, memory):
        memory.save_fact("user1", "name", "Alice")
        memory.save_fact("user2", "name", "Bob")
        assert memory.get_facts("user1")["name"] == "Alice"
        assert memory.get_facts("user2")["name"] == "Bob"

    def test_stats(self, memory):
        memory.save_exchange("user1", "telegram", "hello", "hi")
        memory.save_fact("user1", "name", "Alice")
        stats = memory.get_stats()
        assert stats["total_messages"] == 2
        assert stats["total_facts"] == 1
        assert stats["unique_users"] == 1

    def test_add_and_get_reminders(self, memory):
        memory.add_reminder("user1", "telegram", "12345", "Check oven", "2020-01-01 00:00:00")
        due = memory.get_due_reminders()
        assert len(due) == 1
        assert due[0]["message"] == "Check oven"
        assert due[0]["chat_id"] == "12345"

    def test_mark_reminder_delivered(self, memory):
        memory.add_reminder("user1", "telegram", "12345", "Test", "2020-01-01 00:00:00")
        due = memory.get_due_reminders()
        memory.mark_reminder_delivered(due[0]["id"])
        assert len(memory.get_due_reminders()) == 0

    def test_user_reminders(self, memory):
        memory.add_reminder("user1", "telegram", "12345", "First", "2030-01-01 00:00:00")
        memory.add_reminder("user1", "telegram", "12345", "Second", "2030-01-02 00:00:00")
        reminders = memory.get_user_reminders("user1")
        assert len(reminders) == 2

    def test_delete_reminder(self, memory):
        memory.add_reminder("user1", "telegram", "12345", "Delete me", "2030-01-01 00:00:00")
        reminders = memory.get_user_reminders("user1")
        assert memory.delete_reminder(reminders[0]["id"], "user1")
        assert len(memory.get_user_reminders("user1")) == 0

    def test_delete_wrong_user(self, memory):
        memory.add_reminder("user1", "telegram", "12345", "Protected", "2030-01-01 00:00:00")
        reminders = memory.get_user_reminders("user1")
        assert not memory.delete_reminder(reminders[0]["id"], "user2")

    def test_export_history(self, memory):
        memory.save_exchange("user1", "telegram", "hello", "hi there")
        export = memory.export_history("user1")
        assert "USER: hello" in export
        assert "ASSISTANT: hi there" in export

    def test_export_empty(self, memory):
        assert memory.export_history("user1") == "No conversation history."


# --- User Management Tests ---

class TestUserManagement:
    @pytest.fixture
    def memory(self):
        from amanclaw.memory import Memory
        m = Memory(":memory:")
        yield m
        m.close()

    def test_register_new_user(self, memory):
        assert memory.register_user("user1", "telegram", "johndoe", "John", "Doe")
        user = memory.get_user("user1")
        assert user["username"] == "johndoe"
        assert user["first_name"] == "John"
        assert user["status"] == "pending"

    def test_register_duplicate(self, memory):
        assert memory.register_user("user1", "telegram")
        assert not memory.register_user("user1", "telegram")

    def test_user_status_flow(self, memory):
        memory.register_user("user1", "telegram")
        assert memory.get_user_status("user1") == "pending"
        memory.approve_user("user1")
        assert memory.get_user_status("user1") == "approved"

    def test_block_user(self, memory):
        memory.register_user("user1", "telegram")
        memory.block_user("user1")
        assert memory.get_user_status("user1") == "blocked"

    def test_approve_only_pending(self, memory):
        memory.register_user("user1", "telegram")
        memory.block_user("user1")
        assert not memory.approve_user("user1")

    def test_unknown_user_status(self, memory):
        assert memory.get_user_status("nonexistent") is None

    def test_list_users(self, memory):
        memory.register_user("user1", "telegram", "alice")
        memory.register_user("user2", "telegram", "bob")
        memory.approve_user("user1")
        all_users = memory.list_users()
        assert len(all_users) == 2
        pending = memory.list_users(status="pending")
        assert len(pending) == 1
        assert pending[0]["username"] == "bob"
        approved = memory.list_users(status="approved")
        assert len(approved) == 1
        assert approved[0]["username"] == "alice"


# --- Knowledge Graph Tests ---

class TestKnowledgeGraph:
    @pytest.fixture
    def memory(self):
        from amanclaw.memory import Memory
        m = Memory(":memory:")
        yield m
        m.close()

    def test_save_and_get_knowledge(self, memory):
        memory.save_knowledge("user1", category="preference", subject="coffee",
                              content="prefers dark roast", context="morning only")
        entries = memory.get_active_knowledge("user1")
        assert len(entries) == 1
        assert entries[0]["subject"] == "coffee"
        assert entries[0]["content"] == "prefers dark roast"
        assert entries[0]["context"] == "morning only"

    def test_knowledge_categories(self, memory):
        memory.save_knowledge("user1", category="preference", subject="coffee", content="dark roast")
        memory.save_knowledge("user1", category="personal", subject="name", content="Aman")
        memory.save_knowledge("user1", category="temporal", subject="diet",
                              content="keto diet", valid_until="2026-03-31")
        entries = memory.get_active_knowledge("user1")
        assert len(entries) == 3
        categories = {e["category"] for e in entries}
        assert categories == {"preference", "personal", "temporal"}

    def test_knowledge_expiry(self, memory):
        memory.save_knowledge("user1", category="temporal", subject="trip",
                              content="visiting Tokyo", valid_until="2020-01-01")
        entries = memory.get_active_knowledge("user1")
        assert len(entries) == 0  # expired

    def test_knowledge_update(self, memory):
        kid = memory.save_knowledge("user1", category="preference", subject="coffee",
                                    content="dark roast")
        memory.update_knowledge(kid, content="light roast")
        entries = memory.get_active_knowledge("user1")
        assert entries[0]["content"] == "light roast"

    def test_knowledge_user_isolation(self, memory):
        memory.save_knowledge("user1", category="personal", subject="name", content="Aman")
        memory.save_knowledge("user2", category="personal", subject="name", content="Ali")
        assert len(memory.get_active_knowledge("user1")) == 1
        assert len(memory.get_active_knowledge("user2")) == 1

    def test_search_knowledge(self, memory):
        memory.save_knowledge("user1", category="preference", subject="coffee", content="dark roast every morning")
        memory.save_knowledge("user1", category="preference", subject="tea", content="green tea in evening")
        results = memory.search_knowledge("user1", "morning coffee")
        assert len(results) >= 1
        assert any("coffee" in r["subject"] for r in results)

    def test_save_entity(self, memory):
        eid = memory.save_entity("user1", name="Ali", entity_type="person",
                                 attributes={"email": "ali@co.com", "role": "engineer"})
        entities = memory.get_entities("user1")
        assert len(entities) == 1
        assert entities[0]["name"] == "Ali"
        assert entities[0]["attributes"]["email"] == "ali@co.com"

    def test_entity_upsert(self, memory):
        memory.save_entity("user1", name="Ali", entity_type="person",
                           attributes={"role": "engineer"})
        memory.save_entity("user1", name="Ali", entity_type="person",
                           attributes={"role": "senior engineer", "email": "ali@co.com"})
        entities = memory.get_entities("user1")
        assert len(entities) == 1
        assert entities[0]["attributes"]["role"] == "senior engineer"

    def test_get_entity_by_name(self, memory):
        memory.save_entity("user1", name="SecureClaw", entity_type="project",
                           attributes={"desc": "security tool"})
        entity = memory.get_entity_by_name("user1", "SecureClaw")
        assert entity is not None
        assert entity["entity_type"] == "project"

    def test_save_relationship(self, memory):
        eid1 = memory.save_entity("user1", name="Ali", entity_type="person", attributes={})
        eid2 = memory.save_entity("user1", name="SecureClaw", entity_type="project", attributes={})
        memory.save_relationship("user1", eid1, "works_on", eid2)
        rels = memory.get_relationships("user1")
        assert len(rels) == 1
        assert rels[0]["relation"] == "works_on"

    def test_get_relationships_for_entity(self, memory):
        eid1 = memory.save_entity("user1", name="Ali", entity_type="person", attributes={})
        eid2 = memory.save_entity("user1", name="SecureClaw", entity_type="project", attributes={})
        eid3 = memory.save_entity("user1", name="Bob", entity_type="person", attributes={})
        memory.save_relationship("user1", eid1, "works_on", eid2)
        memory.save_relationship("user1", eid3, "works_on", eid2)
        rels = memory.get_relationships("user1", entity_id=eid2)
        assert len(rels) == 2

    def test_expire_old_knowledge(self, memory):
        memory.save_knowledge("user1", category="temporal", subject="trip",
                              content="visiting Tokyo", valid_until="2020-01-01")
        memory.save_knowledge("user1", category="personal", subject="name", content="Aman")
        count = memory.expire_old_knowledge()
        assert count == 1

    def test_migrate_facts_to_knowledge(self, memory):
        # Simulate old-style facts
        memory.save_fact("user1", "name", "Aman")
        memory.save_fact("user1", "timezone", "UTC+8")
        memory.migrate_facts_to_knowledge()
        entries = memory.get_active_knowledge("user1")
        subjects = {e["subject"] for e in entries}
        assert "name" in subjects
        assert "timezone" in subjects


# --- Knowledge Extraction Tests ---

class TestKnowledgeExtraction:
    def test_parse_extraction_response_valid(self):
        from amanclaw.llm import parse_extraction_response
        raw = json.dumps({
            "knowledge": [
                {"category": "preference", "subject": "coffee", "content": "dark roast"}
            ],
            "entities": [
                {"name": "Ali", "type": "person", "attributes": {"role": "engineer"}}
            ],
            "relationships": [
                {"from": "Ali", "relation": "works_on", "to": "SecureClaw"}
            ],
            "updates": []
        })
        result = parse_extraction_response(raw)
        assert len(result["knowledge"]) == 1
        assert result["knowledge"][0]["subject"] == "coffee"
        assert len(result["entities"]) == 1
        assert len(result["relationships"]) == 1

    def test_parse_extraction_response_empty(self):
        from amanclaw.llm import parse_extraction_response
        raw = json.dumps({"knowledge": [], "entities": [], "relationships": [], "updates": []})
        result = parse_extraction_response(raw)
        assert result["knowledge"] == []

    def test_parse_extraction_response_invalid_json(self):
        from amanclaw.llm import parse_extraction_response
        result = parse_extraction_response("not json at all")
        assert result is None

    def test_parse_extraction_response_json_in_markdown(self):
        from amanclaw.llm import parse_extraction_response
        raw = '```json\n{"knowledge": [{"category": "personal", "subject": "name", "content": "Aman"}], "entities": [], "relationships": [], "updates": []}\n```'
        result = parse_extraction_response(raw)
        assert len(result["knowledge"]) == 1


# --- Remember Skill with Knowledge Graph ---

class TestRememberSkillKnowledge:
    @pytest.fixture(autouse=True)
    def setup(self):
        from amanclaw.memory import Memory
        from amanclaw.skills.remember import configure, set_current_user
        self.memory = Memory(":memory:")
        configure(memory=self.memory)
        set_current_user("testuser")
        yield
        self.memory.close()

    def test_save_fact_writes_to_knowledge(self):
        from amanclaw.skills.remember import save_fact
        result = save_fact(key="timezone", value="UTC+8")
        assert "Remembered" in result
        # Should be in knowledge table
        entries = self.memory.get_active_knowledge("testuser")
        assert any(e["subject"] == "timezone" for e in entries)

    def test_save_fact_with_category(self):
        from amanclaw.skills.remember import save_fact
        result = save_fact(key="coffee", value="dark roast", category="preference")
        entries = self.memory.get_active_knowledge("testuser")
        coffee = [e for e in entries if e["subject"] == "coffee"]
        assert len(coffee) == 1
        assert coffee[0]["category"] == "preference"

    def test_recall_skill(self):
        from amanclaw.skills.remember import save_fact, recall
        save_fact(key="name", value="Aman")
        save_fact(key="language", value="Python")
        result = recall(query="name")
        assert "Aman" in result


# --- Knowledge Integration Tests ---

class TestKnowledgeIntegration:
    @pytest.fixture
    def memory(self):
        from amanclaw.memory import Memory
        m = Memory(":memory:")
        yield m
        m.close()

    def test_full_knowledge_flow(self, memory):
        """Test the complete knowledge lifecycle."""
        # Save some knowledge
        kid = memory.save_knowledge("user1", "preference", "coffee", "dark roast",
                                    context="morning only")
        assert kid > 0

        # Save entities
        eid1 = memory.save_entity("user1", "Ali", "person", {"role": "engineer"})
        eid2 = memory.save_entity("user1", "SecureClaw", "project", {"type": "security"})

        # Save relationship
        memory.save_relationship("user1", eid1, "works_on", eid2)

        # Query everything
        knowledge = memory.get_active_knowledge("user1")
        assert len(knowledge) == 1

        entities = memory.get_entities("user1")
        assert len(entities) == 2

        rels = memory.get_relationships("user1")
        assert len(rels) == 1
        assert rels[0]["from_name"] == "Ali"
        assert rels[0]["to_name"] == "SecureClaw"

        # Search
        results = memory.search_knowledge("user1", "coffee")
        assert len(results) >= 1

        # Update
        memory.update_knowledge(kid, content="light roast")
        knowledge = memory.get_active_knowledge("user1")
        assert knowledge[0]["content"] == "light roast"

    def test_facts_migration_on_init(self):
        """Test that existing facts are migrated to knowledge on init."""
        from amanclaw.memory import Memory
        m = Memory(":memory:")
        # Save facts using old method
        m.save_fact("user1", "name", "Aman")
        m.save_fact("user1", "lang", "Python")
        # Migration should have happened at init since facts exist
        # But we need to re-trigger since we added facts after init
        m.migrate_facts_to_knowledge()
        entries = m.get_active_knowledge("user1")
        assert len(entries) >= 2
        m.close()
