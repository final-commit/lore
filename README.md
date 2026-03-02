# Forge

**AI-native, self-hosted, git-backed documentation platform.**

Forge is a Notion/Outline alternative that stores all documents as Markdown files in a Git repository. Built in Rust for minimal RAM usage on self-hosted hardware.

## Why Forge?

| | Outline | Notion | **Forge** |
|--|---------|--------|-----------|
| Self-hosted | ✅ | ❌ | ✅ |
| Open source | ✅ | ❌ | ✅ |
| Git-backed | ❌ | ❌ | ✅ |
| RAM (100 users) | ~1.5GB | Cloud | **~200MB** |
| Offline editing | ❌ | ❌ | ✅ |
| AI agent friendly | Partial | Partial | ✅ |
| Real-time collab | ✅ | ✅ | ✅ |

## Features

- **Markdown + Git** — Documents are `.md` files in a git repo. Edit in Forge or your editor. Both stay in sync.
- **Real-time collaboration** — Yjs CRDT-powered WebSocket collab. Multiple users, no conflicts.
- **Inline comments** — Threaded, anchored, resolvable. Agent comments tagged separately.
- **Full-text search** — Tantivy BM25 search, runs in-process (no separate service).
- **Collections** — Organize docs into named groups backed by git directories.
- **Revision history** — Git log as first-class revision history.
- **Teams & permissions** — Groups, collection-level permissions, user roles.
- **Stars, pins, views** — Bookmark, pin to collections, track recently viewed.
- **Notifications** — Subscribe to docs, get notified on changes.
- **Import** — Bring in existing docs from Outline JSON export or markdown zip.
- **API tokens** — For CI pipelines and AI agents.
- **Outbound webhooks** — Notify external systems on events.
- **Small binary** — ~15MB static binary, ~50MB Docker image.

## Quick Start

```bash
# Docker
docker run -d \
  --name forge \
  -p 3000:3000 \
  -v forge-data:/data \
  -e FORGE_JWT_SECRET="$(openssl rand -base64 32)" \
  ghcr.io/your-org/forge:latest

# Or build from source
git clone https://github.com/your-org/forge-rust
cd forge-rust
cargo build --release
FORGE_JWT_SECRET="your-secret-here" ./target/release/forge
```

Open `http://localhost:3000`. Register the first account (auto-becomes admin). Done.

## Architecture

```
Browser (React/Next.js)
    ↕ HTTP REST + WebSocket
Forge Server (Rust/Axum)
    ├── Git engine (git2/libgit2)    ← document storage
    ├── Comment engine (SQLite)      ← metadata
    ├── Search engine (Tantivy)      ← full-text, in-process
    ├── Collab engine (yrs/Yjs)      ← real-time editing
    └── Cache (moka)                 ← LRU, commit-hash invalidation
```

**No external dependencies.** Single binary. Single SQLite file. Single git repo.

## Tech Stack

| Layer | Tech |
|-------|------|
| Web framework | Axum 0.8 |
| Async runtime | Tokio 1 |
| Git | git2 (libgit2) |
| Database | SQLite via rusqlite |
| Search | Tantivy 0.22 |
| Collab | yrs (Yjs port) |
| Cache | moka 0.12 |
| Auth | Argon2id + JWT |
| Frontend | Next.js 16, React 19, Tiptap v3, Tailwind 4 |

## Development

```bash
# Backend
cd forge-rust
cargo test          # run tests (219 tests)
cargo run           # dev server on :3000

# Frontend  
cd forge/packages/web
pnpm dev            # dev server on :3001
```

See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) for production deployment.
See [docs/API.md](docs/API.md) for full API reference.

## Status

**Production-ready backend.** Active frontend development.

- ✅ 219 backend tests passing
- ✅ Full Outline feature parity (backend)
- ✅ E2E verified
- 🔨 Frontend: auth, collections, notifications, settings (in progress)
