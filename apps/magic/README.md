# Magic Studio Desktop (`apps/magic`)

Pure-Rust `eframe` 0.37 + `egui` 0.37 + `storytold/craft-libs` desktop coordinator over `magic::Session` (`crates/engine`) and `magic-ui-egui` (`crates/ui-egui`).

```sh
# Live development with loopback control port:
cargo run -p magic -- --control 9876

# Connect MCP agent to watch live:
magic-cli mcp --connect 127.0.0.1:9876
```

See root [`README.md`](../../README.md), [`docs/control-protocol.md`](../../docs/control-protocol.md), and [`docs/mcp.md`](../../docs/mcp.md).
