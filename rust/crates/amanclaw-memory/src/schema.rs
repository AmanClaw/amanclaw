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
"#;

/// Migration statements for existing databases that lack namespace columns.
/// Each runs independently — errors ignored if columns already exist.
pub const MIGRATE_NS_STMTS: &[&str] = &[
    "ALTER TABLE messages ADD COLUMN namespace TEXT NOT NULL DEFAULT 'default';",
    "ALTER TABLE summaries ADD COLUMN namespace TEXT NOT NULL DEFAULT 'default';",
    "CREATE INDEX IF NOT EXISTS idx_messages_ns_user ON messages(namespace, user_id);",
    "CREATE INDEX IF NOT EXISTS idx_summaries_ns_user ON summaries(namespace, user_id);",
];
