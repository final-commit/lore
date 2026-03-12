# agents — Lore Agent SDK

**Phase 1 (weeks 3–10).** This crate is a scaffold. No agent code exists yet.

## What will live here

- **Agent SDK (Rust)** — the library that agent authors link against.
  Defines the capability model, WASI host functions, and the agent manifest format.
- **Built-in agents** — first-party agents compiled to WASM and embedded in the `lore` binary:
  - `doc-summariser` — summarises changed documents on commit
  - `issue-triager` — categorises and labels new issues
  - `decision-recorder` — monitors discussions and writes to DECISIONS.md
  - `cron-agent` — runs scheduled tasks (digest emails, stale-doc alerts)

## Architecture (planned)

```
agents/
├── sdk/         # Agent SDK crate (Rust lib, published to crates.io in Phase 8)
├── built-in/    # First-party agents (compiled to .wasm, embedded in lore binary)
│   ├── doc-summariser/
│   ├── issue-triager/
│   └── decision-recorder/
└── examples/    # Sample third-party agents for documentation
```

## Runtime

Agents run inside **wasmtime** (Bytecode Alliance reference implementation).
See `DECISIONS.md` D-002 for the rationale.

Key properties:
- Sandboxed: each agent runs in an isolated WASM module
- Capability-gated: agents declare which Lore APIs they need in their manifest
- Metered: CPU and memory limits enforced via wasmtime fuel/epochs
- Auditable: every agent action is written to the Lore audit log

## WASI interface (planned)

Agents communicate with the Lore host via WASI Preview 2 component interfaces:

- `lore:docs/read` — read document content and metadata
- `lore:docs/write` — create or update documents
- `lore:events/subscribe` — receive workspace events
- `lore:comments/post` — post comments as the agent identity
- `lore:ai/complete` — call the configured AI provider

## Getting started (Phase 1)

Documentation and examples will be added when the SDK is implemented.
Track progress in the [Phase 1 milestone](https://github.com/final-commit/lore/milestones).
