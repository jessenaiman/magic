# Magic Studio MCP Server (`magic-cli mcp` / `magic-mcp`)

`magic-cli mcp` (and `magic-mcp`) serves the [Model Context Protocol](https://modelcontextprotocol.io) over stdio (JSON-RPC 2.0), matching [FilmCraft MCP](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md) (`filmcraft-cli mcp --bridge 127.0.0.1:9876`) and [EffectCraft MCP](https://github.com/storytold/effectcraft/blob/main/docs/mcp.md) (`effectcraft-cli mcp --autosave` / `--bridge 9877`).

| Mode | Command | What Works |
|---|---|---|
| **Remote (`--connect`)** | `magic-cli mcp --connect 127.0.0.1:9875` (with `magic --control 9875`) | Everything — coordinator commands, `$N.field` / `$last.field` batches, live `egui` inspection/input (`inspect_ui`, `ui_elements`, `ui_click`, `screenshot`, `ui_control`), and `bridge_craft` (`:9876` / `:9877`). |
| **Headless (`--headless`)** | `magic-cli mcp --headless [--project song.magic]` | Full `magic::Session` driving `filmcraft-cli`, `effectcraft-cli`, `designcraft-cli`, `photocraft-cli`, `vectorcraft-cli`, and `anicraft-cli`, plus `bridge_craft` to running sibling Craft windows. |

With neither flag, `magic-cli mcp` probes `MAGIC_CONTROL_PORT` (`127.0.0.1:9875`) once and falls back cleanly to headless mode.

## Multi-Server `.mcp.json` Setup (`Magic` + `FilmCraft :9876` + `EffectCraft :9877`)

```json
{
  "mcpServers": {
    "magic": {
      "command": "magic-cli",
      "args": ["mcp"]
    },
    "filmcraft": {
      "command": "filmcraft-cli",
      "args": ["mcp", "--bridge", "127.0.0.1:9876"]
    },
    "effectcraft": {
      "command": "effectcraft-cli",
      "args": ["mcp", "--autosave"]
    }
  }
}
```

## Tools

| Tool (and ArtCraft Core Alias) | Backend | Description |
|---|---|---|
| `list_commands` / `command_list` | Both | List coordinator command IDs, labels, and parameter schemas (`{filter?, enabled_only?, compact?}`). |
| `run_command` / `command_run` | Both | Run one coordinator command (`{command\|id, params?}`). |
| `run_batch` / `command_batch` | Both | Run sequential steps with `"$N.field"` and `"$last.field"` result interpolation (`{steps, stop_on_error?}`). |
| `inspect_document` / `doc_inspect` | Both | Inspect `all`, `project`, `storyboard`, `timeline`, `bridge` (`:9876`/`:9877`), `preflight`, or `history`. |
| `render_preview` | Both | Render a PNG frame for `studio`, `lyrics`, `motion`, `animation`, or `storyboard`. |
| `bridge_craft` | Both | Launch, focus, check status, or execute commands/methods on `filmcraft` (`:9876`), `effectcraft` (`:9877`), `designcraft` (`:7979`), `photocraft`, or `vectorcraft`. |
| `open_file` / `save_file` | Both | Open or save `.magic` projects (`file.open`, `file.save`, `file.saveAs`). |
| `inspect_ui`, `ui_elements`, `ui_click`, `ui_drag`, `ui_scroll`, `ui_key`, `ui_type`, `screenshot`, `ui_control` | `--connect` | Inspect and operate the live `eframe`/`egui` window (`screenshot {window: false}` also renders storyboard spreads headlessly). |
