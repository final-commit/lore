# Phase 1: Agent Layer — Spec

## Goal
First sellable product. Lore + AI agents that work. Agents are first-class participants.

## Tasks

### 1.1 — wasmtime Host in lore-server
- Add `wasmtime` to server/Cargo.toml dependencies
- Create `server/src/agents/runtime.rs` — WasmRuntime struct
  - `WasmRuntime::new()` — initialise wasmtime Engine with safe defaults
  - `WasmRuntime::load(wasm_bytes: &[u8]) -> Result<AgentInstance>`
  - `AgentInstance::call(fn_name, args) -> Result<Value>`
  - Fuel metering enabled (prevent infinite loops)
  - Memory limits: 64MB per agent instance
- Create `server/src/agents/mod.rs` — module root

### 1.2 — Capability Model
- Create `server/src/agents/capabilities.rs`
- Capabilities enum: ReadDocs, WriteDocs, ReadMessages, WriteMessages, HttpOutbound, ExecuteCode, BrowseWeb, ShellRun, CreateIssues, ReadUsers
- `CapabilitySet` — set of granted capabilities for an agent
- `CapabilityGuard` — enforces caps at call time, returns CapabilityDenied if not granted
- All WASM host functions check CapabilityGuard before executing

### 1.3 — Agent Manifest Format (TOML)
- Create `agents/sdk/manifest.rs` (parse) + `agents/sdk/manifest_schema.toml` (example)
- Fields: name, version, description, capabilities[], trigger_patterns[], wasm_hash
- `AgentManifest::from_toml(str) -> Result<AgentManifest>`
- Validate: wasm_hash matches loaded WASM bytes, capabilities are known values

### 1.4 — Database Tables for Agents
- Add migrations:
  - `agents` — id, name, wasm_hash, capabilities[], description, manifest_toml, enabled, created_at
  - `agent_runs` — id, agent_id, triggered_by, status (pending/running/complete/failed), input, output, started_at, completed_at
  - `agent_approvals` — id, agent_id, capability, granted_by, granted_at
- Wire into DbBackend trait (new methods: install_agent, list_agents, create_agent_run, update_agent_run)

### 1.5 — Agent Task Queue
- Create `server/src/agents/queue.rs`
- Async task queue using tokio channels
- AgentJob { run_id, agent_id, input, capabilities }
- Worker pool: 4 workers by default (configurable via LORE_AGENT_WORKERS)
- Streaming output via broadcast channel → WebSocket → UI

### 1.6 — Built-in Agents (compiled to WASM)
Create in `agents/builtin/`:

**docs-agent** (`agents/builtin/docs-agent/`):
- Reads a doc + context window
- Answers questions about it
- Suggests improvements (returned as diff proposal)
- Capabilities: ReadDocs

**summarise-agent** (`agents/builtin/summarise-agent/`):
- Summarises a doc or collection
- Posts summary as a comment
- Capabilities: ReadDocs, WriteMessages (to post result)

**search-agent** (`agents/builtin/search-agent/`):
- Semantic search across workspace
- Surfaces relevant docs for a query
- Capabilities: ReadDocs

Each agent: Rust crate compiled to `wasm32-wasip1`, calls OpenAI-compatible API via HttpOutbound.
Graceful degradation if no AI key configured (LORE_AI_API_KEY).

### 1.7 — @agent-name Trigger in Comments
- When a comment is created, check if body contains `@agent-name`
- If matched: look up agent by name, check enabled + has WriteMessages cap
- Enqueue AgentJob with comment context
- Agent reads thread + doc → posts reply comment
- Agent committer identity: `{agent-name}@lore` in git

### 1.8 — Human-in-the-Loop (Diff Approval)
- Agent can propose a document edit as a diff
- Stored as pending AgentRun with output = diff patch
- UI shows diff, user approves → commit applied with agent git identity
- User rejects → run marked declined, no changes

### 1.9 — REST API for Agents
- `GET /api/agents` — list installed agents
- `POST /api/agents` — install agent (upload WASM + manifest)
- `DELETE /api/agents/:id` — uninstall
- `POST /api/agents/:id/enable` / `disable`
- `GET /api/agents/:id/runs` — list runs
- `GET /api/agents/runs/:run_id` — get run status/output
- `POST /api/agents/:id/capabilities` — grant capability (admin only)

### 1.10 — Agent Management UI (app/)
- `/settings/agents` page — list installed agents, enable/disable toggle
- Agent detail page — capabilities granted, recent runs, run history
- Agent run viewer — streaming output, approve/reject diff proposals
- Activity feed in sidebar — recent agent actions (last 10 runs)
- Install agent modal — upload WASM + manifest TOML

### 1.11 — Tests
- Unit tests: WasmRuntime load + call, CapabilityGuard enforcement, AgentManifest parsing
- Integration tests: install agent → trigger via comment → check run created → check output
- E2E: full flow with mock WASM agent
- All existing 263 tests must still pass

## Acceptance Criteria
- [ ] wasmtime integrated, agent loads and executes WASM
- [ ] Capability model enforced — CapabilityDenied if not granted
- [ ] 3 built-in agents compiled to WASM and installable
- [ ] @agent-name trigger in comments works end-to-end
- [ ] Human-in-the-loop diff approval works
- [ ] Agent management UI complete
- [ ] cargo test — all tests pass (263 + new agent tests)
- [ ] cargo clippy -- -D warnings — zero warnings
- [ ] pnpm build — zero errors
- [ ] git commit: "feat: Phase 1 — agent layer (wasmtime, capabilities, built-in agents)"
