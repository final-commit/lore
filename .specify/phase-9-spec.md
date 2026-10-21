# Phase 9: Mobile — Spec

## Goal
Daily-driver product. Messaging requires mobile.

## Stack: React Native (Expo) + Expo Router

## Tasks

### 9.1 — Project Setup
- `mobile/` directory at repo root
- Expo + React Native + Expo Router
- TypeScript throughout
- Shared API client with app/ (extract to `packages/api-client/`)
- iOS + Android targets

### 9.2 — Authentication
- Login screen (email/password + OAuth)
- JWT storage: Expo SecureStore (iOS Keychain / Android Keystore)
- Auto-refresh token on expiry
- Biometric unlock (FaceID / fingerprint) for app re-open

### 9.3 — Messaging (Priority Feature)
- Channel list
- Channel view: message history + send input
- DM list + DM view
- Thread view (reply thread)
- MLS key sync: derive mobile keys from desktop key material (key agreement protocol)
- Mobile MLS keys: Secure Enclave (iOS) / StrongBox (Android)
- Offline queue: messages composed offline queued in local SQLite, sent on reconnect

### 9.4 — Push Notifications
- APNs (iOS) + FCM (Android) via Expo Notifications
- Server: on message received → send encrypted push token to recipient
- Client: decrypt notification → show "New message in #channel"
- Agent notifications: "Agent task completed" push

### 9.5 — Docs (Read + Edit)
- Doc browser: collections → documents
- Document viewer: rendered markdown
- Mobile-optimised editor: basic formatting (bold, italic, headings, lists, links)
- Offline: cache last 50 docs in local SQLite, sync on reconnect

### 9.6 — Agent Notifications
- Agent task completed → push notification with result summary
- Tap → open agent run viewer in app
- Approve/reject diff proposals on mobile

### 9.7 — App Store Submission
- iOS: App Store Connect setup, provisioning profiles, screenshots
- Android: Google Play Console setup, signing, screenshots
- Privacy policy + App Store description
- TestFlight (iOS) + Internal Testing (Android) for beta

### 9.8 — Tests
- Unit: API client, JWT refresh, offline queue
- E2E: Detox (iOS simulator) — login → send message → receive message
- All existing server tests pass

## Acceptance Criteria
- [ ] Messaging: send + receive E2E encrypted messages on iOS + Android
- [ ] Push notifications working (APNs + FCM)
- [ ] Doc viewer + basic editor
- [ ] Offline message queue
- [ ] Builds for iOS (Xcode) + Android (Gradle)
- [ ] Submitted to TestFlight + Play Internal Testing
- [ ] git commit: "feat: Phase 9 — mobile (React Native, iOS + Android, E2E messaging)"
