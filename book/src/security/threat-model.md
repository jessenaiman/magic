# Threat Model

| Surface | Threat | Mitigation |
|---|---|---|
| **Cross-origin browser tabs** | Local web page calls `fetch("http://127.0.0.1:9875", ...)` to smuggle a JSON command line after HTTP headers. | `--control` is strictly opt-in on `127.0.0.1`, and `control.rs` rejects the first non-JSON line (`POST / HTTP/1.1`) with `"invalid JSON request; closing the connection"` and closes the TCP socket immediately before reading subsequent lines, matching [`DesignCraft` `docs/control-protocol.md`](https://github.com/storytold/designcraft/blob/main/docs/control-protocol.md). |
| **Untrusted MCP / agent prompts** | Prompt attempts to read `~/.ssh` or write outside the song folder. | Launch `magic-cli mcp` with `--automation-read-root` and `--automation-write-root`; `AutomationRoots` canonicalizes roots and target paths via `std::fs::canonicalize` to block out-of-root access. |
| **Unreviewed batch generation** | Agent runs batch visual prompts before the artist approves storyboard structure. | `extensions/scribbles/` enforces the saved-Storyboard-slide gate, requiring saved DeckCraft storyboard slides on disk before generating turnarounds or scene plates. |
| **Accidental data loss on quit / switch** | Window close or `app.quit` / `file.open` runs while unsaved edits exist. | `craft_quit` + `Session` dirty flags check unsaved work before allowing window close or project switch unless `force: true` is passed. |
