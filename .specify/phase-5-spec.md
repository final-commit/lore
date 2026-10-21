# Phase 5: Enterprise Foundation — Spec

## Goal
Close first enterprise deals. SAML, Postgres, S3, HA, audit log, key escrow.

## Tasks

### 5.1 — PostgresBackend Implementation
- Implement full PostgresBackend (was scaffold in Phase 0)
- Use `sqlx` with Postgres driver
- Connection pool: `sqlx::PgPool`, configurable via LORE_DATABASE_URL
- All DbBackend trait methods implemented with equivalent Postgres SQL
- Migration runner for Postgres (sqlx migrate)
- Tests: run full test suite against Postgres (CI matrix: SQLite + Postgres)

### 5.2 — SAML 2.0 SP
- Add `samael` crate
- SP-initiated SSO flow: redirect to IdP → receive assertion → create session
- IdP metadata import (XML)
- Attribute mapping: email, name, role from SAML assertion
- Tested against: Okta (dev account), Azure AD (dev tenant), generic SAML 2.0
- Config: LORE_SAML_IDP_METADATA_URL, LORE_SAML_SP_ENTITY_ID
- SSO enforcement policy: workspace admin can block local auth (LORE_SSO_REQUIRED=true)

### 5.3 — WebAuthn / Passkeys
- Add `webauthn-rs` crate
- Registration: `/api/auth/webauthn/register/begin` + `/finish`
- Authentication: `/api/auth/webauthn/authenticate/begin` + `/finish`
- Store credential in `webauthn_credentials` table
- Works alongside existing JWT auth (second factor or primary)
- UI: settings page → "Add passkey" flow

### 5.4 — S3-Compatible Storage Adapter
- Create `StorageBackend` trait: `put(key, bytes)`, `get(key)`, `delete(key)`, `presigned_url(key, ttl)`
- `LocalStorageBackend` — stores files on disk (current behaviour)
- `S3StorageBackend` — uses `aws-sdk-s3` crate (S3, Minio, R2, Backblaze compatible)
- Config: LORE_STORAGE_BACKEND=local|s3, LORE_S3_BUCKET, LORE_S3_ENDPOINT (for Minio/R2)
- Attachments, export archives, agent outputs use StorageBackend

### 5.5 — Immutable Audit Log
- `events` table: append-only with signed hash chain
- Each event: `id, type, actor_id, actor_type, resource_type, resource_id, payload, prev_hash, hash, timestamp`
- `hash = SHA256(prev_hash || timestamp || type || actor_id || resource_id || payload)`
- Genesis block: `prev_hash = "0000..."`
- `AuditLog::append(event)` — atomic: compute hash, insert, never update/delete
- `AuditLog::verify_chain() -> Result<(), TamperEvidence>` — validates chain integrity
- Export: `GET /api/audit-log?from=&to=&format=json|csv` (admin only)
- Retention: never deleted, archiveable to S3

### 5.6 — MLS Key Escrow
- Admin public key: generated + stored at workspace setup
- When group key material rotates: encrypt backup with admin pubkey → store in `mls_key_escrow`
- Recovery flow: admin provides privkey → decrypt group keys → rejoin group
- Legal hold: enables accessing message history if legally required
- Optional — can be disabled in privacy-only deployments (LORE_KEY_ESCROW_ENABLED=false)

### 5.7 — Stateless Server + Redis
- Externalise all in-memory state: WebSocket presence map → Redis
- Session tokens: JWT (already stateless) — no change
- Agent run queue: already DB-backed — no change
- Redis pub/sub: WebSocket message routing across multiple server instances
- Config: LORE_REDIS_URL (optional — falls back to in-memory if not set)

### 5.8 — HA Deployment + Helm Chart
- Server must be horizontally scalable when LORE_REDIS_URL set
- Docker Compose for HA: 3x lore-server + Postgres + Redis + Nginx load balancer
- Kubernetes Helm chart: `charts/lore/`
  - Deployment (replicas: 3), Service, Ingress, ConfigMap, Secret
  - PostgresOperator or external Postgres
  - Redis (bitnami/redis subchart)
  - HPA (CPU-based autoscaling)
- Test: deploy 3 replicas, send message via instance 1, verify delivery via instance 2

### 5.9 — Granular RBAC
- Custom roles (beyond admin/member): define per-workspace
- Per-collection permissions (already exists — extend)
- Per-channel permissions
- Per-project permissions
- Role assignment: user → role → permissions
- API: CRUD for custom roles, role assignments

### 5.10 — Tests
- Postgres: full test suite green with PG backend
- SAML: mock IdP assertion → verify session created
- WebAuthn: registration + authentication round-trip
- S3: upload/download via mock S3 (minio in CI)
- Audit log: append + verify chain + tamper detection
- All existing tests pass

## Acceptance Criteria
- [ ] Postgres backend fully implemented, test suite green on both SQLite + Postgres
- [ ] SAML SSO works with mock Okta assertion
- [ ] WebAuthn passkey registration + auth works
- [ ] S3 storage adapter works (local + Minio)
- [ ] Audit log: immutable, signed, chain-verifiable
- [ ] Redis-backed HA: 2 instances share WebSocket state
- [ ] Helm chart deploys cleanly
- [ ] All tests pass
- [ ] git commit: "feat: Phase 5 — enterprise foundation (Postgres, SAML, S3, HA, audit log)"
