# Control Protocol (`magic --control`)

> **Last Reviewed & Updated:** 2026-11-12T22:48:11-05:00 · **Reviewed by:** Jesse A. Naiman

Magic Studio exposes the standard ArtCraft loopback JSON-lines control channel ([`apps/magic/src/control.rs`](../apps/magic/src/control.rs)) for [`magic-cli`](../apps/magic-cli/README.md), automated tests, and [`magic-cli mcp --connect`](mcp.md). Read the canonical protocol design in [`storytold/designcraft/docs/control-protocol.md`](https://github.com/storytold/designcraft/blob/main/docs/control-protocol.md):

```sh
magic --control 9875          # or MAGIC_CONTROL_PORT=9875
```

- **Transport**: Opt-in TCP on `127.0.0.1` (`Ipv4Addr::LOCALHOST`) only; `--control 0` picks a free port and prints `MAGIC_CONTROL_PORT=<port>`, one JSON object per line in each direction, persistent connections, up to 16 concurrent clients, `16 MiB` max line, and `60 s` UI reply timeout with expired-in-queue skip (`timeout: not-run` vs `timeout: outcome unknown`).
- **Request**: `{"id": 1, "method": "engine.execute", "params": {"command": "workshop.status", "params": {}}}`
- **Reply**: `{"id": 1, "ok": true, "result": ...}` or `{"id": 1, "ok": false, "error": "..."}`
- **Authentication**: Unauthenticated on `127.0.0.1`, matching [`DesignCraft` `docs/control-protocol.md`](https://github.com/storytold/designcraft/blob/main/docs/control-protocol.md) — enable `--control` only while actively automating or testing.
- **Framing & Smuggling Protection**: Any non-JSON line, missing `method`, invalid UTF-8, or oversized line receives one error ending in `"; closing the connection"` and closes the socket immediately.

## Quick Test

```sh
printf '{"id":1,"method":"ui.docking","params":{}}\n' | nc 127.0.0.1 9875
```

## Control Methods (`apps/magic/src/control.rs`)

| Method | Params | Result |
| :--- | :--- | :--- |
| `engine.execute` / `command` | `{command, params?}` | Execute any Magic engine command in the live session |
| `engine.commands` | `{filter?, enabledOnly?}` | Command registry (`id, label, menu, shortcut, params, enabled`) |
| `document.inspect` | `{}` | Open `.magic` project snapshot (`project.snapshot`) |
| `ui.inspect` / `ui.state` | `{}` | Active view, `docking` workspace summary, viewport size, focus, dialog, playback |
| `ui.docking` | `{}` | Full [`craft_ui::docking`](https://github.com/storytold/craft-libs/blob/main/docs/ui-docking.md) hit-rect snapshot (`DockArea::hit_rects`: active preset, docked tabs/groups/columns/dividers, floating windows, native OS floating rects, and canonical `DockCommand::IDS`) |
| `ui.elements` | `{prefix?, unidentified?}` | Visible `egui` widgets (`[{id, label, rect}]`) |
| `ui.menu.list` / `ui.menu.invoke` | `{}` / `{id, params?}` | Menu tree or invoke menu item (including `DockCommand::IDS` such as `window.float_panel`, `window.maximize_group`, `window.reset_workspace`, and `window.panel.<id>`) |
| `ui.set` | `{view?, workspacePreset?, studioWorkspace?, theme?, style?, id?, value?}` | Mutate active stage view, `craft_ui::docking` workspace preset, theme, or widget |
| `ui.click` / `ui.move` / `ui.drag` / `ui.scroll` | `{id / x, y, ...}` | Synthetic pointer input (drags tabs/dividers across `ui.docking` hit rects) |
| `ui.key` / `ui.keymap.press` / `ui.type` / `ui.text` | `{key}` / `{chord}` / `{text}` | Synthetic keyboard or text input |
| `ui.viewer.locate` / `ui.viewer.hit` | `{x, y}` | Stage ↔ window coordinate mapping (`crates/ui-egui/src/editorial.rs`) |
| `ui.timeline.locate` / `ui.timeline.hit` | `{seconds / tick}` / `{x, y}` | Timeline ↔ window coordinate mapping (`crates/ui-egui/src/timeline.rs`) |
| `ui.playback` | `{action: play/stop/toggle/status, seconds?}` | Local song audio transport + live `craft-audio` L/R `rmsDbfs` & `peakDbfs` meters (`apps/magic/src/audio_player.rs`) |
| `ui.screenshot` | `{path?, panel?, id?}` | Capture window/panel PNG via `craft_ui::capture::CaptureSlot` |
| `render.frame` | `{stage?, page?, time?, path?, max_side?, base64?, write?}` | Render stage preview frame with optional longest-side cap (`max_side`) and inline PNG `base64` |
| `ui.resize` / `ui.focus` / `window.snap` | `{width, height}` / `{id?}` / `{}` | Window geometry and focus |
| `app.hide` / `app.restart` / `app.quit` | `{force?}` | Lifecycle controls |
