# Lore

**A self-hosted team wiki that runs on a Raspberry Pi.**

Forge is a full-featured documentation platform — real-time collaborative editing, inline comments, AI writing assistance, and Git-backed history — built in Rust. It uses ~150MB RAM under load. The same workload on Outline (Node.js) uses 1.5GB.

> ⚠️ Early release. Production use at your own risk. Feedback welcome.

---

## Why

Most team wikis are either:
- **Cloud-only** (Notion, Confluence) — your data lives on someone else's server
- **Self-hostable but heavy** (Outline, BookStack) — needs 1-4GB RAM, a VPS that costs £10-20/month minimum
- **Personal** (Obsidian, Logseq) — no real multi-user or permissions model

Forge is self-hostable, genuinely lightweight, and built for teams. It runs comfortably on a Raspberry Pi 4 (4GB) or the cheapest VPS tier (£3-5/month).

---

## Features

| | Forge | Outline | Notion |
|---|---|---|---|
| Self-hosted | ✅ | ✅ | ❌ |
| RAM (100 users) | ~150MB | ~1.5GB | — |
| Real-time collab | ✅ | ✅ | ✅ |
| Git-backed history | ✅ native | ❌ | ❌ |
| Inline comments | ✅ | ✅ | ✅ |
| AI writing assistant | ✅ | ✅ paid | ✅ paid |
| Full-text search | ✅ | ✅ | ✅ |
| Single binary | ✅ | ❌ | — |
| Runs on Pi | ✅ | ❌ | — |

**Core features:**
- 📝 Rich Markdown editor (Tiptap, collaborative via Yjs)
- 📁 Collections with granular permissions (admin / member / viewer)
- 💬 Inline threaded comments with reactions
- 🔀 Real-time presence — see who's editing
- 🔍 Full-text search (Tantivy, embedded, no Elasticsearch)
- 📜 Full document history — view or restore any past version
- ⭐ Stars, pins, subscriptions, notifications
- 🔗 Public share links with optional expiry
- 🤖 AI: suggest improvements, summarise, answer questions, generate from outline (OpenAI-compatible, bring your own key — or disable entirely)
- 🔒 OAuth / SSO (Google + generic OIDC)
- 📤 Export to Markdown or HTML
- 🔄 Git sync — push/pull to GitHub, GitLab, or Gitea
- 📥 Import from Outline JSON or Markdown zip
- 🪝 Outbound webhooks
- 🎨 Custom emoji
- ⌨️ Keyboard shortcuts

---

## Quick start

**Docker (recommended):**

```bash
git clone https://github.com/yourorg/forge
cd forge
cp .env.example .env
# Edit .env — at minimum set LORE_JWT_SECRET
docker compose up -d
```

Open [http://localhost:3000](http://localhost:3000). Register the first account — it gets admin role automatically.

**Binary:**

```bash
# Download the latest release binary for your platform
curl -L https://github.com/yourorg/forge/releases/latest/download/forge-linux-aarch64.tar.gz | tar xz
cp .env.example .env && $EDITOR .env
./forge
```

---

## Configuration

All config via environment variables. Copy `.env.example` to `.env`:

```env
# Required
LORE_JWT_SECRET=generate-with-openssl-rand-hex-32

# Optional: AI (any OpenAI-compatible endpoint)
LORE_AI_API_KEY=sk-...
LORE_AI_BASE_URL=https://api.openai.com/v1
LORE_AI_MODEL=gpt-4o-mini

# Optional: Google OAuth
LORE_OAUTH_GOOGLE_CLIENT_ID=...
LORE_OAUTH_GOOGLE_CLIENT_SECRET=...

# Optional: Git sync remote
LORE_GIT_REMOTE_URL=https://github.com/yourorg/docs.git
LORE_GIT_REMOTE_TOKEN=ghp_...
```

---

## Architecture

```
┌─────────────────────────────────────────┐
│              Forge binary               │
│                                         │
│  ┌──────────┐  ┌────────┐  ┌────────┐  │
│  │  Axum    │  │ SQLite │  │  Git   │  │
│  │  HTTP/WS │  │ (meta) │  │ (docs) │  │
│  └──────────┘  └────────┘  └────────┘  │
│  ┌──────────┐  ┌────────┐              │
│  │  Tantivy │  │  Yjs   │              │
│  │ (search) │  │(collab)│              │
│  └──────────┘  └────────┘              │
└─────────────────────────────────────────┘
```

Single binary. Single SQLite file. Single Git repo. No external services required.

Documents are stored as Markdown files in a Git repository — readable, editable, and portable without Forge installed. Metadata (comments, stars, shares, etc.) lives in SQLite.

---

## Development

```bash
# Prerequisites: Rust 1.75+, Node.js 20+, pnpm

# Backend
cd lore
cargo test        # 257 tests
cargo run         # starts on :3334

# Frontend
cd forge/packages/web
pnpm install
pnpm dev          # starts on :3000, proxies API to :3334
```

---

## Deployment on Raspberry Pi

```bash
# On your Pi (aarch64):
curl -L https://github.com/yourorg/forge/releases/latest/download/forge-linux-aarch64.tar.gz | tar xz
sudo mv forge /usr/local/bin/

# Create systemd service
sudo tee /etc/systemd/system/forge.service << EOF
[Unit]
Description=Lore wiki
After=network.target

[Service]
EnvironmentFile=/etc/forge.env
ExecStart=/usr/local/bin/forge
Restart=on-failure
User=forge

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl enable --now forge
```

Memory usage on idle: ~80MB. Under load (10 concurrent editors): ~150MB.

---

## Roadmap

- [ ] Release binaries for linux/aarch64, linux/amd64, macOS
- [ ] Postgres adapter (for larger teams)
- [ ] Mobile-friendly editor improvements
- [ ] SAML / enterprise SSO
- [ ] Hosted cloud version (if there's demand — [register interest](mailto:forge@daraoui.com))

---

## Licence

[Business Source License 1.1](LICENSE) — free to self-host, commercial hosting requires a licence. Converts to Apache 2.0 on 2029-01-01.

---

## Contributing

PRs welcome. Please open an issue first for significant changes.

Built with: Rust, Axum, SQLite, git2, Tantivy, Yjs/yrs, Next.js, Tiptap, Tailwind.
