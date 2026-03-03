# Forge API Reference

Base URL: `http://localhost:3000` (or your deployed URL)

## Authentication

All endpoints (except `/health`, `/api/installation`, `/api/auth/*`, and public share URLs) require a Bearer token:

```
Authorization: Bearer <access_token>
```

Tokens are obtained from `/api/auth/login` or `/api/auth/register`.

### Token Types
- **Access token** — short-lived JWT (1 hour), for API calls
- **Refresh token** — long-lived JWT (30 days), for getting new access tokens
- **API token** — never-expiring (or admin-configured expiry), for CI/agents

---

## Auth

### Register
`POST /api/auth/register`

First user becomes admin. Subsequent users get `editor` role.

```json
{ "email": "user@example.com", "name": "Alice", "password": "min8chars" }
```

Response: `{ "access_token": "...", "refresh_token": "...", "user": { ... } }`

### Login
`POST /api/auth/login`

```json
{ "email": "user@example.com", "password": "..." }
```

### Refresh
`POST /api/auth/refresh`

```json
{ "refresh_token": "..." }
```

### Me
`GET /api/auth/me`

Returns current user info.

### Create API Token
`POST /api/auth/tokens` (admin)

```json
{ "name": "CI Bot", "scope": "read", "expires_at": "2027-01-01T00:00:00Z" }
```

### Revoke API Token
`DELETE /api/auth/tokens/{id}` (admin)

---

## Documents

### Get Document Tree
`GET /api/tree`

Returns recursive file tree from git repository.

### Get Document
`GET /api/docs/{path}`

Returns document content and metadata.

```json
{
  "path": "guide/getting-started.md",
  "content": "# Getting Started\n\n...",
  "sha": "abc123",
  "commit_sha": "def456"
}
```

### Create Document
`POST /api/docs`

```json
{
  "path": "guide/new-page.md",
  "content": "# New Page\n\nContent here.",
  "message": "Create new page"
}
```

### Update Document
`PUT /api/docs/{path}`

```json
{ "content": "# Updated Content\n\n...", "message": "Update page" }
```

### Delete Document
`DELETE /api/docs/{path}` (admin)

### Document History
`GET /api/docs-history/{path}`

Returns git log for the file (up to 50 commits).

### Publish / Unpublish
`POST /api/doc-publish/{path}` — Publish a draft

`POST /api/doc-unpublish/{path}` — Move back to draft

### Trash / Restore
`POST /api/doc-trash/{path}` — Soft delete (30-day auto-purge)

`POST /api/doc-restore/{path}` — Restore from trash

`DELETE /api/doc-delete/{path}` — Permanent delete (admin)

### Archive
`POST /api/doc-archive/{path}` — Archive document

`POST /api/doc-unarchive/{path}` — Unarchive

### List Trash / Archive
`GET /api/trash` — List trashed documents

`GET /api/archive` — List archived documents

### List Drafts
`GET /api/drafts` — Current user's unpublished documents

---

## Collections

Collections organize documents into named groups (backed by git directories).

### List Collections
`GET /api/collections`

### Create Collection
`POST /api/collections`

```json
{ "name": "Engineering", "slug": "engineering", "description": "Technical docs", "icon": "⚙️", "color": "#3B82F6" }
```

### Update Collection
`PUT /api/collections/{id}`

### Delete Collection
`DELETE /api/collections/{id}` (admin)

---

## Comments

### List Comments
`GET /api/comments?doc_path=guide/getting-started.md`

### Create Comment
`POST /api/comments`

```json
{
  "doc_path": "guide/getting-started.md",
  "body": "This section needs more detail.",
  "parent_id": null,
  "anchor_text": "Getting Started",
  "anchor_start": 2,
  "anchor_end": 17
}
```

### Update Comment
`PUT /api/comments/{id}`

```json
{ "body": "Updated comment text" }
```

### Delete Comment
`DELETE /api/comments/{id}` (owner or admin)

### Resolve Thread
`POST /api/comments/{id}/resolve`

---

## Search

`GET /api/search?q=getting+started&limit=20`

Returns documents matching the query with BM25 relevance scoring and text snippets.

---

## Stars (Bookmarks)

`POST /api/stars` — Star/unstar a document (toggle)

```json
{ "doc_path": "guide/getting-started.md" }
```

`GET /api/stars` — List current user's starred documents

`DELETE /api/stars/{id}` — Unstar

---

## Pins

Pin documents to a collection for quick access.

`POST /api/pins` — `{ "doc_path": "...", "collection_id": "..." }`

