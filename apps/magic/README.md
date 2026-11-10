# Magic Studio Desktop (`apps/magic`)

Tauri v2 desktop shell over `magic::Session` (`crates/engine`).

```sh
# Live development with control port:
bun install
cargo run -p magic -- --control 9876

# Connect MCP agent to watch live:
magic-cli mcp --connect 127.0.0.1:9876
```

See root [`README.md`](../../README.md) and [`docs/mcp.md`](../../docs/mcp.md).
