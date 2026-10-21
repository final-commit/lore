# Phase 3: E2E Messaging — Spec

## Goal
Slack replacement. Privacy-first teams switch here. Server never sees plaintext.

## MLS Crate: mls-rs (Decision D-001)

## Tasks

### 3.1 — MLS Integration (mls-rs)
- Add `mls-rs` to server/Cargo.toml
- Server role: key package distributor + ciphertext relay. Never decrypts.
- `server/src/messaging/mls.rs` — MlsKeyService
  - Store user key packages (public only)
  - Distribute key packages to group members
  - Relay Welcome messages for group add/remove

### 3.2 — Database Tables
- `channels` — id, name, workspace_id, mls_group_id, is_dm, is_archived, created_at
- `messages` — id, channel_id, sender_id, ciphertext (bytes), mls_epoch, thread_id, sent_at, edited_at
- `threads` — id, channel_id, parent_message_id, reply_count
- `mls_key_packages` — user_id, key_package_bytes, uploaded_at
- `mls_group_state` — channel_id, group_info (server-side group state for adds/removes)

### 3.3 — Messaging Engine (server/src/messaging/)
- `engine.rs` — MessageEngine
  - send_message: validate sender in channel, store ciphertext, fan-out via WebSocket
  - create_channel: create MLS group, store mls_group_id
  - add_member: fetch key package, generate Welcome, distribute
  - remove_member: Update message, commit to group
  - edit_message: store new ciphertext, mark edited
  - delete_message: mark deleted (ciphertext zeroed)

### 3.4 — WebSocket Message Delivery
- Extend existing WebSocket handler for messaging
- Per-user subscription: authenticated WS connection → user receives messages for their channels
- Fan-out: message stored → push to all online channel members
- Offline delivery: messages stored, delivered on reconnect

### 3.5 — REST API for Messaging
- `GET /api/channels` — list workspace channels
- `POST /api/channels` — create channel
- `GET /api/channels/:id/messages` — paginated history (ciphertext only)
- `POST /api/channels/:id/messages` — send message (ciphertext payload)
- `POST /api/channels/:id/members` — add member
- `DELETE /api/channels/:id/members/:user_id` — remove member
- `GET /api/channels/dm/:user_id` — get or create DM channel
- `POST /api/messages/:id/react` — add reaction
- `PUT /api/messages/:id` — edit
- `DELETE /api/messages/:id` — delete
- `GET /api/messages/:id/thread` — thread replies
- `POST /api/keys/upload` — upload MLS key package
- `GET /api/keys/:user_id` — fetch key package for user

### 3.6 — Client-Side MLS (app/)
- MLS key generation in browser: WebCrypto API
- Key package upload on first login / key rotation
- MLS group operations: join, add member, remove member
- Encrypt before send, decrypt on receive
- Local encrypted message store (IndexedDB)
- Key material never leaves browser unencrypted

### 3.7 — Messaging UI (app/)
- `/messages` — channel list sidebar
- Channel view: message history, input box, thread panel
- DM view: direct message conversation
- Thread panel: reply thread sidebar
- Reactions: emoji picker, reaction counts
- Typing indicators, online presence (encrypted status)
- Read receipts
- Message search (client-side decrypt → search local store)
- `@mention` autocomplete for users and agents

### 3.8 — Presence System
- Online/away/do-not-disturb status
- WebSocket heartbeat: ping every 30s, mark offline after 90s no ping
- Status stored in Redis (Phase 5) or in-memory map (Phase 3)

### 3.9 — Agent Messaging
- Agents can post to channels (WriteMessages capability)
- Agent messages: signed with agent identity, labelled with agent badge in UI
- Agents receive messages mentioning them (ReadMessages capability)

### 3.10 — Tauri Desktop App (desktop/)
- Initialise Tauri project in desktop/
- Wraps the Next.js web app in a desktop shell
- OS keychain integration for MLS keys (tauri-plugin-keychain or keyring-rs)
- Native notifications (tauri-plugin-notification)
- System tray (show/hide window)
- Auto-updater scaffold
- Offline message queue: messages composed offline queued in local SQLite, sent on reconnect
- macOS + Windows + Linux targets

### 3.11 — Tests
- Server: channel CRUD, message send/receive (ciphertext round-trip), member add/remove
- MLS key package upload/fetch
- WebSocket fan-out (mock clients)
- All existing tests pass

## Acceptance Criteria
- [ ] Server stores only ciphertext — verified by architecture test
- [ ] Channel create, member add/remove, DM
- [ ] Messages delivered via WebSocket
- [ ] Client-side MLS encrypt/decrypt (tested with mls-rs test vectors)
- [ ] Messaging UI complete
- [ ] Tauri desktop app builds for macOS
- [ ] Agent messaging works
- [ ] All tests pass
- [ ] git commit: "feat: Phase 3 — E2E messaging (MLS, channels, threads, Tauri desktop)"
