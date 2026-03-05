# Lore Code Review: Pre-Release Security & Quality Audit

**Review Date:** 2026-03-04
**Reviewer:** Claude Opus 4.5
**Scope:** Open source release readiness (server/Rust + community files)

---

## P0 Issues (Must Fix Before Release)

### [P0] OAuth CSRF Vulnerability - Missing State Parameter

**File:** `server/src/api/oauth_api.rs:23-34`

**Issue:** The OAuth redirect flow does not include a `state` parameter. This allows CSRF attacks where an attacker can:
1. Initiate an OAuth flow on their own machine
2. Capture the callback URL with the authorization code
3. Trick a victim into visiting that URL, linking the attacker's account

The current `oauth_redirect` function constructs the auth URL without a state parameter:
```rust
let auth_url = format!(
    "{}?client_id={}&redirect_uri={}&response_type=code&scope=openid+email+profile&access_type=offline",
    prov.auth_url,
    urlencoding::encode(&client_id),
    urlencoding::encode(&redirect_uri),
);
```

**Fix:**
1. Generate a cryptographically random state token
2. Store it in a short-lived cache or signed cookie
3. Include `&state={state}` in the OAuth redirect URL
4. Verify the state matches in `oauth_callback` before exchanging the code

---

### [P0] OAuth Endpoints Lack Rate Limiting

**File:** `server/src/api/oauth_api.rs:23, 40`

**Issue:** The `oauth_redirect` and `oauth_callback` endpoints do not apply rate limiting, unlike the password-based auth endpoints (`login`, `register`). This allows brute-force attacks against the OAuth flow.

**Fix:** Apply the same `rate_limiter.check()` pattern used in `api/auth.rs:36-38` to the OAuth endpoints.

---

### [P0] Production Code Contains `.unwrap()` That Can Panic

**File:** `server/src/api/attachments.rs:79`

**Issue:** The attachment download endpoint uses `.unwrap()` on a `Response::builder()`:
```rust
Ok(Response::builder()
    .status(StatusCode::OK)
    .header(header::CONTENT_TYPE, meta.content_type)
    .header(header::CONTENT_DISPOSITION, disposition)
    .header(header::CONTENT_LENGTH, content_len)
    .body(Body::from(bytes))
    .unwrap())  // <-- Can panic
```

While this specific builder is unlikely to fail, any panic in an Axum handler crashes the request and could be triggered by malformed header values.

**Fix:** Replace with proper error handling:
```rust
.body(Body::from(bytes))
.map_err(|e| AppError::Internal(format!("response builder failed: {e}")))?
```

---

## P1 Issues (Important Before Release)

### [P1] README Contains Stale "Forge" References

**File:** `README.md:5, 18, 24-35, 60-77, 148-170`

**Issue:** The project was renamed from Forge to Lore, but the README still contains "Forge" in multiple places:
- Line 5: "Forge is a full-featured documentation platform"
- Line 18: "Forge is self-hostable..."
- Feature comparison table uses "Forge" as header
- Clone URLs point to `yourorg/forge`
- Quick start instructions reference wrong repo

**Fix:** Global search-replace "Forge" -> "Lore" and "forge" -> "lore" in README.md. Update repository URLs to match actual GitHub location.

---

### [P1] CHANGELOG Missing Version Tag for Release

**File:** `CHANGELOG.md:10`

**Issue:** All changes are listed under `[Unreleased]`. For a 0.1.0 release, this should be versioned.

**Fix:** Change:
```markdown
## [Unreleased]
```
to:
```markdown
## [0.1.0] - 2026-03-XX
```

---

### [P1] Git + SQLite Atomicity Gap

**Files:** `server/src/git/engine.rs`, `server/src/doc_meta/engine.rs`

**Issue:** Document operations write to both Git (via `GitEngine`) and SQLite (via `DocMetaEngine`), but these are not atomic. If a Git commit succeeds but the SQLite write fails (e.g., disk full), the system can get out of sync.

Example flow in document creation:
1. `git.create_file()` - creates commit
2. `doc_meta.get_or_create()` - writes to SQLite
3. If step 2 fails, Git has the file but SQLite doesn't know about it

**Fix:** For v0.1.0, document this as a known limitation. For future: implement a write-ahead log or saga pattern that can recover from partial failures.

---

