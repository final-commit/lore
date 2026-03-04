# Changelog

All notable changes to Lore are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Full Outline feature parity (97 API routes, 257 tests)
- Rust/Axum backend — ~150MB RAM vs Node.js ~1.5GB
- Real-time collaborative editing via Yjs WebSocket
- Inline threaded comments with reactions
- Git-backed document storage (every edit is a commit)
- Full-text search (Tantivy, embedded)
- Collections with per-user permissions
- Stars, pins, views, subscriptions, notifications
- Public share links with optional expiry
- Document history — view or restore any past version
- OAuth/SSO: Google + generic OIDC
- AI writing assistant (OpenAI-compatible, optional)
- URL unfurling with embed support
- Custom emoji
- Async export jobs (Markdown, HTML)
- Import from Outline JSON or Markdown zip
- Outbound webhooks
- Team settings, user preferences, keyboard shortcuts
- User management: invite, suspend/activate, role change
- Docker Compose single-command deploy
- GitHub Actions CI
