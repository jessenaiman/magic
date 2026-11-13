# `magic-cli` & `magic-mcp` (`apps/magic-cli`, Layer 7)

Command-line interface and MCP stdio entrypoint for Magic Studio. Drives either a headless [`magic_engine::Session`](../../crates/engine/README.md) directly on `.magic` files or a live [`magic --control 9875`](../magic/README.md) desktop window over loopback TCP.

## Where & Why to Explore Code

| File | Related Engine / Upstream Docs | Why You Explore This File |
| :--- | :--- | :--- |
| **[`src/main.rs`](src/main.rs)** | [`crates/engine/README.md`](../../crates/engine/README.md) · [`docs/mcp.md`](../../docs/mcp.md) | Explore to see CLI flag parsing (`--project`, `--connect`, `--automation-read-root`, `--automation-write-root`), `magic-cli mcp` startup, `timeline` interchange subcommands (`inspect`, `import`, `export` for `EDL`/`ALE`/`FCP7`/`FCPXML`/`OTIO` via [`craft-timeline`](../../docs/craft-reference/craft-libs/crates/craft-timeline/README.md)), and `mesh` subcommands (`inspect`, `convert` via [`craft-mesh`](../../docs/craft-reference/craft-libs/crates/craft-mesh/README.md)). |
| **[`src/subcommands.rs`](src/subcommands.rs)** | [`docs/control-protocol.md`](../../docs/control-protocol.md) | Explore for ergonomic domain subcommands (`new`, `storyboard`, `character`, `words`, `motion`, `film`, `app`, `exec`, `commands`, `describe`) and JSON-lines `--connect` client framing. |
| **[`src/mcp_bin.rs`](src/mcp_bin.rs)** (if invoked directly) / `magic-cli mcp` | [`crates/mcp/README.md`](../../crates/mcp/README.md) | Dedicated stdio MCP server entrypoint delegating directly to `magic_mcp`. |
