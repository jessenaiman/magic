# `magic-cli` (`apps/magic-cli`)

Command-line interface and MCP stdio entrypoint (`magic-cli mcp`) for Magic Studio, matching [`storytold/deckcraft/docs/mcp.md`](https://github.com/storytold/deckcraft/blob/main/docs/mcp.md) and [`storytold/designcraft/docs/mcp.md`](https://github.com/storytold/designcraft/blob/main/docs/mcp.md). Drives either a headless [`magic_engine::Session`](../../crates/engine/README.md) directly on `.magic` projects or a live [`magic --control 9875`](../magic/README.md) desktop window over `127.0.0.1`.

## Where & Why to Explore Code

| File | Related Engine / Upstream Docs | Why You Explore This File |
| :--- | :--- | :--- |
| **[`src/main.rs`](src/main.rs)** | [`crates/engine/README.md`](../../crates/engine/README.md) · [`docs/mcp.md`](../../docs/mcp.md) | CLI flag parsing (`--project`, `--connect`, `--automation-read-root`, `--automation-write-root`), `magic-cli new [existing_project] --audio song.wav --name "..."`, `magic-cli studio map-tracks`, `magic-cli mcp` (`--connect 9875` or headless with `magic://` resources), `timeline` interchange subcommands (`craft-timeline`), and `mesh` subcommands (`craft-mesh`). |
| **[`src/subcommands.rs`](src/subcommands.rs)** | [`docs/control-protocol.md`](../../docs/control-protocol.md) | Ergonomic domain subcommands (`new`, `studio`, `storyboard`, `character`, `words`, `motion`, `film`, `app`, `exec`, `commands`, `describe`) and JSON-lines `--connect` client framing. |
