# desktop — Lore Desktop App

**Phase 3 (weeks 17–26).** This directory is a scaffold. No desktop code exists yet.

## What will live here

A cross-platform desktop application for Lore built with **Tauri** (Rust + WebView).

The desktop app wraps the Next.js frontend and adds:
- System tray icon with notification badges
- Native OS notifications
- Offline mode (local SQLite + sync-on-reconnect)
- Keyboard shortcut registration at the OS level
- Auto-updater

## Platforms (planned)

| Platform | Architecture |
|----------|-------------|
| macOS    | arm64, x86_64 |
| Linux    | x86_64, aarch64 |
| Windows  | x86_64 |

## Architecture (planned)

```
desktop/
├── src-tauri/   # Tauri Rust backend (window management, IPC, auto-update)
├── src/         # Desktop-specific React components (tray, notifications)
└── icons/       # App icons for each platform
```

The Tauri backend communicates with the Lore server binary over localhost HTTP.
In offline mode, it embeds a local server instance directly.

## Getting started (Phase 3)

Documentation and build instructions will be added when the desktop app is implemented.
Track progress in the [Phase 3 milestone](https://github.com/final-commit/lore/milestones).
