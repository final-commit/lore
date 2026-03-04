# Outline Feature Parity — Gap Analysis & Build Plan

**Date:** 2026-03-02
**Source:** Outline repo (cloned at `~/lore-platform/outline-ref/`)
**Method:** Full code audit of Outline's 35 API route modules, 34 models, 35 stores, and all UI scenes

---

## Feature Inventory

### ✅ Already Built (in Forge Rust backend)

| Feature | Outline Equivalent | Notes |
|---------|-------------------|-------|
| Document CRUD | `documents` API | Read/write/delete via git |
| Document tree | `collections` (partial) | Flat tree from git, not nested collections |
| Comments (threaded) | `comments` API | With anchoring, resolve, agent support |
| Full-text search | `searches` API | Tantivy vs Outline's Postgres FTS |
| Auth (register/login/JWT) | `auth` API | Argon2+JWT vs Outline's SSO-heavy approach |
| API tokens | `apiKeys` API | Scoped, hashed |
| Git sync | N/A (Outline has no git) | Our differentiator |
| Document history | `revisions` API | Via git log vs Outline's revision table |
| Real-time collab (Yjs) | `Multiplayer.ts` | WebSocket, room-per-doc |
| Health endpoint | N/A | Operational |
| Webhook receive | N/A | GitHub/GitLab/Gitea push events |

### 🔴 Missing — Must Build for Feature Parity

#### Tier 1: Core UX (Users Expect These)

| # | Feature | Outline Source | Priority | Effort |
|---|---------|---------------|----------|--------|
| 1 | **Collections (folders/workspaces)** | `Collection` model, collections API | P0 | Medium |
| 2 | **Drafts** | `Document.publishedAt`, Drafts.tsx | P0 | Small |
| 3 | **Templates** | `Template` model, templates API | P0 | Small |
| 4 | **Trash / Archive / Restore** | `Document.deletedAt/archivedAt`, Trash/ | P0 | Small |
| 5 | **Document publish workflow** | `DocumentPublish.tsx` | P0 | Small |
| 6 | **Stars (bookmarks)** | `Star` model, stars API | P1 | Small |
| 7 | **Pins** | `Pin` model, pins API | P1 | Small |
| 8 | **Views tracking** | `View` model, views API | P1 | Small |
| 9 | **Revisions UI** | `revisions` API, diff view | P1 | Medium |
| 10 | **File attachments/uploads** | `Attachment` model, attachments API | P0 | Medium |
| 11 | **Document sharing (public links)** | `Share` model, shares API | P1 | Medium |

#### Tier 2: Team & Permissions

| # | Feature | Outline Source | Priority | Effort |
|---|---------|---------------|----------|--------|
| 12 | **Groups** | `Group` model, groups API | P1 | Medium |
| 13 | **Group memberships** | `GroupMembership`, `GroupUser` | P1 | Small |
| 14 | **User memberships (collection-level perms)** | `UserMembership` model | P1 | Medium |
| 15 | **Invite users** | `Invite.tsx`, users API | P1 | Small |
| 16 | **User roles per collection** | viewer/editor/admin per collection | P2 | Medium |

#### Tier 3: Editor Features

| # | Feature | Outline Source | Priority | Effort |
|---|---------|---------------|----------|--------|
| 17 | **Find & replace** | `FindAndReplace.tsx` | P1 | Small |
| 18 | **Emoji picker** | `EmojiMenu.tsx`, `Emoji` model | P2 | Small |
| 19 | **Mentions (@user)** | `MentionMenu.tsx` | P1 | Medium |
| 20 | **Hover previews (doc links)** | `HoverPreviews.tsx` | P2 | Small |
| 21 | **Block menu (slash commands)** | `BlockMenu.tsx` | P1 | Medium |
| 22 | **Selection toolbar** | `SelectionToolbar.tsx` | P1 | Small |
| 23 | **Smart text (auto-format)** | `SmartText.ts` | P2 | Small |
| 24 | **Paste handler (rich paste)** | `PasteHandler.tsx` | P1 | Small |

#### Tier 4: Integrations & Advanced

| # | Feature | Outline Source | Priority | Effort |
|---|---------|---------------|----------|--------|
| 25 | **Notifications** | `Notification` model, notifications API | P1 | Medium |
| 26 | **Subscriptions (follow docs)** | `Subscription` model | P2 | Small |
| 27 | **Reactions (on comments)** | `Reaction` model, reactions API | P2 | Small |
| 28 | **Events/audit log** | `Event` model, events API | P1 | Medium |
| 29 | **File operations (export)** | `FileOperation` model, export API | P1 | Medium |
| 30 | **Import (from other platforms)** | `Import` model, imports API | P2 | Large |
| 31 | **Integrations framework** | `Integration` model | P3 | Large |
| 32 | **OAuth clients** | `oauthClients` API | P3 | Medium |
| 33 | **Webhooks (outbound)** | `WebhookSubscription` model | P2 | Medium |
| 34 | **Document relationships** | `Relationship` model | P2 | Small |
| 35 | **Suggestions** | `suggestions` API | P3 | Medium |

#### Tier 5: Settings & Admin

| # | Feature | Outline Source | Priority | Effort |
|---|---------|---------------|----------|--------|
| 36 | **Team settings** | `Team` model, Settings/ | P1 | Medium |
| 37 | **Security settings** | Settings/Security.tsx | P2 | Small |
| 38 | **Auth providers (SSO)** | `AuthenticationProvider` model | P2 | Large |
| 39 | **Custom branding** | Settings/Details.tsx | P3 | Small |
| 40 | **Keyboard shortcuts panel** | KeyboardShortcuts.tsx | P2 | Small |
| 41 | **User preferences** | Settings/Preferences.tsx | P2 | Small |

---

## Build Order (TDD, spec-first)

### Sprint 1: Core Document UX
1. Collections (folders) — DB + API + UI
2. Drafts + publish workflow
3. Templates
4. Trash / archive / restore
5. File attachments (store in git or local fs)

### Sprint 2: Social & Discovery
6. Stars (bookmarks)
7. Pins
8. Views tracking
9. Document sharing (public links)
10. Revisions UI (diff view)

### Sprint 3: Editor Enhancements
11. Slash commands (block menu)
12. Find & replace
13. Mentions (@user)
14. Selection toolbar improvements
15. Paste handler

### Sprint 4: Teams & Permissions
16. Groups + group memberships
17. Collection-level permissions
18. Invite flow
19. Events/audit log

### Sprint 5: Notifications & Export
20. Notifications system
21. Subscriptions (follow)
22. Reactions
23. Export (markdown zip, JSON)
24. Outbound webhooks

### Sprint 6: Advanced
25. Import from Outline/Notion/Confluence
26. OAuth/SSO providers
27. Document relationships
28. Settings UI (team, security, preferences)
29. Keyboard shortcuts panel

---

## What We DON'T Need (Outline has, we skip)

| Feature | Why Skip |
|---------|----------|
| PostgreSQL | We use Git + SQLite (our north star) |
| Redis | Not needed — moka cache in-process |
| S3 storage | Attachments go in git or local fs |
| Multi-team | Self-hosted = single team |
| Desktop app redirect | Web-only for now |
| Cron jobs | Tokio scheduler if needed |
| Authentication providers (Google, OIDC, SAML) | P2 — basic auth first |
