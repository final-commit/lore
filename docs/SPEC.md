# Forge Rust Backend — Technical Specification

**Version:** 1.0.0
**Date:** 2026-03-02
**Status:** Implementation Ready

---

## 1. Overview

Forge is an AI-native, self-hosted documentation platform where Markdown files in a Git repository are the single source of truth. This spec covers the Rust backend that replaces the Node.js/Hono/tRPC prototype.

**Why Rust:** Self-hosted users run on Raspberry Pi (1-4GB), $5 VPS (512MB), old laptops. Node.js needs 1-1.5GB for 100 users. Rust needs ~200MB.

## 2. Technology Stack

| Component | Library | Version | Rationale |
|-----------|---------|---------|-----------|
| Web framework | **Axum** | 0.8.x | Tokio-native, Tower middleware, best balance of perf + ergonomics |
| Async runtime | **Tokio** | 1.x | Industry standard, multi-threaded |
| Git operations | **git2** (libgit2) | 0.19.x | Battle-tested, full feature set. gix still maturing for write ops |
| Collaborative editing | **yrs** | 0.21.x | Official Yjs Rust port, CRDT, full compat with JS Yjs clients |
| Full-text search | **Tantivy** | 0.25.x | Embedded, 2x faster than Lucene, BM25 + fuzzy |
| Database | **rusqlite** | 0.32.x | SQLite-focused, lightweight, no compile-time DB needed |
| Cache | **moka** | 0.12.x | Concurrent, async, TTL + LRU eviction |
| Serialization | **serde** + **serde_json** | 1.x | Standard |
| Password hashing | **argon2** | 0.5.x | Memory-hard, timing-safe |
| JWT | **jsonwebtoken** | 9.x | Encode/decode, RS256 support |
| WebSocket | **axum built-in** | (via tokio-tungstenite) | Native upgrade support |
| YAML frontmatter | **serde_yaml** | 0.9.x | Parse/emit frontmatter |
| Markdown parsing | **pulldown-cmark** | 0.12.x | CommonMark compliant, fast |
| Logging | **tracing** + **tracing-subscriber** | 0.1.x / 0.3.x | Structured, async-aware |
| Config | **figment** | 0.10.x | Layered config (file + env + defaults) |
| UUID | **uuid** | 1.x | v7 (time-ordered) for primary keys |
| HMAC | **hmac** + **sha2** | 0.12.x / 0.10.x | Webhook signature verification |
| HTTP client | **reqwest** | 0.12.x | For webhook dispatch |

### Frontend (Unchanged)
React (Next.js 16) frontend from the existing `packages/web/`. Talks to Rust backend via REST + WebSocket. The tRPC layer is replaced with typed REST endpoints that match the same API shape.

## 3. Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Web Browser                          │
│  Next.js 16 (React 19) — Tiptap v3 — Yjs — Tailwind 4    │
└───────────────────────────┬─────────────────────────────────┘
                            │ HTTP REST + WebSocket
