# `magic-tui` — Backstage Snapshot Console (`apps/magic-tui`, Layer 7)

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

Zero-polling ANSI terminal dashboard (**Backstage**) rendering a 5-tab Technicolor Workshop snapshot (`Overview`, `Storyboard`, `Tracks`, `Doctor`, `Commands`) from a `.magic` project on disk or a one-shot `--connect 127.0.0.1:9875` control query, using zero third-party TUI crates outside `Cargo.lock`.

## Files

- **[`src/main.rs`](src/main.rs)** — CLI flags (`--project`, `--connect`, `--tab`, `--width`) and one-shot entrypoint.
- **[`src/snapshot.rs`](src/snapshot.rs)** — Headless `magic_engine::Session` and live `--control` JSON snapshot loader.
- **[`src/ui.rs`](src/ui.rs)** — 24-bit ANSI 5-tab Technicolor Workshop renderer.
