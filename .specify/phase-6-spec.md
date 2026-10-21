# Phase 6: Code Engine — Spec

## Goal
Agents with full codebase context. "Docs stay in sync with code" story.

## Tasks

### 6.1 — Repository Browser
- `GET /api/repositories` — list connected repos
- `POST /api/repositories` — connect repo (path or remote URL)
- `GET /api/repositories/:id/tree?path=` — file tree
- `GET /api/repositories/:id/file?path=` — file content (with syntax highlight metadata)
- `GET /api/repositories/:id/blame?path=&line=` — git blame
- `GET /api/repositories/:id/log?path=` — git log for file/repo
- `GET /api/repositories/:id/diff?from=&to=` — diff between commits/branches

### 6.2 — Code Search
- Extend Tantivy index for code: language-aware tokenisation
- Index: file path, symbol names, function bodies, comments
- `GET /api/search/code?q=&repo=&lang=` — full-text code search
- Symbol search: `GET /api/search/symbols?name=&kind=function|class|type&repo=`
- Unified search across docs + messages + code + issues

### 6.3 — Symbol Index
- `code_symbols` table: repo_id, file_path, symbol_name, kind, line_number, signature
- Tree-sitter integration for parsing: Rust, Python, JavaScript, TypeScript, Go
- Background job: index repos on connect + on push webhook
- `GET /api/repositories/:id/symbols?file=` — symbols in file

### 6.4 — Diff Viewer UI
- Inline diff view (unified format)
- Side-by-side diff view
- Syntax highlighted
- Linked to commits (from git log) and linked issues

### 6.5 — Webhook Ingestion
- `POST /api/webhooks/github` — GitHub push + PR events
- `POST /api/webhooks/gitlab` — GitLab push + MR events
- `POST /api/webhooks/gitea` — Gitea push events
- On push: trigger index update, trigger registered agents
- On PR open: trigger pr-summariser agent if installed
- Webhook secret validation (HMAC-SHA256)

### 6.6 — Agent Code Capabilities
- `code.read_file(repo_id, path) -> String` — requires ReadCode cap
- `code.search(query) -> Vec<SearchResult>` — requires ReadCode cap
- `code.run(language, code) -> Stream<Output>` — requires ExecuteCode cap (Docker)
- `code.list_symbols(repo_id, file) -> Vec<Symbol>` — requires ReadCode cap

### 6.7 — Built-in Code Agents
**pr-summariser** (`agents/builtin/pr-summariser/`):
- Trigger: PR opened webhook
- Reads diff, writes structured summary doc in linked collection
- Posts summary link to #engineering channel
- Capabilities: ReadCode, WriteDocs, WriteMessages

**doc-freshness** (`agents/builtin/doc-freshness/`):
- Trigger: push to main branch
- Checks if docs referencing changed files are stale (last updated > changed)
- Flags stale docs (comment + notification) or auto-updates (if WriteDocs granted)
- Capabilities: ReadCode, ReadDocs, WriteDocs, WriteMessages

**code-reviewer** (`agents/builtin/code-reviewer/`):
- Trigger: PR opened webhook
- Reviews diff, posts structured review comment on PR (via GitHub API)
- Capabilities: ReadCode, HttpOutbound (github.com)

### 6.8 — Tests
- Repository connect + file tree + content
- Code search returns results
- Symbol index populated after index job
- Webhook received → agent triggered
- All existing tests pass

## Acceptance Criteria
- [ ] Repository browser (tree, file content, blame, log, diff)
- [ ] Code search + symbol search working
- [ ] Tree-sitter symbol indexing for 5 languages
- [ ] GitHub/GitLab/Gitea webhooks trigger agents
- [ ] 3 built-in code agents work end-to-end
- [ ] All tests pass
- [ ] git commit: "feat: Phase 6 — code engine (repo browser, symbol search, code agents)"