┌───────────────────────────┴─────────────────────────────────┐
│                  Forge Server (Rust / Axum)                  │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────┐ │
│  │  REST API    │  │  Git Engine │  │  Yjs WebSocket       │ │
│  │  (Axum)      │  │  (git2)     │  │  (yrs + tungstenite) │ │
│  └─────────────┘  └─────────────┘  └──────────────────────┘ │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────┐ │
│  │  Comment     │  │  Search     │  │  Auth                │ │
│  │  Engine      │  │  (Tantivy)  │  │  (Argon2 + JWT)      │ │
│  │  (SQLite)    │  │             │  │                      │ │
│  └─────────────┘  └─────────────┘  └──────────────────────┘ │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────┐ │
│  │  Cache       │  │  Sync       │  │  Webhook             │ │
│  │  (moka)      │  │  Engine     │  │  Dispatch            │ │
│  └─────────────┘  └─────────────┘  └──────────────────────┘ │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────┴─────────────────────────────────┐
│  Git Repository (.md files)  │  SQLite (WAL) — metadata     │
└─────────────────────────────────────────────────────────────┘
```

## 4. Module Structure

```
lore/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, server startup
│   ├── config.rs            # Figment-based config
│   ├── error.rs             # AppError type, Into<Response>
│   ├── state.rs             # AppState (shared across handlers)
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── middleware.rs    # Axum middleware (extract user from JWT/cookie)
│   │   ├── handler.rs      # Login, register, token refresh
│   │   ├── token.rs        # JWT encode/decode, API token management
│   │   └── password.rs     # Argon2id hash/verify
│   ├── api/
│   │   ├── mod.rs           # Router composition
│   │   ├── docs.rs          # GET /api/docs/:path, PUT, POST, DELETE
│   │   ├── tree.rs          # GET /api/tree
│   │   ├── comments.rs      # CRUD comments, threads, resolve
│   │   ├── search.rs        # GET /api/search?q=
│   │   ├── sync.rs          # POST /api/sync/pull, /push, /status
│   │   ├── health.rs        # GET /health
│   │   └── webhooks.rs      # POST /api/webhooks/git (HMAC verified)
│   ├── git/
│   │   ├── mod.rs
│   │   ├── engine.rs        # GitEngine: read, write, history, diff
│   │   └── queue.rs         # Async operation queue (serialize git ops)
│   ├── comments/
│   │   ├── mod.rs
│   │   └── engine.rs        # CommentEngine: SQLite CRUD, threading, anchoring
│   ├── search/
│   │   ├── mod.rs
│   │   └── engine.rs        # Tantivy index, query, incremental update
│   ├── sync/
│   │   ├── mod.rs
│   │   └── engine.rs        # Pull/push, conflict detection, branch creation
│   ├── realtime/
│   │   ├── mod.rs
│   │   └── yjs.rs           # Yjs WebSocket handler, room-per-doc, save on close
│   ├── cache/
│   │   ├── mod.rs
│   │   └── page.rs          # moka async cache, commit-hash invalidation
│   └── db/
│       ├── mod.rs
│       └── schema.rs         # SQLite schema (rusqlite migrations)
├── tests/
│   ├── git_engine.rs
│   ├── comment_engine.rs
│   ├── search_engine.rs
│   ├── auth.rs
│   ├── sync_engine.rs
│   ├── cache.rs
│   ├── queue.rs
│   ├── api_integration.rs
│   └── common/
│       └── mod.rs           # Test helpers (temp dirs, test DB, etc.)
└── docs/
    └── SPEC.md              # This file
```

## 5. API Endpoints

All endpoints return JSON. Authentication via `Authorization: Bearer <token>` header or `forge_session` cookie.

### Documents
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/tree` | reader | Document tree (recursive) |
| GET | `/api/docs/*path` | reader | Get document content + frontmatter |
| PUT | `/api/docs/*path` | editor | Update document |
| POST | `/api/docs` | editor | Create new document |
| DELETE | `/api/docs/*path` | admin | Delete document |
| GET | `/api/docs/*path/history` | reader | Git log for file |

### Comments
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/comments?doc_path=` | reader | List comments for doc |
| POST | `/api/comments` | editor | Create comment (top-level or reply) |
| PUT | `/api/comments/:id` | owner | Update comment body |
| DELETE | `/api/comments/:id` | owner/admin | Delete comment |
| POST | `/api/comments/:id/resolve` | editor | Resolve thread |

### Search
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/search?q=&limit=` | reader | Full-text search |

### Auth
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/auth/register` | none | Create account |
| POST | `/api/auth/login` | none | Get JWT |
| POST | `/api/auth/refresh` | valid refresh | Refresh access token |
| GET | `/api/auth/me` | any | Current user info |
| POST | `/api/auth/tokens` | admin | Create API token (scoped) |
| DELETE | `/api/auth/tokens/:id` | admin | Revoke API token |

### Sync
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/sync/pull` | editor | Pull from remote |
| POST | `/api/sync/push` | editor | Push to remote |
| GET | `/api/sync/status` | reader | Sync state (ahead/behind/conflicts) |

### Webhooks
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/webhooks/git` | HMAC | Receive push events from GitHub/GitLab/Gitea |

### System
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health` | none | Health check (git, db, search, realtime stats) |

### WebSocket
| Path | Protocol | Description |
|------|----------|-------------|
| `/ws/yjs/:doc_path` | WebSocket | Yjs collaborative editing (room per doc) |

## 6. Data Models

### SQLite Schema

