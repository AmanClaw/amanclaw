pub const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    namespace TEXT NOT NULL DEFAULT 'default',
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
    namespace TEXT NOT NULL DEFAULT 'default',
    summary TEXT NOT NULL,
    message_count INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_messages_user ON messages(user_id);
CREATE INDEX IF NOT EXISTS idx_messages_ns_user ON messages(namespace, user_id);
CREATE INDEX IF NOT EXISTS idx_facts_user ON facts(user_id);
CREATE INDEX IF NOT EXISTS idx_summaries_ns_user ON summaries(namespace, user_id);

CREATE TABLE IF NOT EXISTS communities (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    zone TEXT NOT NULL DEFAULT 'WLY01',
    language TEXT NOT NULL DEFAULT 'rojak',
    platform TEXT NOT NULL,
    platform_group_id TEXT NOT NULL UNIQUE,
    enabled_skills TEXT NOT NULL DEFAULT '[]',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS community_notifications (
    community_id TEXT NOT NULL REFERENCES communities(id),
    notification_type TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (community_id, notification_type)
);

CREATE TABLE IF NOT EXISTS community_admins (
    community_id TEXT NOT NULL REFERENCES communities(id),
    user_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    added_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (community_id, user_id)
);

CREATE TABLE IF NOT EXISTS users (
    user_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'pending',
    username TEXT,
    first_name TEXT,
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, platform)
);

CREATE INDEX IF NOT EXISTS idx_users_state ON users(state);
CREATE INDEX IF NOT EXISTS idx_users_platform ON users(platform);

CREATE TABLE IF NOT EXISTS vector_documents (
    id TEXT NOT NULL,
    collection TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    embedding BLOB,
    PRIMARY KEY (collection, id)
);

CREATE INDEX IF NOT EXISTS idx_vector_collection ON vector_documents(collection);

CREATE VIRTUAL TABLE IF NOT EXISTS vector_documents_fts
USING fts5(content, tokenize='unicode61 remove_diacritics 2', content='vector_documents', content_rowid='rowid');

CREATE TRIGGER IF NOT EXISTS vector_documents_ai AFTER INSERT ON vector_documents BEGIN
    INSERT INTO vector_documents_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TRIGGER IF NOT EXISTS vector_documents_ad AFTER DELETE ON vector_documents BEGIN
    INSERT INTO vector_documents_fts(vector_documents_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;

CREATE TRIGGER IF NOT EXISTS vector_documents_au AFTER UPDATE ON vector_documents BEGIN
    INSERT INTO vector_documents_fts(vector_documents_fts, rowid, content) VALUES('delete', old.rowid, old.content);
    INSERT INTO vector_documents_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TABLE IF NOT EXISTS cron_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    status TEXT NOT NULL,
    output TEXT,
    duration_ms INTEGER,
    executed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_cron_history_job ON cron_history(job_id, executed_at);

CREATE TABLE IF NOT EXISTS webhook_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    webhook_id TEXT NOT NULL,
    status TEXT NOT NULL,
    source_ip TEXT,
    payload_preview TEXT,
    error TEXT,
    duration_ms INTEGER,
    received_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_webhook_history ON webhook_history(webhook_id, received_at);
"#;

/// Migration statements for existing databases that lack namespace columns.
/// Each runs independently — errors ignored if columns already exist.
pub const MIGRATE_NS_STMTS: &[&str] = &[
    "ALTER TABLE messages ADD COLUMN namespace TEXT NOT NULL DEFAULT 'default';",
    "ALTER TABLE summaries ADD COLUMN namespace TEXT NOT NULL DEFAULT 'default';",
    "CREATE INDEX IF NOT EXISTS idx_messages_ns_user ON messages(namespace, user_id);",
    "CREATE INDEX IF NOT EXISTS idx_summaries_ns_user ON summaries(namespace, user_id);",
];

/// SQL to create Reactive Learning Engine tables.
pub const RLE_INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS correction_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trigger_pattern TEXT NOT NULL,
    wrong_response TEXT,
    correct_response TEXT NOT NULL,
    topic TEXT,
    user_id TEXT,
    community_id TEXT,
    layer TEXT NOT NULL DEFAULT 'global' CHECK (layer IN ('user', 'community', 'global')),
    confidence REAL NOT NULL DEFAULT 0.7,
    hit_count INTEGER NOT NULL DEFAULT 0,
    last_used DATETIME,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'candidate', 'retracted')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS correction_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id INTEGER REFERENCES correction_rules(id),
    user_id TEXT NOT NULL,
    platform TEXT,
    signal_type TEXT NOT NULL,
    signal_confidence REAL NOT NULL,
    source_user_msg TEXT,
    source_bot_msg TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_rules_lookup ON correction_rules(layer, status, topic);
CREATE INDEX IF NOT EXISTS idx_rules_user ON correction_rules(user_id, status);
CREATE INDEX IF NOT EXISTS idx_rules_community ON correction_rules(community_id, status);
CREATE INDEX IF NOT EXISTS idx_events_rule ON correction_events(rule_id);
"#;
