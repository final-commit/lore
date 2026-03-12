# Phase 0: Foundation Cleanup — Spec

## Goal
Clean the foundation before building. Nothing visible to users. Everything correct beneath the surface.

## Tasks

### 0.1 — Licence Switch (MIT OR Apache 2.0)
- Replace `LICENSE` with MIT OR Apache 2.0 dual licence
- Add `LICENSE-MIT` and `LICENSE-APACHE` files
- Update `Cargo.toml` licence field: `license = "MIT OR Apache-2.0"`
- Update `package.json` in app/: `"license": "MIT OR Apache-2.0"`
- Update README.md licence badge + section

### 0.2 — DbBackend Trait
- Create `server/src/db/backend.rs` with `DbBackend` async trait
- Move all SQLite logic into `SqliteBackend` struct implementing `DbBackend`
- Create `PostgresBackend` scaffold (stub `unimplemented!()` or feature-flag)
- All existing code routes through trait methods
- All 260 tests still pass

### 0.3 — Monorepo Structure
- Current: `server/` (Rust) + `app/` (Next.js) — already correct
- Add: `agents/` directory with README and placeholder Cargo.toml (lib, no code yet)
- Add: `desktop/` directory with README (Tauri, Phase 3)
- Update root README.md to document structure

### 0.4 — Docker in Compose
- Add Docker executor service scaffold to `docker-compose.yml`
  (lore-docker-executor placeholder, commented out — activated in Phase 2)
- Document in README that Docker is required for agent execution capabilities

### 0.5 — CI Green
- Ensure `cargo test` passes (260 tests)
- Ensure `cargo clippy -- -D warnings` passes
- Ensure `pnpm build` in app/ passes
- GitHub Actions workflow passes end-to-end

## Acceptance Criteria
- [ ] `cargo test` — 260 tests pass
- [ ] `cargo clippy -- -D warnings` — zero warnings
- [ ] `pnpm build` — zero errors
- [ ] Licence files present and correct
- [ ] `DbBackend` trait in place, `SqliteBackend` implements it
- [ ] `agents/` and `desktop/` directories created
- [ ] Git committed with clean history
