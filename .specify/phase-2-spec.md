# Phase 2: Docker Execution Layer — Spec

## Goal
Agents that can run code. "AI that actually does things."

## Tasks

### 2.1 — Docker Daemon Integration
- Add `bollard` crate (Docker API client for Rust) to server/Cargo.toml
- Create `server/src/execution/docker.rs` — DockerExecutor struct
- `DockerExecutor::new()` — connect to Docker socket, verify daemon running
- Fail-fast at startup if Docker unavailable (log warning, disable execute:code cap)
- Never expose Docker socket to agents — all operations go through DockerExecutor

### 2.2 — Container Pool
- `server/src/execution/pool.rs` — ContainerPool
- Pre-warm N containers at startup (default: 2 Python, 2 Node.js, 1 shell)
- Configurable via LORE_CONTAINER_POOL_SIZE
- Container images: `python:3.12-slim`, `node:22-alpine`, `alpine:3.19`
- Pool assignment: `pool.acquire(language) -> ContainerHandle` (~10ms)
- Auto-replenish: when container returned, clean and re-warm

### 2.3 — execute:code Capability
- Agent calls: `execute_code(language: &str, code: &str) -> Stream<OutputChunk>`
- Languages: python, javascript, shell
- Each run: ephemeral Docker volume, resource limits enforced
  - CPU: 0.5 core
  - Memory: 512MB
  - Timeout: 60s (configurable per agent)
  - Network: blocked by default (unless http:outbound declared)
- stdout/stderr streamed via capability channel → agent → WebSocket → UI
- Exit code captured, returned to agent

### 2.4 — shell:run Capability
- Agent calls: `shell_run(command: &str) -> Stream<OutputChunk>`
- Runs in alpine container
- Same resource limits as execute:code
- Separate capability — must be explicitly granted

### 2.5 — Ephemeral Filesystem
- Each agent run gets a Docker volume: `lore-run-{run_id}`
- Mounted at `/workspace` in container
- Files created by agent persist for run duration
- Volume destroyed on run completion (`docker volume rm`)

### 2.6 — Streaming Architecture
- OutputChunk { kind: Stdout|Stderr|ExitCode, data: String, timestamp }
- Streamed through AgentRun broadcast channel
- WebSocket handler fans out to subscribed UI clients
- UI shows real-time streaming output in agent run viewer

### 2.7 — Agent Run History
- Full input/output log stored in agent_runs table
- Resource usage: peak memory, CPU time, wall clock duration
- Truncate output at 1MB (store truncation marker)

### 2.8 — Tests
- Unit: container pool acquire/release, resource limit config
- Integration: execute Python "print('hello')" → verify stdout streamed + captured
- Integration: shell:run "echo test" → verify output
- Integration: timeout enforcement (run sleep 300 with 5s timeout → verify killed)
- Integration: network blocked (curl google.com without http:outbound → verify fail)
- All existing tests pass

## Acceptance Criteria
- [ ] Docker integration works (bollard)
- [ ] Container pool pre-warms at startup
- [ ] execute:code streams output end-to-end
- [ ] shell:run works
- [ ] Resource limits enforced (CPU, memory, timeout, network)
- [ ] Ephemeral volumes cleaned up after runs
- [ ] All tests pass
- [ ] git commit: "feat: Phase 2 — Docker execution layer (execute:code, shell:run)"