### [P1] Default JWT Secret in Config

**File:** `server/src/config.rs:33`

**Issue:** The default JWT secret is a hardcoded placeholder:
```rust
jwt_secret: "change-me-in-production-use-32-chars-min".to_string(),
```

If someone runs Lore without setting `LORE_JWT_SECRET`, tokens are signed with a known key.

**Fix:** Add a startup check that panics if JWT secret matches the default or is too short:
```rust
if config.jwt_secret == "change-me-in-production-use-32-chars-min" {
    panic!("LORE_JWT_SECRET must be set in production");
}
```

Alternatively, generate a random secret on first run and persist it.

---

## P2 Issues (Should Fix)

### [P2] Content-Disposition Header Injection Risk

**File:** `server/src/api/attachments.rs:71`

**Issue:** The filename is interpolated directly into the Content-Disposition header:
```rust
let disposition = format!("inline; filename=\"{}\"", meta.filename);
```

If `meta.filename` contains quotes or control characters, this could lead to header injection or download dialog manipulation.

**Fix:** Sanitize the filename or use proper RFC 5987 encoding:
```rust
let safe_filename = meta.filename.replace('"', "").replace('\n', "").replace('\r', "");
let disposition = format!("inline; filename=\"{safe_filename}\"");
```

---

### [P2] No Image MIME Type Validation for Emoji Upload

**File:** `server/src/api/emojis_api.rs:19-20`

**Issue:** The emoji upload accepts any file and guesses the extension from content-type, but doesn't validate that the uploaded bytes are actually a valid image:
```rust
let ct = field.content_type().unwrap_or("image/png").to_string();
ext = if ct.contains("gif") { "gif".into() } else if ct.contains("webp") { "webp".into() } else { "png".into() };
```

An attacker could upload arbitrary data with a fake content-type.

**Fix:** Consider using the `infer` crate to detect actual file type from magic bytes, or at minimum validate the content-type header isn't spoofed.

---

### [P2] Rate Limiter Not Applied to Password Reset (if exists)

**File:** `server/src/api/auth.rs`

**Issue:** Rate limiting is applied to login and register, but if a password reset endpoint exists, it should also be rate-limited. (Not found in current code, but flagging for when it's added.)

---

### [P2] WebSocket Path Not Validated

**File:** `server/src/api/mod.rs:216`

**Issue:** The WebSocket route uses a wildcard path:
```rust
.route("/ws/yjs/{doc_path}", get(yjs_ws_handler))
```

The `doc_path` should be validated with `validate_path()` to prevent path traversal via WebSocket connections.

---

## Positive Findings

The codebase demonstrates strong security practices in several areas:

1. **Path traversal protection:** `validate_path()` in `error.rs:77-92` properly checks for `..`, absolute paths, and null bytes.

2. **Foreign keys enforced:** `PRAGMA foreign_keys=ON` is set in `db/mod.rs:21`.

3. **Git operations serialized:** The `GitQueue` mutex pattern in `git/queue.rs` prevents race conditions.

4. **Webhook signatures verified:** The `webhooks.rs` file uses HMAC-SHA256 with constant-time comparison.

5. **JWT validation correct:** Access vs refresh token distinction enforced, expiry validated.

6. **Rate limiting implemented:** Auth endpoints use the moka-based rate limiter.

7. **Error messages sanitized:** 5xx errors return generic messages in `error.rs:67-70`.

8. **Parameterized queries:** All SQL uses `rusqlite::params![]`, no string interpolation found.

---

## Summary

| Priority | Count |
|----------|-------|
| P0       | 3     |
| P1       | 4     |
| P2       | 4     |

### Overall Assessment

**Not ready for public release.** The P0 OAuth CSRF vulnerability is a significant security issue that could allow account takeover. The missing rate limiting on OAuth endpoints compounds this risk.

### Top 3 Fixes Before Launch

1. **Add OAuth state parameter** (P0) - Prevents CSRF attacks during login
2. **Apply rate limiting to OAuth endpoints** (P0) - Prevents brute-force attacks
3. **Replace .unwrap() with proper error handling** (P0) - Prevents server panics

After fixing these three issues and the README branding inconsistencies, Lore is ready for a 0.1.0 release. The overall code quality is high, with good security practices throughout the authentication, database, and git layers.

---

*Review completed by Claude Opus 4.5*
