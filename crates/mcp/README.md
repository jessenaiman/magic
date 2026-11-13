# `magic-mcp` Library Crate (`crates/mcp`)

Model Context Protocol (MCP) stdio server library linked into `magic-cli` (`magic-cli mcp` and `magic-cli mcp --connect 9875`), following [`docs/mcp.md`](../../docs/mcp.md), [`storytold/deckcraft/docs/mcp.md`](https://github.com/storytold/deckcraft/blob/main/docs/mcp.md), and [`storytold/designcraft/docs/mcp.md`](https://github.com/storytold/designcraft/blob/main/docs/mcp.md).

## Where & Why to Explore Code

| File | Related Docs | Why You Explore This File |
| :--- | :--- | :--- |
| **[`src/lib.rs`](src/lib.rs)** & **[`src/server.rs`](src/server.rs)** | [`docs/mcp.md`](../../docs/mcp.md) · [`docs/control-protocol.md`](../../docs/control-protocol.md) | JSON-RPC 2.0 `initialize`, `tools/list`, `tools/call`, and read-only `resources/list` + `resources/read` (`magic://` URIs) for both headless `magic_engine::Session` execution (`magic-cli mcp`) and live loopback TCP forwarding (`magic-cli mcp --connect 9875`). |
