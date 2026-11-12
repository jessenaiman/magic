# Threat Model

| Surface | Threat | Mitigation |
|---|---|---|
| **Cross-origin browser tabs** | Local web page calls `fetch("http://127.0.0.1:9875", ...)` to smuggle a JSON command line after HTTP headers. | `control.rs` rejects the first non-JSON line (`POST / HTTP/1.1`) with `"invalid JSON request; closing the connection"` and closes the TCP socket immediately before reading subsequent lines, plus requires a 64-hex token. |
| **Untrusted MCP / agent prompts** | Prompt attempts to read `~/.ssh` or write outside the song folder. | Launch `magic-cli mcp` or `magic --control` with `--automation-read-root` and `--automation-write-root`; `AutomationRoots` blocks out-of-root paths and symlinks. |
| **Accidental data loss on quit / switch** | Window close or `app.quit` / `file.open` runs while unsaved edits or modal drafts exist. | `apps/magic/src/lifecycle.rs` + `craft_quit::WindowClose` check both `Session` dirty flags and in-process `egui` `ui.drafts` before allowing close or project switch unless `force: true` is passed. |
| **Pre-alpha sibling GUI instability** | Agent tries to open early-alpha `AniCraft` desktop window. | `craft-doc::Ani.allows_live_ui()` is locked to `false`; only headless `anicraft-cli` frame/sprite exports are allowed. |
