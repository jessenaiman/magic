# Control Protocol (`magic --control`)

Magic Studio exposes the standard ArtCraft loopback JSON-lines control channel for `magic-cli`, automated tests, and `magic-cli mcp --connect`:

```sh
magic --control 9875          # or MAGIC_CONTROL_PORT=9875
```

- **Transport**: TCP on `127.0.0.1` (`Ipv4Addr::LOCALHOST`) only; `--control 0` picks a free port and prints `MAGIC_CONTROL_PORT=<port>`, one JSON object per line in each direction, persistent connections, up to 16 concurrent clients, 16 MiB max line, 60 s UI reply timeout with expired-in-queue skip (`timeout: not-run` vs `timeout: outcome unknown`).
- **Request**: `{"id": 1, "method": "engine.execute", "params": {"command": "workshop.status", "params": {}}}`
- **Reply**: `{"id": 1, "ok": true, "result": ...}` or `{"id": 1, "ok": false, "error": "..."}`
- **Authentication**: Unauthenticated on loopback by default (matching `EffectCraft`, `DeckCraft`, `DesignCraft`, and `VectorCraft`). If `MAGIC_CONTROL_TOKEN` is set in the environment when `magic --control` starts, each request must include `"token": "<value>"` (checked in constant time).
- **Smuggling protection**: Any non-JSON line, missing `method`, invalid UTF-8, or oversized line receives one error ending in `"; closing the connection"` and closes the socket immediately.

## Quick Test

```sh
printf '{"id":1,"method":"engine.commands","params":{}}\n' | nc 127.0.0.1 9875
```

## Control Methods

| Method | Params | Result |
| :--- | :--- | :--- |
| `engine.execute` / `command` | `{command, params?}` | Execute any Magic engine command in the live session |
| `engine.commands` | `{filter?, enabledOnly?}` | Command registry (`id, label, menu, shortcut, params, enabled`) |
| `document.inspect` | `{}` | Open `.magic` project snapshot (`project.snapshot`) |
| `ui.inspect` / `ui.state` | `{}` | Active view, panel tabs, viewport size, focus, dialog, playback |
| `ui.elements` | `{prefix?, unidentified?}` | Visible `egui` widgets (`[{id, label, rect}]`) |
| `ui.menu.list` / `ui.menu.invoke` | `{}` / `{id, params?}` | Menu tree or invoke menu item |
| `ui.set` | `{view?, studioWorkspace?, theme?, style?, id?, value?}` | Mutate active view, workspace tab, theme, or widget |
| `ui.click` / `ui.move` / `ui.drag` / `ui.scroll` | `{id / x, y, ...}` | Synthetic pointer input |
| `ui.key` / `ui.keymap.press` / `ui.type` / `ui.text` | `{key}` / `{chord}` / `{text}` | Synthetic keyboard or text input |
| `ui.viewer.locate` / `ui.viewer.hit` | `{x, y}` | Stage ↔ window coordinate mapping |
| `ui.timeline.locate` / `ui.timeline.hit` | `{seconds / tick}` / `{x, y}` | Timeline ↔ window coordinate mapping |
| `ui.playback` | `{action: play/stop/toggle/status, seconds?}` | Local song audio preview transport |
| `ui.screenshot` / `render.frame` | `{path?, panel?, id?}` | Capture window/panel PNG or stage preview frame |
| `ui.resize` / `ui.focus` / `window.snap` | `{width, height}` / `{id?}` / `{}` | Window geometry and focus |
| `app.hide` / `app.restart` / `app.quit` | `{force?}` | Lifecycle controls |
