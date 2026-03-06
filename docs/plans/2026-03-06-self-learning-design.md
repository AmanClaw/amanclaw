# Self-Learning Engine Design

**Date:** 2026-03-06
**Status:** Approved
**Approach:** Layered Learning Engine (Approach A)

## Goal

Make AmanClaw self-learning like a baby — it learns from corrections, feedback patterns,
external sources, its own mistakes, and active teaching by the user.

## Architecture

```
User Message
    |
    v
Bot Handler --> LLM Respond --> Reply
    |                              |
    v                              v
Learning Engine (background, non-blocking)
    |
    +-- Correction Detector --> update knowledge + log correction
    +-- Knowledge Extractor --> knowledge graph (existing, enhanced)
    +-- Teaching Processor --> teachings table
    +-- Document Ingestor --> chunked knowledge
    +-- Failure Tracker --> failure_log
    +-- Pattern Analyzer --> behavioral_patterns (weekly job)
                                    |
                                    v
                            Proactive Check-ins (weekly)
```

## New Database Tables

### corrections
| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | Auto-increment |
| user_id | TEXT | Owner |
| knowledge_id | INTEGER FK | The knowledge entry corrected |
| old_content | TEXT | What was wrong |
| new_content | TEXT | What is right |
| trigger_text | TEXT | User message that triggered correction |
| created_at | DATETIME | Auto |

### teachings
| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | Auto-increment |
| user_id | TEXT | Owner |
| trigger_pattern | TEXT | "when I say X" or topic |
| response_guidance | TEXT | "do Y" or "the answer is Z" |
| category | TEXT | Category for organization |
| active | INTEGER | 1=active, 0=disabled |
| usage_count | INTEGER | Times this teaching was used |
| created_at | DATETIME | Auto |

### documents
| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | Auto-increment |
| user_id | TEXT | Owner |
| source_name | TEXT | Filename or URL |
| source_type | TEXT | pdf, txt, url |
| chunk_index | INTEGER | Position in document |
| content | TEXT | Chunk text (~500 chars) |
| created_at | DATETIME | Auto |

### failure_log
| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | Auto-increment |
| user_id | TEXT | Owner |
| skill_name | TEXT | Tool that was called |
| skill_input | TEXT | JSON of input args |
| error_message | TEXT | What went wrong |
| user_feedback | TEXT | "try again", "wrong", etc. |
| resolved | INTEGER | 0=unresolved, 1=resolved |
| created_at | DATETIME | Auto |

### behavioral_patterns
| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | Auto-increment |
| user_id | TEXT | Owner |
| pattern_type | TEXT | response_length, topic_frequency, time_pattern |
| description | TEXT | Human-readable pattern |
| evidence | TEXT | JSON of supporting data |
| confidence | REAL | 0.0-1.0 |
| confirmed | INTEGER | 0=observed, 1=user-confirmed |
| created_at | DATETIME | Auto |
| updated_at | DATETIME | Auto |

## Confidence Hierarchy

1. Explicit teachings (1.0)
2. Corrections (0.95)
3. Explicit facts (0.9)
4. Inferred knowledge (0.7)
5. Document knowledge (0.6)
6. Behavioral patterns (0.5)

## User-Facing Features

- Gentle acks when learning happens
- /teach command for structured teaching
- /learned command for learning journal
- /forget command to remove knowledge
- Proactive check-ins (max 1-2 per week)
- Document ingestion via file + "learn this"

## Estimated Scope

~900 lines new/changed Python. Zero new dependencies.
