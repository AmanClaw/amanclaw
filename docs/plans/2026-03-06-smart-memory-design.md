# Smart Memory System Design

**Date:** 2026-03-06
**Status:** Approved
**Approach:** Enhanced SQLite Knowledge Graph (Approach 1)

## Problem

The current memory system uses a flat `facts` table (key/value pairs) that cannot represent:
- Contextual preferences (conditions, time-of-day)
- Relationships between entities (people, projects, places)
- Temporal knowledge (facts that expire or change)
- Rich structured knowledge with confidence and source tracking

## Solution

Replace the flat facts store with a three-table knowledge graph in SQLite, plus FTS5 full-text search. Add LLM-powered background knowledge extraction after each exchange.

## Data Model

### `knowledge` table (replaces `facts`)

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | Auto-increment |
| user_id | TEXT | Owner |
| category | TEXT | `preference`, `personal`, `work`, `health`, `routine`, `temporal` |
| subject | TEXT | Topic: `coffee`, `diet`, `meeting` |
| content | TEXT | The knowledge: `prefers dark roast` |
| context | TEXT | Optional condition: `in the evening`, `on weekdays` |
| valid_from | DATE | When this became true (NULL = always) |
| valid_until | DATE | When this expires (NULL = no expiry) |
| confidence | REAL | 0.0-1.0, default 1.0 |
| source | TEXT | `conversation`, `explicit`, `inferred` |
| created_at | DATETIME | Auto |
| updated_at | DATETIME | Auto |

### `entities` table

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | Auto-increment |
| user_id | TEXT | Owner |
| name | TEXT | Entity name |
| entity_type | TEXT | `person`, `project`, `place`, `organization` |
| attributes | TEXT | JSON blob for typed attributes |
| created_at | DATETIME | Auto |

Unique constraint on `(user_id, name, entity_type)`.

### `relationships` table

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | Auto-increment |
| user_id | TEXT | Owner |
| from_entity_id | INTEGER FK | References `entities(id)` |
| relation | TEXT | `works_on`, `manages`, `lives_in`, `reports_to` |
| to_entity_id | INTEGER FK | References `entities(id)` |
| context | TEXT | Optional extra info |
| created_at | DATETIME | Auto |

### FTS5 Index

```sql
CREATE VIRTUAL TABLE knowledge_fts USING fts5(
    subject, content, context,
    content=knowledge, content_rowid=id
);
```

## Knowledge Extraction

After each conversation exchange, a background async task extracts structured knowledge:

1. User message + bot reply sent to LLM with extraction prompt
2. LLM returns JSON with: knowledge entries, entities, relationships, updates to existing entries
3. Results saved to new tables
4. Runs in background -- does not block the user's reply
5. Includes existing knowledge summary to avoid duplicates
6. Skips extraction for greetings/small talk (LLM decides)

### Extraction Prompt Structure

The prompt asks the LLM to return JSON with:
- `knowledge[]` -- new facts with category, subject, content, context, valid_until
- `entities[]` -- new entities with name, type, attributes
- `relationships[]` -- new relationships between entities
- `updates[]` -- corrections to existing knowledge (by ID)

## Knowledge Retrieval

When building LLM context for a new message:

1. **Active knowledge** -- all non-expired entries for the user
2. **Relevant entities** -- FTS5 match on current message keywords
3. **Related knowledge** -- FTS5 search, top 10 results
4. **Relationships** -- connections between matched entities

### Context Format in System Prompt

Knowledge injected into `## What I know about this user` section, organized by category:
- Preferences (with context conditions)
- Personal info
- Temporal facts (with expiry dates)
- People & Projects (entities + relationships)

## Code Changes

### `memory.py`
- New methods: `save_knowledge()`, `search_knowledge()`, `get_active_knowledge()`, `save_entity()`, `get_entities()`, `get_entity_by_name()`, `save_relationship()`, `get_relationships()`, `expire_old_knowledge()`
- Migration logic: copy existing `facts` rows to `knowledge` table on first run
- FTS5 index maintenance (triggers or manual sync)

### `bot.py`
- `build_context()` upgraded to query knowledge graph
- Background extraction task after each exchange

### `llm.py`
- `_build_system_prompt()` formats knowledge by category
- New `extract_knowledge()` method with extraction prompt
- Extraction prompt template

### `skills/remember.py`
- `save_fact` skill writes to `knowledge` table (with category, context)
- New `recall` skill for user-initiated knowledge search

### Migration
- On init, if `knowledge` table doesn't exist, create it and migrate `facts` rows
- `facts` table kept but no longer written to by new code

## Expired Knowledge

- Daily prune job (existing) extended to soft-delete expired knowledge
- Expired entries marked, not deleted, for historical queries

## Estimated Size

~200-300 lines of new/changed Python code. Zero new dependencies.
