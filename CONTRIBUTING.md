# Contributing to Lore

Thanks for your interest in contributing. This document covers how to get started, what to work on, and how to submit changes.

## Before you start

- Check [open issues](https://github.com/final-commit/lore/issues) — your idea may already be tracked
- For large changes, open an issue first to discuss before writing code
- Small bug fixes and docs improvements can go straight to a PR

## Development setup

**Prerequisites:** Rust 1.75+, Node.js 20+, pnpm 9+

```bash
git clone https://github.com/final-commit/lore
cd lore

# Start the backend
cd server
cargo run
# Runs on http://localhost:3334

# Start the frontend (new terminal)
cd app
pnpm install
pnpm dev
# Runs on http://localhost:3000
```

## Running tests

```bash
# Backend
cd server && cargo test

# Lint
cd server && cargo clippy -- -D warnings

# Frontend
cd app && pnpm build
```

All PRs must pass CI (257+ backend tests, zero frontend build errors).

## What to work on

Good first issues are tagged [`good first issue`](https://github.com/final-commit/lore/labels/good%20first%20issue).

Areas that always welcome contributions:
- Bug fixes (especially with a failing test to reproduce)
- Documentation improvements
- Performance improvements with benchmarks
- Accessibility in the frontend
- Additional OAuth providers

## Pull request guidelines

- One feature or fix per PR
- Include tests for new backend functionality
- Keep commits clean — squash fixups before submitting
- Describe *what* and *why*, not just *what*
- Reference the issue number if applicable (`Closes #123`)

## Code style

**Rust:** `cargo fmt` + `cargo clippy`. No `unwrap()` in non-test code.

**TypeScript:** ESLint config in `app/`. No `any` types.

## Licence

By contributing, you agree that your contributions will be licensed under the project's [BSL 1.1 licence](LICENSE).
