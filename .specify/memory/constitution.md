# Project Constitution — Lore Platform

## Vision
An AI-native, self-hosted team OS. One Rust binary. Docs + Messaging + Agents + Code + Projects.
E2E encrypted. Agents as first-class participants — not bolt-on features.

## Principles
1. **Tests first** — never ship without tests passing
2. **Spec first** — always write the spec before coding
3. **Research first** — validate assumptions with web search before major decisions
4. **1-3-1 decisions** — log all significant decisions in DECISIONS.md
5. **Commit regularly** — logical units, clear messages
6. **Existing tests are sacred** — 260 must always pass, never regress

## Customer
- Dev teams 5-50 people (pre tool lock-in)
- Compliance-heavy SMBs (law, fintech, healthcare) needing E2E + self-hosted
- Target: replace Notion + Slack + Linear + GitHub docs for one team

## Architecture North Star
See `docs/NORTH_STAR_ARCHITECTURE.md` and `CLAUDE.md`.

## What We Are NOT Building
- A code editor (browser only)
- A CI/CD system
- A Matrix homeserver
- An email client
- A personal notes app
