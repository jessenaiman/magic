# Magic Studio Control Protocol (`--control`)

`magic --control [PORT]` (alias `--controls`) opens an authenticated loopback JSON-lines socket (`127.0.0.1:9875` by default when omitted) so `magic-cli`, `magic-cli mcp` (`magic-mcp`), `magic-tui`, test harnesses, and agents can drive a live pure-Rust `eframe` 0.37 + `egui` 0.37 Magic Studio window while watching every change live.

Magic Studio is a **pure-Rust ArtCraft coordinator shell** — frame-accurate timeline editing, color grading, audio DSP, and compositing live in [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md) (`filmcraft --control 9876` + `FILMCRAFT_CONTROL_TOKEN` / `filmcraft-cli`) and [`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md) (`effectcraft --control 9877` / `effectcraft-cli mcp --autosave`), with [`DesignCraft`](https://github.com/storytold/designcraft/blob/main/docs/control-protocol.md) (`designcraft --control 7979`) for storyboards. Early-alpha [`AniCraft`](https://github.com/storytold/AniCraft) is strictly excluded from live desktop `--control` (`CraftTarget::Ani.allows_live_ui() == false`) and runs headlessly via `anicraft-cli`.

## 1. Non-Colliding ArtCraft Control Port Allocation

| Application | Default Loopback `--control` | Auth / Auto-Save Contract | CLI / MCP Bridge |
|---|---|---|---|
| **Magic Studio** | `127.0.0.1:9875` (`MAGIC_CONTROL_PORT`) | `MAGIC_CONTROL_TOKEN` (`~/.config/magic/control-token`, `0600`) | `magic-cli --connect 127.0.0.1:9875` · `magic-cli mcp --connect 127.0.0.1:9875` |
| **FilmCraft** | `127.0.0.1:9876` (`FILMCRAFT_CONTROL_PORT`) | `FILMCRAFT_CONTROL_TOKEN` (`64` hex chars) | `filmcraft-cli --bridge 127.0.0.1:9876` · `filmcraft-cli mcp --bridge 127.0.0.1:9876` |
| **EffectCraft** | `127.0.0.1:9877` (`EFFECTCRAFT_CONTROL_PORT`) | Loopback JSON-lines; headless `mcp --autosave` | `effectcraft-cli --bridge 9877` · `effectcraft-cli mcp --bridge 9877` |
| **DesignCraft** | `127.0.0.1:7979` (`DESIGNCRAFT_CONTROL_PORT`) | Loopback JSON-lines | `designcraft-cli mcp --connect 127.0.0.1:7979` |
| **PhotoCraft** | `127.0.0.1:9879` (`PHOTOCRAFT_CONTROL_PORT`) | `--control-token-file` (`FirstFrameAuth`) | `photocraft-cli mcp --bridge 127.0.0.1:9879` |
| **VectorCraft** | `127.0.0.1:9880` (`VECTORCRAFT_CONTROL_PORT`) | Loopback JSON-lines | `vectorcraft-cli mcp --connect 127.0.0.1:9880` |
| **DeckCraft** | `127.0.0.1:7990` (`DECKCRAFT_CONTROL_PORT`) | Loopback JSON-lines | `deckcraft-cli mcp --connect 127.0.0.1:7990` |

## 2. Starting the Live Control Server

```sh
# Start Magic Studio on default loopback port 127.0.0.1:9875 (without stealing OS focus)
magic --control

# Explicit port + input-only harness mode + confined roots
magic --project /work/demo/song.magic --view studio --control 9875 --control-input-only \
  --automation-read-root /work/demo \
  --automation-write-root /work/demo/out

# Dump all coordinator commands and --control methods as JSON
magic --list-controls
magic-cli controls --stage studio
```

Port and `MAGIC_CONTROL_TOKEN` validation run **before** opening a window (`1..=65535`, loopback `127.0.0.1` or `::1` only). Invalid flags exit immediately with status `2`.

## 3. Complete `--control` Method Reference

| Method | `magic-cli` Equivalent | Parameters | Description |
|---|---|---|---|
| `engine.execute` | `magic-cli --connect 9875 exec <id> [k=v ...]` | `{"command": "<id>", "params": {...}}` | Run any coordinator command on the live session (blocked for mutations when `--control-input-only` is set). |
| `engine.commands` | `magic-cli --connect 9875 commands [filter]` | `{"filter"?: "...", "enabledOnly"?: bool}` | List coordinator commands with live enablement. |
| `ui.inspect` (`ui.state`) | `magic-cli --connect 9875 ui inspect` | `{}` | Return `view`, `panels` (`studioWorkspace`, `studioScopes`, `animTab`, `theatreTab`, `lyricsTab`, `motionTab`), `input` (`mode`: `normal`\|`controlInputOnly`), `dirtyTracks`, `bridges` (`:9876`, `:9877`), `keyboardFocus`, `playback`, and `viewport`. |
| `ui.elements` | `magic-cli --connect 9875 ui elements tab-` | `{"prefix"?: "...", "unidentified"?: bool}` | List visible `egui` widget automation IDs, labels, enabled state, and logical rects. |
| `ui.set` (`ui.set_view`) | `magic-cli --connect 9875 ui workspace color` | `{"view"?, "time"?, "studioWorkspace"?, "studioScopes"?, "animTab"?, "theatreTab"?, "lyricsTab"?, "motionTab"?, "theme"?, "style"?, "accent"?, "highContrast"?, "uiScalePct"?}` OR `{"id", "value"\|"checked"}` | Switch stage views, Sheet 08 supplementary tabs/workspaces, appearance presets, or widget values. |
| `ui.viewer.locate` / `ui.viewer.hit` | `magic-cli --connect 9875 ui locate x=960 y=540` | `{"x", "y", "width"?, "height"?, "stage"?}` | Map stage coordinates `(x, y)` to window points and back across Storyboard, Animations, Lyrics, Motion, and Studio. |
| `ui.timeline.locate` / `ui.timeline.hit` | `magic-cli --connect 9875 ui timeline-locate time=4.2` | `{"time", "track"?, "id"?}` / `{"x", "y", "id"?}` | Map song seconds and `254,016,000,000` ticks/sec to logical window coordinates on the active Craft timeline. |
| `render.frame` | `magic-cli --connect 9875 ui render-frame /tmp/f.png` | `{"stage"?, "time"?, "page"?, "path"?}` | Render a stage frame PNG via FilmCraft, EffectCraft, or DesignCraft. |
| `ui.screenshot` | `magic-cli --connect 9875 ui screenshot /tmp/ui.png` | `{"path"?, "panel"?, "id"?}` | Capture the live `eframe`/`wgpu` window, panel, or widget automation ID to PNG. |
| `ui.click` / `ui.move` / `ui.drag` / `ui.scroll` | `magic-cli --connect 9875 ui click studio-ws-color` | `{id \| x, y, ...}` | Inject synthetic `egui` pointer/wheel gestures. |
| `ui.key` / `ui.keymap.press` / `ui.type` | `magic-cli --connect 9875 ui key Cmd+Z` | `{"key"}` / `{"chord"}` / `{"text", "id"?}` | Inject `craft-keymap` shortcuts or text input. |
| `ui.filmcraft` / `ui.effectcraft` / `ui.craft` | `magic-cli bridge fc sequence.inspect` | `{"action"?: "status"\|"launch"\|"focus"\|"close", "command"?, "method"?, "params"?, "port"?}` | Launch, inspect, or forward commands/methods to `FilmCraft (:9876)` and `EffectCraft (:9877)`. |
| `perf.stats` | `magic-cli --connect 9875 ui perf` | `{}` | Return `eframe-wgpu` FPS, `lastRenderMs`, `pixelsPerPoint`, and `ticksPerSecond`. |
| `app.hide` / `app.restart` / `app.quit` | `magic-cli --connect 9875 ui quit` | `{"force"?: bool}` | Guarded window lifecycle operations. |

## 4. Verification Checklist (`EffectCraft` & `FilmCraft` Parity)

- [x] **`--control-input-only` startup flag** — Restricts `--control` to `ui.*` inspection/input methods and suppresses physical host input during unattended harnesses.
- [x] **Non-activating background launch (`with_active(cli.control.is_none())`)** — Opens `magic --control` without stealing keyboard focus.
- [x] **`ui.screenshot {panel?, id?}` cropped captures** — Captures full window, named panel, or individual widget rect via `eframe`/`wgpu`.
- [x] **`perf.stats` telemetry** — Reports `eframe-wgpu` frame timing and `254,016,000,000` tick rate.
- [x] **`ui.viewer.locate` / `ui.viewer.hit` & `ui.timeline.locate` / `ui.timeline.hit`** — Implemented across all stages alongside native `ui.filmcraft` (`:9876`) and `ui.effectcraft` (`:9877`) pass-throughs.