```sql
-- Users
CREATE TABLE users (
    id TEXT PRIMARY KEY,           -- uuid v7
    email TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'editor',  -- admin, editor, reader
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Sessions (JWT refresh tokens)
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    refresh_token_hash TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- API Tokens (for CI/agents)
CREATE TABLE api_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    scope TEXT NOT NULL DEFAULT 'read',  -- read, write, admin
    expires_at TEXT,
    last_used_at TEXT,
    created_at TEXT NOT NULL
);

-- Comments
CREATE TABLE comments (
    id TEXT PRIMARY KEY,
    doc_path TEXT NOT NULL,
    parent_id TEXT REFERENCES comments(id) ON DELETE CASCADE,
    author_id TEXT NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    anchor_text TEXT,              -- text the comment is anchored to
    anchor_start INTEGER,
    anchor_end INTEGER,
    resolved_at TEXT,
    resolved_by TEXT REFERENCES users(id),
    is_agent BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_comments_doc ON comments(doc_path);
CREATE INDEX idx_comments_parent ON comments(parent_id);

-- Sync State
CREATE TABLE sync_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),  -- singleton
    last_pull_at TEXT,
    last_push_at TEXT,
    last_pull_commit TEXT,
    last_push_commit TEXT,
    remote_url TEXT,
    branch TEXT NOT NULL DEFAULT 'main'
);

-- Webhook Config
CREATE TABLE webhook_configs (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,        -- github, gitlab, gitea
    secret TEXT NOT NULL,
    events TEXT NOT NULL DEFAULT 'push',
    created_at TEXT NOT NULL
);
```

## 7. Key Design Decisions

### 7.1 git2 over gix
gix (gitoxide) is pure Rust but still maturing for write operations (commit, merge, push). git2 wraps libgit2 which is battle-tested in production (used by Cargo). The FFI cost is negligible for our use case.

### 7.2 rusqlite over sqlx
SQLite-only project. No need for multi-DB support or compile-time query checking (which requires a running DB during CI). rusqlite is lighter and gives direct access to SQLite extensions.

### 7.3 Tantivy embedded over MeiliSearch
Single binary deployment. No separate search service. Tantivy runs in-process with ~50MB RAM overhead. Perfect for self-hosted.

### 7.4 JWT + Argon2 over session-only auth
API tokens (JWT) are essential for CI/agent integration. Argon2id is the current best practice for password hashing (memory-hard, timing-safe).

### 7.5 REST over tRPC
tRPC is TypeScript-specific. Rust backend means we use typed REST endpoints. The frontend's `api.ts` already uses fetch — minimal change needed.

### 7.6 Operation Queue
All git operations go through an async queue (tokio::sync::Mutex + channel) to prevent interleaving. Same pattern as the Node.js version but with tokio primitives.

## 8. Performance Targets

| Metric | Target | How |
|--------|--------|-----|
| Idle memory | < 50MB | Single binary, no runtime |
| 100 users memory | < 250MB | moka bounded cache, Tantivy memory-mapped |
| API response (cached) | < 10ms | moka async cache |
| API response (git read) | < 50ms | git2 in-process, no CLI spawn |
| Search query | < 20ms | Tantivy in-process |
| WebSocket latency | < 50ms | yrs in-process |
| Startup time | < 500ms | No JIT, no module resolution |
| Docker image | < 100MB | Static linking, alpine base |
| Cold start to first request | < 1s | Including DB migration + search index |

## 9. Development Plan (TDD)

### Phase 1: Foundation (tests first)
1. Config + error types
2. Database setup + migrations (rusqlite)
3. Git engine (read tree, read file, write file, history)
4. Operation queue

### Phase 2: Core Engines
5. Comment engine (CRUD, threading, anchoring)
6. Search engine (index, query, incremental update)
7. Cache (page cache with commit-hash invalidation)
8. Auth (register, login, JWT, API tokens, middleware)

### Phase 3: API Layer
9. REST endpoints (docs, comments, search, auth, sync, health)
10. Webhook handler (HMAC verification)
11. Rate limiting middleware

### Phase 4: Realtime
12. Yjs WebSocket server (room-per-doc, load from git, save on close)
13. Sync engine (pull, push, conflict detection)

### Phase 5: Integration
14. Full API integration tests
15. Docker build
16. Frontend API client update (point to Rust backend)

## 10. Error Handling

All errors use a unified `AppError` enum that implements `IntoResponse`:

```rust
pub enum AppError {
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    BadRequest(String),
    Conflict(String),
    Internal(String),
    Git(git2::Error),
    Db(rusqlite::Error),
    Search(tantivy::TantivyError),
}
```

Each variant maps to an HTTP status code. Errors are logged via `tracing` and returned as `{ "error": "message" }`.

## 11. Security

- Path traversal prevention (reject `..`, absolute paths, null bytes)
- Argon2id for passwords (memory: 64MB, iterations: 3, parallelism: 4)
- JWT with RS256 (asymmetric) or HS256 (symmetric, for simplicity v1)
- HMAC-SHA256 webhook verification
- Rate limiting (100/min API, 10/min auth)
- CORS configured per environment
- All user input validated before git/db operations
