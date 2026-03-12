# Reactive Learning Engine (RLE) — Design Spec

## Problem

AmanClaw is a stateless agent — every conversation starts from zero. The LLM has no memory of past mistakes, user preferences, or community knowledge. Users must repeat corrections. Trust erodes when the bot makes the same mistake twice.

## Solution

A Reactive Learning Engine that detects corrections, stores them as structured knowledge, and injects them into future interactions. The system gets smarter with every interaction, independent of which LLM sits underneath.

## Core Principle

The LLM is a replaceable reasoning substrate. Intelligence lives in the system's accumulated knowledge and behavioral adaptations.

## Architecture

### Three-Layer Learning Hierarchy

```
┌─────────────────────────┐
│   User-Level Learning    │  ← Personal corrections & preferences
│   (overrides below)      │
├─────────────────────────┤
│  Community-Level Learning│  ← Group-specific knowledge
│   (overrides below)      │
├─────────────────────────┤
│   Global Learning        │  ← Universal patterns from all users
│   (universal defaults)   │
└─────────────────────────┘
```

Each layer overrides the one below. User-level always wins.

### Pipeline Integration

Two new middleware slots into the existing chain:

```
Current:  Auth → RateLimit → Sanitize → Context → ToolCalling → Persist

New:      Auth → RateLimit → Sanitize → RLE_Retrieve → Context → ToolCalling → RLE_Detect → Persist
```

- **RLE_Retrieve** (before Context): queries knowledge store, injects matched rules into context extensions
- **RLE_Detect** (after ToolCalling): analyzes full exchange for correction signals, writes to knowledge store

## Component Design

### 1. Correction Detection

The LLM extracts corrections as structured data via a small focused prompt, separate from the main response.

**Detection signals:**

| Signal | Example | Confidence |
|--------|---------|------------|
| Explicit negation | "No, that's wrong. It's X" | 0.95 |
| Direct correction | "Actually it's Asr at 4:15, not 4:30" | 0.90 |
| Rephrased re-ask | Same question rephrased after bot answered | 0.75 |
| Negative reaction | "That's not helpful" | 0.60 |
| Abandoned flow | User ignores answer, changes topic | 0.40 |

**Output: CorrectionRecord**

```json
{
  "trigger": "prayer time for Asr in KL",
  "wrong_response": "4:30 PM",
  "correct_response": "4:15 PM",
  "context": {"user_id": "...", "community_id": "...", "topic": "solat"},
  "confidence": 0.90,
  "layer": "user",
  "signal_type": "direct_correction",
  "source_messages": ["user: ...", "assistant: ..."]
}
```

**Confidence thresholds:**
- High (>0.7): becomes a rule immediately
- Low (0.4–0.7): stored as candidate, promoted after repeated occurrence

### 2. Knowledge Store (Dual-layer)

**Layer A — Rule Store (SQLite)**

Structured, inspectable, fast lookup (<5ms).

Matching at query time:
1. Exact match on trigger pattern + context filter
2. Fuzzy match (keyword overlap + topic)
3. Scored by: confidence * recency * hit_count

**Layer B — Vector Store (Embeddings)**

Uses existing RAG infrastructure in amanclaw-memory. Catches corrections that don't match structured patterns (~50ms).

**Query flow:**
```
User message → Rule Store (exact/fuzzy) → hit? → inject
                                        → miss? → Vector Store (semantic) → hit? → inject
                                                                           → miss? → no learned knowledge
```

### 3. Behavioral Adaptation

**Injection modes based on confidence:**

| Confidence | Behavior |
|------------|----------|
| 0.85+ | Silent — bot uses correction without mentioning it |
| 0.6–0.85 | Soft surface — "Based on what you've told me before, X. Still correct?" |
| < 0.6 | Not injected, waits for reinforcement |

**Context injection format:**
```
[LEARNED KNOWLEDGE — treat as ground truth unless user says otherwise]
- For user {user_id}: Asr prayer in KL is 4:15 PM (corrected 2x, confidence: 0.92)
- For community {masjid-xyz}: Use Shafi'i calculation method (corrected 5x, confidence: 0.97)
```

