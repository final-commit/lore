use rusqlite::Connection;

/// Apply all schema migrations in order. Each migration is idempotent.
pub fn apply_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA)?;
    Ok(())
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS users (
    id          TEXT PRIMARY KEY,
    email       TEXT UNIQUE NOT NULL,
    name        TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role        TEXT NOT NULL DEFAULT 'editor',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id                  TEXT PRIMARY KEY,
    user_id             TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    refresh_token_hash  TEXT NOT NULL,
    expires_at          TEXT NOT NULL,
    created_at          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS api_tokens (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    token_hash  TEXT NOT NULL,
    scope       TEXT NOT NULL DEFAULT 'read',
    expires_at  TEXT,
    last_used_at TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS comments (
    id              TEXT PRIMARY KEY,
    doc_path        TEXT NOT NULL,
    parent_id       TEXT REFERENCES comments(id) ON DELETE CASCADE,
    author_id       TEXT NOT NULL REFERENCES users(id),
    body            TEXT NOT NULL,
    anchor_text     TEXT,
    anchor_start    INTEGER,
    anchor_end      INTEGER,
    resolved_at     TEXT,
    resolved_by     TEXT REFERENCES users(id),
    is_agent        INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_comments_doc    ON comments(doc_path);
CREATE INDEX IF NOT EXISTS idx_comments_parent ON comments(parent_id);

CREATE TABLE IF NOT EXISTS sync_state (
    id                  INTEGER PRIMARY KEY CHECK (id = 1),
    last_pull_at        TEXT,
    last_push_at        TEXT,
    last_pull_commit    TEXT,
    last_push_commit    TEXT,
    remote_url          TEXT,
    branch              TEXT NOT NULL DEFAULT 'main'
);

-- Insert default sync_state singleton
INSERT OR IGNORE INTO sync_state (id, branch) VALUES (1, 'main');

CREATE TABLE IF NOT EXISTS webhook_configs (
    id          TEXT PRIMARY KEY,
    provider    TEXT NOT NULL,
    secret      TEXT NOT NULL,
    events      TEXT NOT NULL DEFAULT 'push',
    created_at  TEXT NOT NULL
);

-- Collections (folders / workspaces)
CREATE TABLE IF NOT EXISTS collections (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    slug        TEXT NOT NULL UNIQUE,
    description TEXT,
    icon        TEXT,
    color       TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    parent_id   TEXT REFERENCES collections(id) ON DELETE SET NULL,
    permission  TEXT NOT NULL DEFAULT 'read',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_collections_parent ON collections(parent_id);
CREATE INDEX IF NOT EXISTS idx_collections_slug   ON collections(slug);

-- Templates (reusable document skeletons)
CREATE TABLE IF NOT EXISTS templates (
    id              TEXT PRIMARY KEY,
    title           TEXT NOT NULL,
    content         TEXT NOT NULL DEFAULT '',
    collection_id   TEXT REFERENCES collections(id) ON DELETE SET NULL,
    created_by      TEXT NOT NULL REFERENCES users(id),
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Document metadata (draft/publish status, archive, soft-delete)
CREATE TABLE IF NOT EXISTS document_meta (
    id              TEXT PRIMARY KEY,
    doc_path        TEXT NOT NULL UNIQUE,
    status          TEXT NOT NULL DEFAULT 'draft',
    published_at    TEXT,
    created_by      TEXT NOT NULL REFERENCES users(id),
    template_id     TEXT REFERENCES templates(id) ON DELETE SET NULL,
    archived_at     TEXT,
    deleted_at      TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_doc_meta_path     ON document_meta(doc_path);
CREATE INDEX IF NOT EXISTS idx_doc_meta_status   ON document_meta(status);
CREATE INDEX IF NOT EXISTS idx_doc_meta_deleted  ON document_meta(deleted_at);
CREATE INDEX IF NOT EXISTS idx_doc_meta_archived ON document_meta(archived_at);

-- File attachments (stored in _attachments/ inside the repo dir)
CREATE TABLE IF NOT EXISTS attachments (
    id              TEXT PRIMARY KEY,
    doc_path        TEXT NOT NULL,
    filename        TEXT NOT NULL,
    content_type    TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL,
    git_path        TEXT NOT NULL,
    created_by      TEXT NOT NULL REFERENCES users(id),
    created_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_attachments_doc ON attachments(doc_path);

-- Sprint 2: Stars (bookmarks)
CREATE TABLE IF NOT EXISTS stars (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    doc_path    TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    UNIQUE(user_id, doc_path)
);
CREATE INDEX IF NOT EXISTS idx_stars_user ON stars(user_id);

-- Sprint 2: Pins (pin important docs to top of a collection)
CREATE TABLE IF NOT EXISTS pins (
    id              TEXT PRIMARY KEY,
    doc_path        TEXT NOT NULL,
    collection_id   TEXT REFERENCES collections(id) ON DELETE CASCADE,
    pinned_by       TEXT NOT NULL REFERENCES users(id),
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    UNIQUE(doc_path, collection_id)
);
CREATE INDEX IF NOT EXISTS idx_pins_collection ON pins(collection_id);

-- Sprint 2: Views tracking (per user per doc)
CREATE TABLE IF NOT EXISTS views (
    id              TEXT PRIMARY KEY,
    doc_path        TEXT NOT NULL,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    count           INTEGER NOT NULL DEFAULT 1,
    last_viewed_at  TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    UNIQUE(doc_path, user_id)
);
CREATE INDEX IF NOT EXISTS idx_views_doc  ON views(doc_path);
CREATE INDEX IF NOT EXISTS idx_views_user ON views(user_id);

-- Sprint 2: Document shares (public links)
CREATE TABLE IF NOT EXISTS shares (
    id                  TEXT PRIMARY KEY,
    doc_path            TEXT NOT NULL,
    shared_by           TEXT NOT NULL REFERENCES users(id),
    include_child_docs  INTEGER NOT NULL DEFAULT 0,
    published           INTEGER NOT NULL DEFAULT 1,
    url_id              TEXT UNIQUE NOT NULL,
    expires_at          TEXT,
    created_at          TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_shares_doc    ON shares(doc_path);
CREATE INDEX IF NOT EXISTS idx_shares_url_id ON shares(url_id);

-- Sprint 2: Events / audit log
CREATE TABLE IF NOT EXISTS events (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    actor_id        TEXT REFERENCES users(id) ON DELETE SET NULL,
    doc_path        TEXT,
    collection_id   TEXT,
    data            TEXT,
    ip_address      TEXT,
    created_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_events_actor      ON events(actor_id);
CREATE INDEX IF NOT EXISTS idx_events_doc        ON events(doc_path);
CREATE INDEX IF NOT EXISTS idx_events_collection ON events(collection_id);
CREATE INDEX IF NOT EXISTS idx_events_name       ON events(name);

-- ── Sprint 3: Groups ──────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS groups (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    created_by  TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS group_users (
    id          TEXT PRIMARY KEY,
    group_id    TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TEXT NOT NULL,
    UNIQUE(group_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_group_users_group ON group_users(group_id);
CREATE INDEX IF NOT EXISTS idx_group_users_user  ON group_users(user_id);

-- Sprint 3: User Memberships (collection-level permissions)
CREATE TABLE IF NOT EXISTS user_memberships (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    collection_id   TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    permission      TEXT NOT NULL DEFAULT 'read',
    created_by      TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at      TEXT NOT NULL,
    UNIQUE(user_id, collection_id)
);
CREATE INDEX IF NOT EXISTS idx_memberships_user       ON user_memberships(user_id);
CREATE INDEX IF NOT EXISTS idx_memberships_collection ON user_memberships(collection_id);

-- Sprint 3: Notifications
CREATE TABLE IF NOT EXISTS notifications (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_id    TEXT REFERENCES events(id) ON DELETE SET NULL,
    type        TEXT NOT NULL,
    read_at     TEXT,
    created_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_notifications_user ON notifications(user_id);
CREATE INDEX IF NOT EXISTS idx_notifications_read ON notifications(read_at);

-- Sprint 3: Subscriptions (follow documents)
CREATE TABLE IF NOT EXISTS subscriptions (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    doc_path    TEXT NOT NULL,
    event       TEXT NOT NULL DEFAULT 'documents.update',
    created_at  TEXT NOT NULL,
    UNIQUE(user_id, doc_path, event)
);
CREATE INDEX IF NOT EXISTS idx_subscriptions_user ON subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_subscriptions_doc  ON subscriptions(doc_path);

-- Sprint 3: Reactions (emoji reactions on comments)
CREATE TABLE IF NOT EXISTS reactions (
    id          TEXT PRIMARY KEY,
    comment_id  TEXT NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji       TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    UNIQUE(comment_id, user_id, emoji)
);
CREATE INDEX IF NOT EXISTS idx_reactions_comment ON reactions(comment_id);

-- Sprint 4: Outbound webhook subscriptions
CREATE TABLE IF NOT EXISTS webhook_subscriptions (
    id          TEXT PRIMARY KEY,
    url         TEXT NOT NULL,
    secret      TEXT,
    events      TEXT NOT NULL DEFAULT '*',
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_by  TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- Sprint 4: Document relationships
CREATE TABLE IF NOT EXISTS relationships (
    id              TEXT PRIMARY KEY,
    source_doc_path TEXT NOT NULL,
    target_doc_path TEXT NOT NULL,
    rel_type        TEXT NOT NULL DEFAULT 'related',
    created_by      TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at      TEXT NOT NULL,
    UNIQUE(source_doc_path, target_doc_path)
);
CREATE INDEX IF NOT EXISTS idx_relationships_source ON relationships(source_doc_path);
CREATE INDEX IF NOT EXISTS idx_relationships_target ON relationships(target_doc_path);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn in_memory() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        conn
    }

    #[test]
    fn test_schema_applies_cleanly() {
        let conn = in_memory();
        apply_migrations(&conn).expect("migrations should succeed");
    }

    #[test]
    fn test_schema_is_idempotent() {
        let conn = in_memory();
        apply_migrations(&conn).expect("first migration");
        apply_migrations(&conn).expect("second migration (idempotent)");
    }

    #[test]
    fn test_tables_exist() {
        let conn = in_memory();
        apply_migrations(&conn).unwrap();

        let tables: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };

        for expected in &[
            "users", "sessions", "api_tokens", "comments", "sync_state",
            "webhook_configs", "collections", "templates", "document_meta", "attachments",
            "stars", "pins", "views", "shares", "events",
            // Sprint 3+4
            "groups", "group_users", "user_memberships", "notifications",
            "subscriptions", "reactions", "webhook_subscriptions", "relationships",
        ] {
            assert!(tables.contains(&expected.to_string()), "missing table: {expected}");
        }
    }

    #[test]
    fn test_sync_state_singleton() {
        let conn = in_memory();
        apply_migrations(&conn).unwrap();

        let branch: String = conn
            .query_row("SELECT branch FROM sync_state WHERE id=1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(branch, "main");
    }
}
