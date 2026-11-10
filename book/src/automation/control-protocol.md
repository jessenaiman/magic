# Magic Studio Control Protocol (`--control`)

`magic --control <PORT>` opens an authenticated loopback JSON-lines socket (`127.0.0.1:9876` by default) so `magic-cli`, `magic-cli mcp` (`magic-mcp`), test harnesses, and agents can drive a live Magic Studio desktop window while you watch every change happen on screen.

Magic Studio is intentionally a **lightweight Tauri coordinator shell** — not a server-side media renderer or frame/tick timeline engine. Frame-accurate timeline-to-storyboard and audio-to-frame synchronization live in [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md) (`filmcraft-cli` / `filmcraft --control`) and [`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md) (`effectcraft-cli` / `effectcraft --control`), which Magic invokes as external CLI tools or forwards to via `ui.effectcraft` and `ui.filmcraft`. Early-alpha [`AniCraft`](https://github.com/storytold/AniCraft) is strictly excluded from live desktop `--control` (`CraftTarget::Ani.allows_live_ui() == false`) and runs headlessly via `anicraft-cli`.

## 1. Every Way to Control Magic Studio

| Surface | Invocation | When to use |
|---|---|---|
| **1. Live CLI (`magic-cli --connect`)** | `magic-cli --connect 127.0.0.1:9876 <subcommand>` | Run one coordinator command (`exec`, `storyboard`, `inspect`) or UI shell action (`ui inspect`, `ui click`, `ui playback`, `ui screenshot`) against the open `magic --control 9876` window. |
| **2. Live or Headless MCP (`magic-cli mcp` / `magic-mcp`)** | `magic-cli mcp --connect 127.0.0.1:9876` or `magic-cli mcp --headless` | Connect Claude Code, Codex, or any [MCP](https://modelcontextprotocol.io) client over stdio ([MCP Server](mcp.md)). With no flag, probes `127.0.0.1:9876` (`MAGIC_CONTROL_PORT`) once and falls back to headless. |
| **3. Headless CLI (`magic-cli`)** | `magic-cli --project song.magic --save exec <id> key=val` | Batch scripts, CI pipelines, and confined `--automation-read-root` / `--automation-write-root` runs driving `<craft>-cli` tools with no window. |
| **4. Raw Loopback TCP Socket** | `127.0.0.1:9876` newline-delimited JSON | Custom harnesses or language bindings speaking JSON-lines directly to `apps/magic/src/control.rs`. |
| **5. Sibling Studio Bridges (`ui.effectcraft`, `ui.filmcraft`, `ui.craft`)** | `magic-cli --connect 127.0.0.1:9876 exec ui.effectcraft --params '{"method":"ui.inspect"}'` | Forward native `--control` calls through Magic to an open [`EffectCraft`](https://github.com/storytold/effectcraft) (`7980`) or [`FilmCraft`](https://github.com/storytold/filmcraft) (`7878`) window when precise timeline/compositor inspection is needed. |

## 2. Starting the Live Control Server

Desktop launchers (`.desktop`, macOS Dock, Windows Start Menu) start `magic` **without** `--control`, so no TCP socket opens unless you opt in explicitly:

```sh
# Start Magic Studio with the loopback control port on 127.0.0.1:9876
magic --control 9876

# Open a project on a chosen view with confined read/write roots
magic --project /work/demo/song.magic --view storyboard --control 127.0.0.1:9876 \
  --automation-read-root /work/demo \
  --automation-write-root /work/demo/out
```

| Variable | Purpose |
|---|---|
| `MAGIC_CONTROL_PORT` | Default `--control` port or `127.0.0.1:<port>` when `--control` is omitted on the CLI (and default address probed by `magic-cli mcp`). |
| `MAGIC_CONTROL_TOKEN` | Explicit 64-character hex authentication token. When unset, `magic --control` generates or reads `~/.config/magic/control-token` (`0600` permissions on Unix). |

Port validation runs **before** opening a window (`1..=65535`, loopback `127.0.0.1` or `::1` only). Passing `0`, a non-loopback address, or a malformed `MAGIC_CONTROL_TOKEN` exits with code `2`.

## 3. Wire Framing & Security Contract

Cross-checked against [`EffectCraft` `docs/control-protocol.md`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md) and [`FilmCraft` `docs/control-protocol.md`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md):

- **Transport**: TCP on `127.0.0.1:<port>` or `[::1]:<port>`, `TCP_NODELAY` enabled, max `16` concurrent connections, `120s` socket read/write timeout, `60s` UI webview reply timeout, and `4 MiB` (`4,194,304` bytes) maximum request line length.
- **Request line** (UTF-8 JSON object terminated by `\n`):
  ```json
  {"id": 1, "token": "<64-hex>", "method": "ui.inspect", "params": {}}
  ```
- **Success / Error reply lines**:
  ```json
  {"id": 1, "ok": true, "result": {"view": "storyboard", "windowFocused": true}}
  {"id": 2, "ok": false, "error": "ui.resize: unknown parameter `foo`; accepted: width, height"}
  ```
- **Browser-smuggling defense (`closing the connection`)**: Any oversized line, non-JSON line (such as an HTTP `POST / HTTP/1.1` header sent by browser `fetch()`), missing `method`, non-object `params`, or invalid `token` (checked in constant time via `subtle::ConstantTimeEq`) writes one `{"id": null, "ok": false, "error": "...; closing the connection"}` line and **immediately drops the TCP stream** (`filmcraft#119`).
- **Strict parameter allow-lists**: Unknown parameter keys on any control method return an error rather than being silently ignored.

## 4. Complete Method Reference (Cross-Checked With `EffectCraft` & `FilmCraft`)

### 4.1 Coordinator Command Dispatch & Discovery

| Method | `magic-cli` Equivalent | Parameters | Description |
|---|---|---|---|
| `engine.execute` (`ui.menu.invoke`) | `magic-cli --connect 127.0.0.1:9876 exec <id> [k=v ...]` | `{"command": "<id>", "params": {...}}` | Run any coordinator command from `magic::command_specs()` on the live session and emit `magic-session-changed` on mutation. |
| `<domain.verb>` (direct) | `magic-cli --connect 127.0.0.1:9876 storyboard check` | Command-specific object | Direct shorthand for `engine.execute` on the control socket. |
| `engine.commands` (`ui.menu.list`) | `magic-cli --connect 127.0.0.1:9876 commands [filter]` | `{"filter"?: "...", "enabledOnly"?: bool}` | List registered coordinator commands (`id`, `label`, `menu`, `shortcut`, `params`, `enabled`, `journal`). |

### 4.2 Live Shell Inspection, Navigation & Audio Preview

| Method | `magic-cli` Equivalent | Parameters | Description |
|---|---|---|---|
| `ui.inspect` (`ui.state`) | `magic-cli --connect 127.0.0.1:9876 ui inspect` | `{}` | Return active `view`, open `project`, `viewport` (`width`, `height`, `devicePixelRatio`), `windowFocused`, `keyboardFocus`, `focus`, active preview `playback`, `alerts`, `errors`, `cspViolations`, `os`, and `notification` state. |
| `ui.drafts` | `magic-cli --connect 127.0.0.1:9876 ui drafts` | `{}` | Report unsaved frontend editor drafts (`{dirty, saving, drafts}`) checked by the non-destructive lifecycle gate. |
| `ui.elements` | `magic-cli --connect 127.0.0.1:9876 ui elements prefix=tab-` | `{"prefix"?: "...", "unidentified"?: bool}` | List visible interactive DOM controls (`id`, `label`, `tag`, `enabled`, `value`, `data`, `media`, `rect: [x, y, w, h]`), or audit widgets missing automation IDs (`unidentified: true`). |
| `ui.set` / `ui.set_view` | `magic-cli --connect 127.0.0.1:9876 ui view storyboard` | `{"view": "home"\|"project"\|"storyboard"\|"design"\|"motion"\|"gallery"\|"animation"\|"studio", "time"?: f64}` OR `{"id": "...", "value": "..."}` OR `{"id": "...", "checked": bool}` | Switch workspace view (with optional authoring timestamp on Lyrics/Motion) or set a form control / checkbox value with native DOM validation. |
| `ui.playback` | `magic-cli --connect 127.0.0.1:9876 ui playback toggle` | `{"action": "play"\|"stop"\|"toggle"\|"status"}` | Control or query the active `<audio>` / `<video>` preview element in the shell (`playing`, `currentTime`, `duration`, `playbackRate`, `muted`, `volume`). |

### 4.3 Pointer, Keyboard, Dialogs, Window Geometry & Lifecycle

| Method | `magic-cli` Equivalent | Parameters | Description |
|---|---|---|---|
| `ui.click` | `magic-cli --connect 127.0.0.1:9876 ui click film-preview` | `{"id"?: "...", "x"?: f64, "y"?: f64, "button"?: "left"\|"middle"\|"right", "count"?: 1..=10, "modifiers"?: ["Ctrl"\|"Shift"\|"Alt"\|"Meta"]}` | Dispatch pointer/mouse click (and `dblclick` when `count == 2`) at an element's center or window coordinates. |
| `ui.move` | `magic-cli --connect 127.0.0.1:9876 ui move x=400 y=300` | `{"id"?: "...", "x"?: f64, "y"?: f64, "modifiers"?: [...]}` | Move pointer to element center or `{x, y}`. |
| `ui.drag` | `magic-cli --connect 127.0.0.1:9876 ui drag --params '{"from":{"id":"a"},"to":{"x":600,"y":300}}'` | `{"from": {"id"\|"x","y"}, "to": {"id"\|"x","y"}, "steps"?: 0..=1024, "button"?: "left", "modifiers"?: [...]}` | Interpolate pointer drag from `from` to `to` (default `12` steps). |
| `ui.scroll` | `magic-cli --connect 127.0.0.1:9876 ui scroll list-rail dy=240` | `{"id"?: "...", "x"?: f64, "y"?: f64, "dx"?: f64, "dy"?: f64, "modifiers"?: [...]}` | Dispatch wheel event and scroll nearest scrollable ancestor. |
| `ui.key` | `magic-cli --connect 127.0.0.1:9876 ui key Ctrl+S` | `{"key": "Enter"\|"Tab"\|"Escape"\|"Ctrl+K"\|..., "modifiers"?: [...]}` | Send `keydown` / default action / `keyup` to the focused element. |
| `ui.keymap.press` | `magic-cli --connect 127.0.0.1:9876 ui keymap.press chord=Cmd+Z` | `{"chord": "..."}` | Trigger a `craft-keymap` shortcut binding directly. |
| `ui.type` (`ui.text`) | `magic-cli --connect 127.0.0.1:9876 ui type "Hello" id=title-input` | `{"text": "...", "id"?: "..."}` | Focus optional `id` and insert up to `10,000` characters into an editable input/textarea. |
| `ui.focus` | `magic-cli --connect 127.0.0.1:9876 ui focus` | `{"id"?: "..."}` | With `id`, focus that DOM element; without `id`, show and focus the native desktop window. |
| `ui.dialog.state` / `ui.dialog.override` | `magic-cli --connect 127.0.0.1:9876 ui dialog.state` | `{}` / `{"next_file"?: "/abs/path", "clear"?: bool}` | Inspect or pre-seed native `craft-dialogs` file-picker responses for unattended automation. |
| `ui.screenshot` | `magic-cli --connect 127.0.0.1:9876 ui screenshot /tmp/magic.png` | `{"path"?: "/abs/file.png"}` | Capture the live WebKitGTK window to an absolute `.png` path (or temp file) and return `{path, width, height, bytes}`. |
| `ui.resize` | `magic-cli --connect 127.0.0.1:9876 ui resize width=1440 height=900` | `{"width": 320..=7680, "height": 240..=4320}` | Resize logical window bounds and verify actual window manager geometry. |
| `app.hide` | `magic-cli --connect 127.0.0.1:9876 ui app.hide` | `{}` | Hide the main desktop window without terminating the session. |
| `app.restart` | `magic-cli --connect 127.0.0.1:9876 ui app.restart` | `{"force"?: bool}` | Run the unsaved-work lifecycle check (unless `force: true`) and restart `magic`. |
| `app.quit` | `magic-cli --connect 127.0.0.1:9876 ui quit` | `{"force"?: bool}` | Run the unsaved-work lifecycle check (unless `force: true`), flush the JSON reply, and exit `0`. |

### 4.4 Sibling ArtCraft `--control` Bridges (`EffectCraft` & `FilmCraft`)

When frame-accurate timeline scrubbing, compositor hit-testing, or audio meter inspection is needed, forward directly to the dedicated ArtCraft engine window:

| Method | Parameters | Description |
|---|---|---|
| `ui.effectcraft` | `{"method": "<effectcraft-method>", "params"?: {...}, "port"?: 1..=65535}` | Forward any method from [`EffectCraft` `docs/control-protocol.md`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md) (`comp.info`, `puppet.*`, `render.frame`, `ui.inspect`, `ui.viewer.locate`, `ui.timeline.locate`) to the live `effectcraft --control` window. |
| `ui.filmcraft` | `{"method": "<filmcraft-method>", "params"?: {...}, "port"?: 1..=65535}` | Forward any method from [`FilmCraft` `docs/control-protocol.md`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md) (`sequence.inspect`, `ui.playback`, `ui.timeline.hit`, `render.frame`) to the live `filmcraft --control` window. |
| `ui.craft` | `{"app": "effectcraft"\|"filmcraft"\|"designcraft"\|"photocraft"\|"vectorcraft"\|"deckcraft"\|"texturecraft", "method": "...", "params"?: {...}, "port"?: 1..=65535}` | Forward a `--control` method to any supported live ArtCraft desktop app (`anicraft` is rejected by safety policy). |

## 5. Storyboard Fine-Print `magic-cli` Instructions (Read-Only)

Every spread inside `crates/engine/templates/storyboard.designcraft` embeds locked, read-only fine-print `magic-cli` instruction frames (`*_cli`) on Layer 2 (`"CLI Fineprint (Read-Only)"`, `Geist Mono` `5.75pt`):

- `magic-cli storyboard pages page=0` returns both editable `slots` and the read-only `cli_instructions` array (`readonly: true`, `locked: true`, `style: "CLI Fineprint"`).
- `magic-cli storyboard add archetype=scene` automatically rewrites the duplicated spread's `*_cli` fine-print labels to the new `page=<N>` index and re-locks them.
- Attempting to overwrite any `*_cli` slot via `magic-cli storyboard set` is rejected by `crates/engine/src/cmd/storyboard/crud.rs`.

## 6. Review Checklist: Nice-to-Have `--control` Enhancements (`EffectCraft` & `FilmCraft` Comparison)

Controls present in [`EffectCraft` `docs/control-protocol.md`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md) or [`FilmCraft` `docs/control-protocol.md`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md) that should be reviewed as optional lightweight shell enhancements for Magic Studio:

### Security & Window Focus Ergonomics
- [ ] **`--control-input-only` startup flag** — Restrict the `--control` socket to `ui.*` inspection/input methods and block direct `engine.execute` mutations (`EffectCraft` / `FilmCraft` input-only harness mode).
- [ ] **Non-activating background window bring-forward** — Keep `magic --control` visible above background windows without stealing keyboard focus from the user's terminal (`FilmCraft` background window ordering).
- [ ] **macOS App Nap opt-out while `--control` is active** — Prevent macOS from throttling webview timers during unattended `--control` sessions (`EffectCraft` / `FilmCraft`).

### Cropped Captures & Cross-Platform Webview Snapshots
- [ ] **`ui.screenshot {panel?, id?}` element/region crop** — Crop `ui.screenshot` directly to a named workspace region or DOM element ID (`EffectCraft` / `FilmCraft` `ui.screenshot {panel}`).
- [ ] **macOS (`WKWebView`) and Windows (`WebView2`) `ui.screenshot` backends** — Extend `apps/magic/src/capture.rs` beyond Linux `WebKitGTK`.
- [ ] **Optional `perf.stats` shell telemetry** — Expose lightweight webview frame timing and memory counters without duplicating `EffectCraft` / `FilmCraft` render-cache telemetry.

### Stage & Timeline Coordinate Locators (Delegated vs Shell)
- [ ] **`ui.viewer.locate` and `ui.viewer.hit` on Storyboard/Lyrics stage** — Map stage coordinates `(x, y)` to window pixels for interactive word-box and storyboard-slot drag tests (compositor-level `ui.viewer.locate` already runs in `EffectCraft` via `ui.effectcraft`).
- [ ] **`ui.timeline.locate` and `ui.timeline.hit` on Lyrics/Motion cue strip** — Map song seconds to window coordinates on Magic's lightweight cue strip (tick-accurate NLE timeline hit-testing remains in `FilmCraft` via `ui.filmcraft`).
- [ ] **Read-only MCP resources (`magic://document`, `magic://storyboard`, `magic://commands`)** — Expose MCP `resources/list` and `resources/read` URIs alongside tool calls in `crates/mcp` (`DesignCraft` MCP resource parity).
