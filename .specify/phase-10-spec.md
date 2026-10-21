# Phase 10: Proactive Agents — Spec

## Goal
The actual north star. Agents that act without being asked.

## Tasks

### 10.1 — Event-Driven Agent Triggers
- Any workspace event can trigger an agent
- `agent_triggers` table: id, agent_id, event_type, filter (JSON), enabled
- Event types: message.created, doc.updated, code.pushed, issue.created, external.webhook, schedule.cron
- On event: match against registered triggers → enqueue matching AgentJobs
- Filter: JSONPath expressions against event payload

### 10.2 — Scheduled Agents (Cron)
- `agent_schedules` table: id, agent_id, cron_expr, last_run, next_run, enabled
- Cron parser: `cron` crate
- Scheduler: tokio task checks every minute, enqueues due jobs
- Examples: every Monday 9am → post weekly summary to #general

### 10.3 — Agent-to-Agent (Sub-tasks)
- WASM host function: `agent.invoke(agent_name, input) -> JobHandle`
- Spawns child AgentRun linked to parent run
- Child run: same capability enforcement, separate resource limits
- Parent can await child result or fire-and-forget
- Cycle detection: max depth 5, reject circular invocations

### 10.4 — Organisational Memory
- `agent_memory` table: agent_id, key, value (text), embedding (vector), created_at, updated_at
- Embeddings via OpenAI-compatible embeddings API (or local model if LORE_EMBEDDINGS_LOCAL=true)
- `memory.store(key, value)` — WASM host function
- `memory.search(query, top_k) -> Vec<MemoryEntry>` — vector similarity search
- `memory.recall(key) -> Option<String>` — exact key lookup
- SQLite vec extension (sqlite-vec) for vector storage

### 10.5 — Multi-Step Autonomous Tasks
- Agent plans sequence of tool calls via reasoning step
- Checkpoint: after N steps (configurable), pause and surface to human for review
- Human can: approve (continue), modify (edit plan), reject (stop)
- Full step-by-step audit trail in agent_runs output
- Streaming: each step streamed to UI as it executes

### 10.6 — Agent Personas
- Agents have: name, avatar URL, description, status
- Appear in sidebar "Agents" section alongside team members
- Show online/idle based on recent activity
- Mention via @agent-name anywhere (messages, docs, issues)
- Activity feed: agent actions visible in workspace activity stream

### 10.7 — Weekly Summary Agent (Built-in)
- `agents/builtin/weekly-summary/`
- Trigger: cron (every Monday 9am, configurable)
- Reads: all docs updated + messages posted + issues closed in past week
- Writes: structured weekly summary doc + posts to #general
- Capabilities: ReadDocs, ReadMessages, WriteDocs, WriteMessages, CreateIssues

### 10.8 — Tests
- Event trigger: create doc → agent triggered → verify run created
- Cron: mock clock advance → verify scheduled agent runs
- Agent-to-agent: agent A invokes agent B → verify both runs in DB
- Memory: store + recall + similarity search
- Multi-step: run 3-step plan → verify checkpoint pause
- All existing tests pass

## Acceptance Criteria
- [ ] Event-driven triggers work for all event types
- [ ] Scheduled agents (cron) run on schedule
- [ ] Agent-to-agent invocation works (max depth 5)
- [ ] Organisational memory: store, recall, semantic search
- [ ] Multi-step tasks with human checkpoint
- [ ] Agent personas visible in sidebar
- [ ] Weekly summary agent works end-to-end
- [ ] All tests pass
- [ ] git commit: "feat: Phase 10 — proactive agents (event-driven, scheduled, agent-to-agent, memory)"