`GET /api/pins?collection_id={id}` — List pins for collection

`PUT /api/pins/{id}` — `{ "sort_order": 1 }` — Reorder

`DELETE /api/pins/{id}` — Unpin

---

## Views

`POST /api/views` — Record a document view `{ "doc_path": "..." }`

`GET /api/views?doc_path=` — List viewers for a document

`GET /api/views/recent` — Current user's recently viewed documents

---

## Shares (Public Links)

`POST /api/shares` — `{ "doc_path": "...", "include_child_docs": false }`

`GET /api/shares?doc_path=` — List shares for document

`DELETE /api/shares/{id}` — Revoke

`GET /api/shares/view/{url_id}` — **No auth required** — Public view of shared document

---

## Templates

`GET /api/templates` — List templates

`POST /api/templates` — `{ "title": "Meeting Notes", "content": "## Agenda\n\n..." }`

`PUT /api/templates/{id}` — Update

`DELETE /api/templates/{id}` — Delete

---

## Attachments

`POST /api/attachments/upload` — Multipart form upload (`file` field)

`GET /api/attachments/{id}` — Download attachment

---

## Groups

`GET /api/groups` — List groups

`POST /api/groups` — `{ "name": "Engineering", "description": "..." }`

`PUT /api/groups/{id}` — Update

`DELETE /api/groups/{id}` — Delete

`GET /api/groups/{id}/members` — List members

`POST /api/groups/{id}/members` — `{ "user_id": "..." }` — Add member

`DELETE /api/groups/{id}/members/{user_id}` — Remove member

---

## Memberships (Collection Permissions)

`GET /api/memberships?collection_id={id}` — List memberships

`POST /api/memberships` — `{ "user_id": "...", "collection_id": "...", "permission": "read|write|admin" }`

`PUT /api/memberships/{id}` — Update permission

`DELETE /api/memberships/{id}` — Remove

---

## Notifications

`GET /api/notifications` — List current user's notifications (unread first)

`POST /api/notifications/{id}/read` — Mark as read

`POST /api/notifications/read-all` — Mark all as read

---

## Subscriptions

Follow a document to receive notifications when it changes.

`POST /api/subscriptions` — `{ "doc_path": "...", "event": "documents.update" }`

`GET /api/subscriptions?doc_path=` — List subscriptions

`DELETE /api/subscriptions/{id}` — Unsubscribe

---

## Reactions

`POST /api/reactions` — `{ "comment_id": "...", "emoji": "👍" }` — Toggle reaction

`GET /api/reactions?comment_id=` — List reactions for comment

`DELETE /api/reactions/{id}` — Remove reaction

---

## Events (Audit Log)

`GET /api/events?doc_path=&actor_id=&limit=50&offset=0` (admin) — Paginated audit log

---

## Export

`GET /api/export/doc?path=guide/getting-started.md&format=markdown` — Export document

Formats: `markdown`, `html`

`GET /api/export/collection/{id}` — Export all documents in collection as JSON array

---

## Import

`POST /api/import/outline` (admin) — Multipart (`file` field) — Import Outline JSON export

`POST /api/import/markdown` (admin) — Multipart (`file` field) — Import zip of markdown files

---

## Relationships (Linked Documents)

`POST /api/relationships` — `{ "source_doc_path": "...", "target_doc_path": "...", "type": "related" }`

`GET /api/relationships?doc_path=` — List related documents

`DELETE /api/relationships/{id}` — Remove relationship

---

## Outbound Webhooks

`GET /api/webhook-subscriptions` — List

`POST /api/webhook-subscriptions` — `{ "url": "https://...", "events": "documents.create,documents.update", "secret": "optional" }`

`PUT /api/webhook-subscriptions/{id}` — Update

`DELETE /api/webhook-subscriptions/{id}` — Delete

---

## Settings

`GET /api/settings` — Team settings

`PUT /api/settings` (admin) — `{ "name": "Acme Docs", "allow_signups": false }`

---

## Preferences

`GET /api/preferences` — Current user's preferences

`PUT /api/preferences` — `{ "theme": "dark", "language": "en", "notification_email": true }`

---

## Shortcuts

`GET /api/shortcuts` — List keyboard shortcuts (no auth required)

---

## Users

`GET /api/users/search?q=alice` — Search users by name/email (for @mentions)

---

## Sync

`POST /api/sync/pull` — Pull from remote git

`POST /api/sync/push` — Push to remote git

`GET /api/sync/status` — Sync state (ahead/behind/conflicts)

