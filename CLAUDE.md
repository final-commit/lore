# CLAUDE.md — Lore Platform: AI-Native Team OS

You are building **Lore** — an AI-native, self-hosted team OS. Single Rust binary.
Docs + Messaging + Agents + Code + Projects. E2E encrypted. Agents are first-class participants.

## North Star

A workspace where AI agents proactively maintain docs, triage issues, summarise decisions,
and act on behalf of teams — with every action visible, auditable, and reversible.

## Repository Layout

```
~/lore-platform/lore/
├── server/          # Rust backend (Axum 0.8, SQLite/Postgres, git2, Tantivy)
├── app/             # Next.js 16 + React 19 + Tiptap v3 + Tailwind 4 frontend
├── agents/          # Agent SDK (Rust) + built-in agents (compiled to WASM)
├── desktop/         # Tauri desktop app (added in Phase 3)
├── docs/            # Architecture + specs
├── .specify/        # Speckit spec-driven development artifacts
└── CLAUDE.md        # This file
```

## Current State (Phase 0 in progress)

Backend: 260 tests, 97 API routes, full Outline parity. Shipped to GitHub.
Frontend: Next.js, full feature parity with Outline.
**Next: Phase 0 cleanup → Phase 1 (Agent Layer).**

## Tech Stack

- **Backend:** Rust, Axum 0.8, rusqlite, git2, Tantivy, moka, Argon2+JWT
- **Frontend:** Next.js 16, React 19, Tiptap v3, Tailwind 4
- **Agent runtime:** wasmtime (WASM host), Docker (execution layer)
- **E2E encryption:** openmls (RFC 9420 / MLS) — Phase 3
- **DB:** SQLite default (`DbBackend` trait), PostgresBackend scaffold now, full in Phase 5
- **Auth:** JWT + OIDC (Google, generic) + SAML (Phase 5) + WebAuthn (Phase 5)

## Key Conventions

- `with_conn` pattern: `.map_err(AppError::Db)` is already inside — don't add it after `.await`
- Axum 0.8: routes use `{param}` not `:param`; wildcards use `{*param}`
- Auth response: `access_token` not `token`
- Env prefix: `LORE_` throughout
- Binary name: `lore`, DB: `lore.db`
- ZipFile `!Send`: collect entries into `Vec<(String, String)>` before any `.await`
- Tests: run `cargo test` to verify — must stay green at all times

## Phase Roadmap

| Phase | Focus | Weeks |
|-------|-------|-------|
| 0 | Foundation cleanup (MIT, DbBackend trait, monorepo, Docker) | 1-2 |
| 1 | Agent layer (wasmtime, capability model, built-in agents) | 3-10 |
| 2 | Docker execution layer (execute:code, shell:run) | 11-16 |
| 3 | E2E Messaging (MLS, channels, threads, Tauri desktop) | 17-26 |
| 4 | Browser automation for agents (Playwright pool) | 27-32 |
| 5 | Enterprise foundation (Postgres, SAML, S3, HA, audit log) | 33-40 |
| 6 | Code engine (repo browser, symbol search, code agents) | 41-48 |
| 7 | Projects (issues, kanban, milestones, GitHub sync) | 49-54 |
| 8 | Agent marketplace | 55-62 |
| 9 | Mobile (React Native, iOS + Android) | 63-74 |
| 10 | Proactive agents (event-driven, scheduled, agent-to-agent) | ongoing |

## Decision Log

See `DECISIONS.md` at repo root. Use 1-3-1 format: 1 problem, 3 options, 1 recommendation.

## Development Rules

1. **Tests first, then implement.** Never ship without tests passing.
2. **Research with web search before every non-trivial decision.**
3. **Log decisions in DECISIONS.md** — use 1-3-1 format.
4. **Commit regularly** — logical commits with clear messages.
5. **Never break existing tests** — 260 must always pass.
6. **Run `cargo clippy` before committing** — zero warnings.
7. **Document architecture changes** in `docs/architecture/`.

## Notify on Completion

When a phase or major milestone is complete, run:
```
openclaw system event --text "Lore: [what was completed]" --mode now
```
