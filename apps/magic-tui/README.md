# `magic-tui` — Backstage Console (`apps/magic-tui`, Layer 7)

Terminal operator console (**Backstage**) for monitoring project readiness, inspecting stage cues, and dispatching Magic engine or `--control 9875` commands over SSH or headless sessions.

## Where & Why to Explore Code

| File | Related Docs | Why You Explore This File |
| :--- | :--- | :--- |
| **[`src/main.rs`](src/main.rs)** | [`docs/control-protocol.md`](../../docs/control-protocol.md) · [`crates/engine/README.md`](../../crates/engine/README.md) | Explore when updating terminal dashboards, keyboard command dispatch, or live `--connect 127.0.0.1:9875` status polling against a running `magic` desktop instance. |