---

## Webhooks (Inbound)

`POST /api/webhooks/git` — Receive push events from GitHub/GitLab/Gitea

Headers: `X-Hub-Signature-256: sha256=...` (GitHub) or `X-Gitlab-Token: ...` (GitLab)

---

## Health & System

`GET /health` — Health check (no auth)

```json
{ "status": "ok", "db": "ok", "git": { "status": "ok", "head": "abc123" }, "version": "0.1.0" }
```

`GET /api/installation` — Setup status (no auth)

```json
{ "setup_complete": true, "user_count": 5, "version": "0.1.0", "git_connected": true }
```

`POST /api/cron/run` (admin) — Manually trigger background cleanup

---

## WebSocket

`ws://host/ws/yjs/{doc_path}?token={access_token}` — Yjs collaborative editing

Connect with a Yjs client (y-websocket). Each document has its own room. The server loads the document from git on first connection and saves back to git when the last client disconnects.

---

## Error Responses

All errors return JSON:

```json
{ "error": "description of what went wrong" }
```

| Status | Meaning |
|--------|---------|
| 400 | Bad request (invalid input) |
| 401 | Unauthorized (missing/invalid token) |
| 403 | Forbidden (insufficient role) |
| 404 | Not found |
| 409 | Conflict (e.g., document already exists) |
| 429 | Rate limited |
| 500 | Internal server error |

---

## OAuth / SSO

`GET /api/auth/providers` — List enabled OAuth providers (no auth)

`GET /api/auth/providers/all` (admin) — List all configured providers

`PUT /api/auth/providers/{provider}` (admin) — Configure provider
```json
{ "client_id": "...", "client_secret": "...", "enabled": true }
```

`GET /api/auth/oauth/{provider}` — Redirect to OAuth login (e.g. `/api/auth/oauth/google`)

`GET /api/auth/oauth/{provider}/callback` — OAuth callback (handles code exchange, creates/finds user, redirects to `/?token=...`)

Supported providers: `google` (pre-configured endpoints). OIDC-compatible for custom providers.

---

## URL Unfurling

`GET /api/unfurl?url=https://youtube.com/watch?v=...` (auth required)

Returns embed info for supported providers:
```json
{
  "url": "https://youtube.com/watch?v=abc",
  "title": "YouTube Video",
  "description": null,
  "image": "https://img.youtube.com/vi/abc/hqdefault.jpg",
  "embed_html": "<iframe ...>",
  "provider": "youtube"
}
```

Supported embed providers: YouTube, Vimeo (OG only), GitHub, Figma, Loom, Twitter/X, any OG-tagged page.

Results cached for 1 hour.

---

## AI

Requires `FORGE_AI_API_KEY` environment variable. Works with any OpenAI-compatible API.

`GET /api/ai/status` — Check if AI is configured (no auth)

`POST /api/ai/suggest` — `{ "doc_path": "...", "content": "..." }` → `[{ "suggestion_type": "improvement|grammar|clarity", "text": "...", "original": "..." }]`

`POST /api/ai/answer` — `{ "doc_path": "...", "question": "..." }` → `{ "answer": "..." }`

`POST /api/ai/summarize` — `{ "content": "..." }` → `{ "summary": "..." }`

`POST /api/ai/generate` — `{ "outline": "## Section\n- point 1\n- point 2" }` → `{ "content": "..." }`

Returns `501 Not Implemented` if `FORGE_AI_API_KEY` is not set.

**Config:**
- `FORGE_AI_API_KEY` — OpenAI API key (or compatible)
- `FORGE_AI_BASE_URL` — Default: `https://api.openai.com/v1`
- `FORGE_AI_MODEL` — Default: `gpt-4o-mini`

---

## Custom Emoji

`GET /api/emojis` — List all custom emojis (no auth)

`POST /api/emojis/upload` (admin) — Multipart: `shortcode` (text) + `file` (image/png, image/gif, image/webp)

`DELETE /api/emojis/{id}` (admin) — Delete emoji

Emoji images served at the URL returned in the `image_url` field.

---

## Async Export Jobs

`POST /api/export-jobs` — Start an export
```json
{ "job_type": "collection-zip", "collection_id": "optional-collection-id" }
{ "job_type": "full-backup" }
```

`GET /api/export-jobs/{id}` — Poll status
```json
{ "id": "...", "status": "pending|complete|failed", "created_at": "...", "completed_at": "..." }
```

`GET /api/export-jobs/{id}/download` — Download completed export (returns zip file)
