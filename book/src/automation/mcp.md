# Magic Studio MCP server

`magic-cli mcp` (and the companion `magic-mcp` binary) runs a lightweight [Model Context Protocol](https://modelcontextprotocol.io) server on stdio (newline-delimited JSON-RPC 2.0), modeled directly on [VectorCraft](https://github.com/storytold/vectorcraft/blob/main/README.md) and [DesignCraft MCP](https://github.com/storytold/designcraft/blob/main/docs/mcp.md).

Magic Studio is a **pure-Rust `eframe` + `egui` 0.37 coordinator shell**, not a server-side compositor or timeline clock: `magic-cli mcp` orchestrates `.magic` project files, [DesignCraft](https://github.com/storytold/designcraft) storyboards, [PhotoCraft](https://github.com/storytold/photocraft) cutouts, [VectorCraft](https://github.com/storytold/vectorcraft) SVG traces, [EffectCraft](https://github.com/storytold/effectcraft) `.ecproj` puppet/lyric/motion compositions, and [FilmCraft](https://github.com/storytold/filmcraft) `.fcproj` timelines by invoking their real `<craft>-cli` binaries (and forwarding live `--control` calls when watching the desktop app). When frame-accurate NLE or compositor timing is needed directly, pair `magic-cli mcp` with [FilmCraft MCP](https://github.com/storytold/filmcraft/blob/main/docs/mcp-server.md) and [EffectCraft MCP](https://github.com/storytold/effectcraft/blob/main/docs/mcp-server.md).

It has two backends:

| Mode | How | What works |
|---|---|---|
| **Remote** | `magic-cli mcp --connect 127.0.0.1:9876` (alias `--bridge`; the desktop app must run with `magic --control 9876`) | Everything. Tool calls are forwarded over the [control protocol](control-protocol.md), waking the `eframe` event loop (`ctx.request_repaint()`) so you watch the Magic Studio window update live. |
| **Headless** | `magic-cli mcp --headless` | An in-process `magic::Session` driving installed `<craft>-cli` binaries. Everything except the live UI tools (`inspect_ui`, `ui_elements`, `ui_click`, `ui_drag`, `ui_scroll`, `ui_key`, `ui_type`, `screenshot {window:true}`, `ui_control`). |

With no flag, `magic-cli mcp` tries `127.0.0.1:9876` (`MAGIC_CONTROL_PORT`) once when it starts and falls back cleanly to headless. Logs go to stderr; stdout carries only MCP JSON-RPC protocol messages.

## Install and register

```sh
./install.sh --from-source
# or CLI + MCP binaries only:
cargo install --path apps/magic-cli
export PATH="$HOME/.local/bin:${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"

magic-cli --help
claude mcp add magic -- magic-cli mcp
# or pin a mode explicitly:
claude mcp add magic-headless -- magic-cli mcp --headless
claude mcp add magic-app -- magic-cli mcp --connect 127.0.0.1:9876
```

Other MCP clients use the same command in `.mcp.json`:

```json
{
  "mcpServers": {
    "magic": {
      "command": "magic-cli",
      "args": ["mcp"]
    },
    "magic-app": {
      "command": "magic-cli",
      "args": ["mcp", "--connect", "127.0.0.1:9876"]
    }
  }
}
```

### Live session (`--control` watching)

Start the desktop app with `--control` **before** the MCP client starts the server:

```sh
magic --control 9876
# or from a source checkout:
cargo run --release -p magic -- --control 9876
```

With `--connect`, a server that finds no running app stops immediately and prints:

```text
magic-cli: cannot connect to 127.0.0.1:9876: Connection refused (os error 111)
```

## Confining file access

```sh
magic-cli mcp --automation-read-root /work/project --automation-write-root /work/project/out
```

- Every file read (`open_file`, `file.open`, `audio`, `transcript`, `image`, `source_project`, `references`) must resolve inside an `--automation-read-root`, with symlinks rejected.
- Every file write (`save_file`, `file.new`, `file.save`, `file.saveAs`, `storyboard.export`, `video.export`, `screenshot {path}`) must resolve inside an `--automation-write-root`.
- Passing `--automation-read-root` or `--automation-write-root` to `magic-cli mcp` implies `--headless` (pass them to `magic --control 9876` to confine a live window).

## Tools

`magic-cli mcp` exposes a single clean tool set:

| Tool | Backend | Description |
|---|---|---|
| `list_commands` | Both | List coordinator command IDs, labels, and parameter schemas (`{filter?, enabled_only?}`). |
| `run_command` | Both | Run one coordinator command (`{command, params?}`). |
| `run_batch` | Both | Run multiple coordinator commands sequentially (`{steps: [{command, params?}], stop_on_error?}`). |
| `inspect_document` | Both | Inspect open `.magic` project, storyboard traffic-light readiness, and FilmCraft timeline (`{target?: "all" \| "project" \| "storyboard" \| "timeline"}`). |
| `open_file` | Both | Open a `.magic` project (`{path}`). |
| `save_file` | Both | Save the open `.magic` project (`{path?}`; runs `file.save` or `file.saveAs`). |
| `inspect_ui` | `--connect` | Inspect live `eframe`/`egui` view, theme, focus, FPS, dialogs, drafts, and transport state. |
| `ui_elements` | `--connect` | List on-screen `egui` widget automation IDs, labels, enabled state, and logical-point bounds (`{prefix?, unidentified?}`). |
| `ui_click` | `--connect` | Click an enabled widget by `{id, fx?, fy?}` or logical window `{x, y}`. |
| `ui_drag` | `--connect` | Drag between `{id, fx?, fy?}` or `{x, y}` endpoints (`steps` bounded `0..=1024`). |
| `ui_scroll` | `--connect` | Scroll a widget or logical point by `{dx, dy}`. |
| `ui_key` | `--connect` | Send a key or chord (`Enter`, `Escape`, `Cmd+S`, `Physical+KeyZ`) through `craft-keymap`. |
| `ui_type` | `--connect` | Type text into the focused `egui` text field or `{id}`. |
| `screenshot` | `--connect` (or `window:false` headless) | Capture the live `eframe` window to PNG via `craft-ui` `CaptureSlot` (`{path?, panel?, id?}`), or render a DesignCraft storyboard spread headlessly with `screenshot {window: false, page: 0}`. |
| `ui_control` | `--connect` | Invoke live shell methods (`ui.set`, `ui.menu.list`, `ui.menu.invoke`, `ui.playback`, `ui.focus`, `ui.effectcraft`, `ui.filmcraft`, `perf.stats`, `app.quit`). |
