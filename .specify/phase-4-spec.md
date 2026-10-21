# Phase 4: Browser Automation for Agents — Spec

## Goal
Full OpenClaw capability parity. Agents can browse, extract, fill forms, automate web workflows.

## Tasks

### 4.1 — Browser Pool
- Add `playwright` via Node.js bridge or use `chromiumoxide` Rust crate
- Recommendation: `chromiumoxide` (pure Rust, CDP protocol, no Node dependency)
- `server/src/execution/browser_pool.rs` — BrowserPool
- Pre-warm 2 headless Chromium instances at startup
- Session isolation: each agent run gets exclusive browser session
- Session lifecycle: open → operations → close → return to pool

### 4.2 — browse:web Capability
Agent API (via WASM host functions):
- `browser.navigate(url: &str) -> Result<()>`
- `browser.screenshot() -> Result<Vec<u8>>` (PNG bytes)
- `browser.click(selector: &str) -> Result<()>`
- `browser.fill(selector: &str, text: &str) -> Result<()>`
- `browser.extract(selector: &str) -> Result<String>`
- `browser.evaluate(js: &str) -> Result<String>`
- `browser.wait_for(selector: &str, timeout_ms: u64) -> Result<()>`

### 4.3 — Web Search Tool
- Wrap Brave Search API (or DuckDuckGo HTML scrape as fallback)
- `web_search(query: &str, count: u8) -> Vec<SearchResult>`
- SearchResult { title, url, snippet }
- Available to agents with HttpOutbound capability
- Rate limited: 10 searches/minute per agent

### 4.4 — Streaming Screenshots to UI
- Agent browser session: screenshots streamed via WebSocket
- UI shows live browser view in agent run panel
- "Agent is browsing..." indicator with current URL
- User can observe but not interact (read-only view)

### 4.5 — Security
- Chromium sandbox enabled (not running as root)
- Network: by default only HTTPS allowed, no internal network (block 10.x, 192.168.x, 127.x)
- `http:outbound` capability must declare allowed domains (glob patterns)
- Session timeout: 5 minutes max per browse session
- No persistent cookies or localStorage between runs (ephemeral profile)

### 4.6 — Tests
- Unit: browser pool acquire/release
- Integration: navigate to example.com → screenshot → verify PNG
- Integration: fill form → submit → extract result
- Integration: domain blocklist enforcement
- All existing tests pass

## Acceptance Criteria
- [ ] Browser pool pre-warms at startup
- [ ] Agent can navigate, screenshot, click, fill, extract via host functions
- [ ] Live browser view streamed to UI
- [ ] Domain restrictions enforced
- [ ] Web search tool available to agents
- [ ] All tests pass
- [ ] git commit: "feat: Phase 4 — browser automation for agents (Playwright/chromiumoxide pool)"
