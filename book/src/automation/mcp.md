# MCP Server (`magic-cli mcp`) & `magic://` Resources

Magic Studio exposes its MCP server directly from `magic-cli` (`crates/mcp`, JSON-RPC 2.0 over stdio), following [`storytold/deckcraft/docs/mcp.md`](https://github.com/storytold/deckcraft/blob/main/docs/mcp.md) and [`storytold/designcraft/docs/mcp.md`](https://github.com/storytold/designcraft/blob/main/docs/mcp.md).

## Install & Register `magic-cli mcp`

Build and install from the repository root:

```sh
./install.sh --from-source
# or: cargo install --path apps/magic-cli
```

Register headless and live `--control` servers:

```sh
# 1. Headless session (isolated in-process Session + magic:// resources)
claude mcp add magic -- magic-cli mcp

# 2. Live desktop watching (start `magic --control 9875` first)
magic --control 9875 &
claude mcp add magic-app -- magic-cli mcp --connect 9875
```

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
      "args": ["mcp", "--connect", "9875"]
    }
  }
}
```

## Core Tools & `magic://` Resources

| Surface | Name | Purpose |
| :--- | :--- | :--- |
| **Tools** | `command_list` / `list_commands` | List Magic command IDs, labels, and parameter schemas |
| **Tools** | `command_run` / `run_command` | Run one Magic command (`{id, params?}`), including `studio.mapTracks` |
| **Tools** | `command_batch` / `run_batch` | Run sequential commands with `$1.field` / `$last.field` interpolation |
| **Tools** | `doc_inspect` / `inspect_document` | Inspect `all`, `project`, `storyboard`, or `timeline` |
| **Tools** | `open_file` / `save_file` | Open or save a `.magic` project |
| **Tools** | `render_preview` / `screenshot` | Extract stage preview PNG or capture live window PNG |
| **Tools** | `inspect_ui`, `ui_elements`, `ui_click`, `ui_drag`, `ui_scroll`, `ui_key`, `ui_type`, `ui_control` | Drive the live `magic --control 9875` window (`--connect` only) |
| **Resources** | `magic://document`, `magic://storyboard`, `magic://commands`, `magic://tracks` | Read-only MCP resources for zero-mutation inspection |