**Decay & reinforcement:**
- Unused for 90 days: confidence decays 0.1/month
- Each hit: confidence += 0.05 (capped at 0.99)
- User says "forget that": soft-delete, mark retracted

### 4. Learning Feedback Loop

```
Correction detected
  → CorrectionRecord stored
  → Same trigger corrected 3+ times globally → promote to global rule
  → Contradicts existing rule → flag, keep higher-confidence version
  → User says "forget" → soft-delete, mark retracted

Candidate promotion:
  Low-confidence signal (0.4-0.7) → stored as candidate
  → Same pattern from different user → confidence += 0.15
  → Crosses 0.7 threshold → promoted to rule
  → 3 months no reinforcement → expired
```

## Data Model

```sql
CREATE TABLE correction_rules (
  id INTEGER PRIMARY KEY,
  trigger_pattern TEXT NOT NULL,
  wrong_response TEXT,
  correct_response TEXT NOT NULL,
  topic TEXT,
  user_id TEXT,
  community_id TEXT,
  layer TEXT NOT NULL CHECK (layer IN ('user', 'community', 'global')),
  confidence REAL DEFAULT 0.7,
  hit_count INTEGER DEFAULT 0,
  last_used DATETIME,
  status TEXT DEFAULT 'active' CHECK (status IN ('active', 'candidate', 'retracted')),
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE correction_events (
  id INTEGER PRIMARY KEY,
  rule_id INTEGER REFERENCES correction_rules(id),
  user_id TEXT NOT NULL,
  platform TEXT,
  signal_type TEXT NOT NULL,
  signal_confidence REAL NOT NULL,
  source_user_msg TEXT,
  source_bot_msg TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE correction_embeddings (
  id INTEGER PRIMARY KEY,
  rule_id INTEGER REFERENCES correction_rules(id),
  embedding BLOB NOT NULL,
  original_text TEXT,
  layer TEXT NOT NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_rules_lookup ON correction_rules(layer, status, topic);
CREATE INDEX idx_rules_user ON correction_rules(user_id, status);
CREATE INDEX idx_rules_community ON correction_rules(community_id, status);
CREATE INDEX idx_events_rule ON correction_events(rule_id);
```

## User-Facing Commands

| Command | Action |
|---------|--------|
| `/learned` | Show what the bot has learned about you |
| `/learned community` | Show community-level learnings |
| `/forget <topic>` | Delete a specific learned rule |
| `/forget all` | Wipe all user-level learnings |
| `/teach <fact>` | Explicitly teach the bot something (confidence: 0.95) |

## New Files

| File | Purpose |
|------|---------|
| `amanclaw-core/src/middleware/rle_retrieve.rs` | Retrieve middleware — query knowledge store, inject into context |
| `amanclaw-core/src/middleware/rle_detect.rs` | Detect middleware — analyze exchange for corrections |
| `amanclaw-core/src/learning/knowledge_store.rs` | Dual-layer knowledge store (rules + vectors) |
| `amanclaw-core/src/learning/detection.rs` | Correction detection engine (signal extraction) |
| `amanclaw-core/src/learning/mod.rs` | Module root |
| `amanclaw-memory/src/migrations/004_correction_tables.sql` | Schema migration |

## Future Extensions

The RLE is a foundation. Once running, these become straightforward:

- **Preference learning** — same pipeline, different signal type
- **Pattern learning** — aggregate events by time/topic → behavioral patterns
- **Effectiveness tracking** — correlate corrections with skill outputs → quality scores
- **Community intelligence** — new user joins → instantly benefits from community knowledge
- **Proactive suggestions** — high-confidence patterns → unprompted recommendations

## Success Criteria

1. Bot never makes the same corrected mistake twice for the same user
2. Community knowledge propagates to new members within their first interaction
3. Users can inspect and manage what the bot has learned
4. Detection runs in <100ms, retrieval in <10ms for rule store
5. Works with any LLM backend — intelligence is in the system, not the model
