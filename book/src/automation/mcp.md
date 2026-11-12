# MCP Server (`magic-cli mcp` / `magic-mcp`)

Magic Studio ships a standard ArtCraft Model Context Protocol server over stdio (`crates/mcp`, JSON-RPC 2.0).

## `.mcp.json` Configuration

```json
{
  "mcpServers": {
    "magic": {
      "command": "magic-cli",
      "args": ["mcp"]
    },
    "magic-app": {
      "command": "magic-cli",
      "args": ["mcp", "--connect", "127.0.0.1:9875"]
    }
  }
}
```

<<<<<<< HEAD
- **Headless by default (`magic-cli mcp`)**: Runs an isolated in-process `magic::Session` over stdio with zero network sockets (never silently auto-probing `:9875`, matching `DeckCraft` and `CADCraft` RFC-0012).
- **Live window (`magic-cli mcp --connect 127.0.0.1:9875`)**: Connects strictly to `127.0.0.1` (`Ipv4Addr::LOCALHOST`) on a running `magic --control 9875` window, bounds replies at `16 MiB`, and never replays mutating requests on lost replies (`outcome unknown`).
- **Confined headless (`magic-cli mcp --automation-read-root <dir> --automation-write-root <dir>`)**: Confines headless file reads and writes to the given roots (`--connect` together with root flags is rejected).
- **Sibling ArtCraft Apps (`magic-cli <product>`)**: Connect directly to each sibling app's own `<product>-cli mcp` (`deckcraft-cli mcp --connect 7990`, `designcraft-cli mcp --connect 7979`, `filmcraft-cli mcp --bridge 127.0.0.1:9876` with `FILMCRAFT_CONTROL_TOKEN`, `effectcraft-cli mcp --bridge 9877`, `photocraft-cli mcp --bridge 127.0.0.1:7878 --control-token-file <file>`, `vectorcraft-cli mcp --connect 127.0.0.1:7979`). Calling `magic-cli <product>` prints a 3-line setup signpost and exits with code `2`.
=======
- **Headless by default (`magic-cli mcp`)**: Runs an isolated in-process `magic_engine::Session` over stdio with zero network sockets (never silently auto-probing `:9875`, matching `CADCraft` RFC-0012).
- **Live window (`magic-cli mcp --connect 127.0.0.1:9875`)**: Connects strictly to `127.0.0.1` (`Ipv4Addr::LOCALHOST`) on a running `magic --control 9875` window, bounds replies at `16 MiB`, and never replays mutating requests on lost replies (`outcome unknown`).
- **Confined headless (`magic-cli mcp --automation-read-root <dir> --automation-write-root <dir>`)**: Confines headless file reads and writes to the given roots (`--connect` together with root flags is rejected).
- **Sibling ArtCraft Apps (`magic-cli <product>`)**: Connect directly to each sibling app's own `<product>-cli mcp` (`designcraft-cli mcp --connect 7979`, `filmcraft-cli mcp --bridge 127.0.0.1:9876` with `FILMCRAFT_CONTROL_TOKEN`, `effectcraft-cli mcp --bridge 9877`, `photocraft-cli mcp --bridge 127.0.0.1:7878 --control-token-file <file>`, `vectorcraft-cli mcp --connect 127.0.0.1:7979`). Calling `magic-cli <product>` prints a 3-line setup signpost and exits with code `2`.
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

## Core Tools

| Tool | Purpose |
| :--- | :--- |
| `command_list` / `list_commands` | List Magic command IDs, labels, and parameter schemas |
| `command_run` / `run_command` | Run one Magic command (`{id, params?}`) |
| `command_batch` / `run_batch` | Run sequential commands with `$1.field` / `$last.field` interpolation |
| `doc_inspect` / `inspect_document` | Inspect `all`, `project`, `storyboard`, or `timeline` |
| `open_file` / `save_file` | Open or save a `.magic` project |
| `render_preview` / `screenshot` | Extract stage preview PNG or capture live window PNG |
| `inspect_ui`, `ui_elements`, `ui_click`, `ui_drag`, `ui_scroll`, `ui_key`, `ui_type`, `ui_control` | Drive the live `magic --control 9875` window (`--connect` only) |
