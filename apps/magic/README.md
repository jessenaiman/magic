# Magic Studio (`apps/magic`)

Pure-Rust `eframe` 0.37 + `egui` 0.37 + `storytold/craft-libs` + `storytold/craft-ai-integrations` desktop workstation coordinator for Magic Studio.

```sh
# Launch Magic Studio with authenticated loopback --control on 127.0.0.1:9875
cargo run -p magic -- --control 9875

# Connect CLI or MCP server (FilmCraft uses :9876, EffectCraft uses :9877)
magic-cli --connect 127.0.0.1:9875 ui inspect
magic-cli mcp --connect 127.0.0.1:9875
magic-cli bridge status --ensure-tokens
```
