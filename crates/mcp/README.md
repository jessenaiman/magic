# `magic-mcp` (`crates/mcp`, Layer 6)

Model Context Protocol (MCP) stdio server exposing Magic Studio's command registry and live `--control` UI automation to AI agents (`Claude Code`, `Gemini`, `Cursor`, and MCP clients), following the ArtCraft convention in [`docs/mcp.md`](../../docs/mcp.md) and [`DesignCraft` `docs/mcp.md`](../../docs/craft-reference/core-craft-apps/designcraft/docs/mcp.md).

## Where & Why to Explore Code

| File | Related Docs | Why You Explore This File |
| :--- | :--- | :--- |
| **[`src/lib.rs`](src/lib.rs)** | [`docs/mcp.md`](../../docs/mcp.md) · [`docs/control-protocol.md`](../../docs/control-protocol.md) | Explore to inspect JSON-RPC 2.0 `initialize`, `tools/list`, and `tools/call` routing for both headless `magic_engine::Session` execution (`--headless`) and live TCP bridge forwarding (`--connect 127.0.0.1:9875`), including `ui.docking`, `ui.inspect`, `ui.click`, and `ui.screenshot`. |
