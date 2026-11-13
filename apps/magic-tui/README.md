# `magic-tui` — Backstage Snapshot Console (`apps/magic-tui`)

Zero-polling ANSI terminal dashboard (**Backstage**) rendering a 5-tab Technicolor Workshop snapshot (`Overview`, `Storyboard`, `Tracks`, `Doctor`, `Commands`) from a `.magic` project on disk or a one-shot `--connect 127.0.0.1:9875` control query, with zero third-party TUI dependencies.

## Files

- **[`src/main.rs`](src/main.rs)** — CLI flags (`--project`, `--connect`, `--tab`, `--width`) and one-shot entrypoint.
- **[`src/snapshot.rs`](src/snapshot.rs)** — Headless `magic_engine::Session` and live `--control` JSON snapshot loader.
- **[`src/ui.rs`](src/ui.rs)** — 24-bit ANSI 5-tab Technicolor Workshop renderer.
