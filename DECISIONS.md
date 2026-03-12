# DECISIONS.md — Lore Platform Decision Log

Format: **1-3-1** — 1 problem, 3 options, 1 recommendation.

---

## D-001 — MLS Crate: openmls vs mls-rs (2026-03-12)

**Problem:** Two production-ready Rust MLS (RFC 9420) implementations exist. Which to use for Phase 3?

**Options:**
1. **openmls** — Phoenix R&D / Cryspen, v0.7 (Jul 2025), reference implementation, battle-tested academic backing, active issues but no blockers. OpenSSL/AWS-LC stable cipher suites. Experimental RustCrypto/WebCrypto.
2. **mls-rs** — AWS-backed, 37 releases to v0.51.0 (Sep 2025), 100% RFC conformance, WASM + SQLite storage built-in, very active. More opinionated API.
3. **Custom MLS subset** — build only the group messaging primitives we need. Fast for Phase 3 but creates long-term security debt; not recommended for E2E.

**Recommendation: mls-rs** ✅
- AWS backing = long-term maintenance confidence
- Built-in SQLite storage adapter aligns with our default DB
- 100% RFC conformance + WASM support (future mobile)
- More releases = more battle-tested in production
- Switch cost is low if openmls becomes dominant later

**Status:** Adopted. Use in Phase 3.

---

## D-002 — WASM Runtime: wasmtime vs wasmer (2026-03-12)

**Problem:** Which WASM runtime to embed in lore-server for the agent brain?

**Options:**
1. **wasmtime** — Bytecode Alliance reference implementation, full WASI Preview 2, Component Model, leading standards compliance, best production security/sandbox story, async Rust native, fuel/epochs metering.
2. **wasmer** — Good portability, but uses non-standard WASIX extension, lags on WASI Preview 2, cloud-account dependency in some deployment models.
3. **Custom WASM sandbox** — not viable without significant investment.

**Recommendation: wasmtime** ✅
- Already in NORTH_STAR_ARCHITECTURE.md decision
- Validated: best production security, async-native, Component Model support
- No cloud dependency, pure embed
- WASI Preview 2 aligns with Agent SDK design

**Status:** Adopted. Use in Phase 1.

---

## D-003 — Licence: MIT vs Apache 2.0 vs MIT/Apache dual (2026-03-12)

**Problem:** NORTH_STAR_ARCHITECTURE.md says MIT/Apache 2.0, current repo has BSL 1.1. Which licence to switch to?

**Options:**
1. **MIT only** — Maximum permissiveness, simplest. Best for r/selfhosted + HN distribution. No patent grant clause.
2. **Apache 2.0 only** — Patent grant included, better for enterprise procurement. Slightly more complex.
3. **MIT OR Apache 2.0 dual licence** — Rust ecosystem standard (rust-lang itself does this). Users choose. Maximises compatibility with both MIT and GPL-2 projects.

**Recommendation: MIT OR Apache 2.0 (dual)** ✅
- Standard for serious Rust open source projects
- Patent grant (Apache) satisfies enterprise legal teams
- MIT permissiveness maximises community adoption
- Used by Axum, Tokio, rust-lang — credibility signal

**Status:** Adopted. Switch in Phase 0.

---

## D-004 — DbBackend Trait: async_trait vs native async trait (2026-03-12)

**Problem:** How to define the `DbBackend` async trait that SqliteBackend and PostgresBackend implement?

**Options:**
1. **`async_trait` macro** — battle-tested, works everywhere, adds Box<dyn Future> overhead.
2. **Native async trait (Rust 1.75+)** — stable since Dec 2023, zero overhead, requires `Send` bound management.
3. **Return-position impl Trait (RPIT)** — not yet object-safe for trait objects, can't use with `dyn DbBackend`.

**Recommendation: Native async trait** ✅
- Rust 1.75+ is over a year old; minimum supported Rust version can be set to 1.75
- Zero overhead vs async_trait's allocations per call
- Cleaner code, no macro dependency
- `Send` bounds manageable with `#[trait_variant::make]` or manual bounds

**Status:** Adopted. Use in Phase 0 DbBackend trait.

---
