# Phase 8: Agent Marketplace — Spec

## Goal
Distribution flywheel. Turns users into a platform.

## Tasks

### 8.1 — Agent Registry Backend
- `registry_agents` table: id, name, publisher_id, wasm_hash, manifest, version, description, capabilities[], downloads, rating, is_featured, is_verified, published_at
- `registry_agent_versions` — version history per agent
- Signed WASM artifacts: publisher signs with Ed25519 keypair, server verifies on install

### 8.2 — Publish Flow
- Publisher registers keypair: `POST /api/marketplace/publishers`
- Publish agent: `POST /api/marketplace/agents` — upload WASM + manifest + signature
- Server: verify signature, scan capabilities declaration, store in registry
- Review queue for new publishers (admin approval before listing)

### 8.3 — Discovery API
- `GET /api/marketplace/agents` — list with filter (capability, verified, featured)
- `GET /api/marketplace/agents/:id` — agent detail + versions
- `GET /api/marketplace/agents/search?q=` — search by name/description
- Featured section, category tags, most installed

### 8.4 — Install / Update / Uninstall
- One-click install: `POST /api/marketplace/agents/:id/install` — downloads WASM, verifies hash, installs
- Update: check for new version, show changelog, install
- Uninstall: `DELETE /api/agents/:id` (existing endpoint)
- Version pinning: lock to specific version
- Rollback: revert to previous installed version

### 8.5 — Marketplace UI (app/)
- `/marketplace` — browse agents
- Featured carousel, category filters, search
- Agent detail page: description, capabilities required, version history, install count, rating
- One-click install button
- Installed agents manager: updates available badge

### 8.6 — Paid Agents
- Price field on registry_agents (null = free)
- Payment: Stripe integration (hosted payment link for simplicity)
- Licence key model: purchase → receive licence key → install validates key
- Revenue share: 70% publisher, 30% platform (tracked in platform DB)

### 8.7 — Publisher Analytics
- `GET /api/marketplace/publishers/me/analytics` — install count, run count, capability usage (aggregated, no PII)
- Published agents list with version history

### 8.8 — Agent SDK Documentation
- `agents/sdk/docs/` — comprehensive SDK docs
- How to: create agent, declare capabilities, use host functions, compile to WASM
- Examples in Rust (canonical), Go, Python (via WASM targets)
- `agents/sdk/examples/` — hello-world, http-fetch, doc-reader, code-runner

### 8.9 — Tests
- Publish agent → install → run (end-to-end with real WASM)
- Signature verification (reject tampered WASM)
- Version update flow
- All existing tests pass

## Acceptance Criteria
- [ ] Publish flow with signature verification
- [ ] Discovery API + marketplace UI
- [ ] One-click install/update/uninstall
- [ ] Paid agent licence key flow (Stripe)
- [ ] Publisher analytics
- [ ] SDK examples compile to WASM and run
- [ ] All tests pass
- [ ] git commit: "feat: Phase 8 — agent marketplace (registry, install, paid agents, SDK docs)"
