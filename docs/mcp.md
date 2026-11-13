# MCP Server (`magic-cli mcp`)

Magic Studio exposes its MCP server directly from `magic-cli` (`crates/mcp`, JSON-RPC 2.0 over stdio), following [`storytold/deckcraft/docs/mcp.md`](https://github.com/storytold/deckcraft/blob/main/docs/mcp.md) and [`storytold/designcraft/docs/mcp.md`](https://github.com/storytold/designcraft/blob/main/docs/mcp.md).

## Install & Register `magic-cli mcp`

Build and install `magic`, `magic-cli`, and `magic-tui` into `~/.local/bin` (or install `magic-cli` into `~/.cargo/bin`):

```sh
./install.sh --from-source
# or: cargo install --path apps/magic-cli
```

Register headless and live `--control` MCP servers:

```sh
# 1. Headless session (isolated in-process Session, zero network sockets)
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

- **Headless by default (`magic-cli mcp`)**: Runs an isolated in-process `magic_engine::Session` over stdio with zero network sockets.
- **Read-only `magic://` resources**: Exposes `magic://` resource URIs (`magic://document`, `magic://storyboard`, `magic://commands`, `magic://tracks`) so agents can read project, storyboard, command, and track state without mutating the session.
- **Live window (`magic-cli mcp --connect 9875` or `127.0.0.1:9875`)**: Forwards tool calls over loopback TCP to a running `magic --control 9875` window, bounds replies at `16 MiB`, and never replays mutating requests on lost replies (`outcome unknown`).
- **Confined headless (`magic-cli mcp --automation-read-root <DIR> --automation-write-root <DIR>`)**: Canonicalizes roots and target paths via `std::fs::canonicalize` to confine headless reads and writes (`--connect` together with root flags is rejected).
- **Crafting App MCP Servers**: Register each installed Craft app's `<slug>-cli mcp` directly from its upstream `README.md` and `docs/mcp.md` ([`DeckCraft`](https://github.com/storytold/deckcraft/blob/main/docs/mcp.md), [`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/docs/mcp.md), [`PhotoCraft`](https://github.com/storytold/photocraft/blob/main/docs/mcp.md), [`VectorCraft`](https://github.com/storytold/vectorcraft/blob/main/docs/mcp.md), and [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/docs/agents.md)).

## Core Tools & Resources

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
