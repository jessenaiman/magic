# Control Protocol (`magic --control`)

Magic Studio is a **pure-Rust `eframe` + `egui` 0.37 desktop coordinator shell** built on [`storytold/craft-libs`](https://github.com/storytold/craft-libs) (`craft-ui`, `craft-menu`, `craft-window`, `craft-dialogs`, `craft-keymap`, `craft-platform`, `craft-quit`). It coordinates song-first storyboards, character cutouts, kinetic lyrics, and multi-track assemblies across installed ArtCraft studio binaries ([`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md), [`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md), [`DesignCraft`](https://github.com/storytold/designcraft), [`PhotoCraft`](https://github.com/storytold/photocraft/blob/main/docs/control-protocol.md), and [`VectorCraft`](https://github.com/storytold/vectorcraft)). Early-alpha [`AniCraft`](https://github.com/storytold/AniCraft) is strictly excluded from live `--control` (`CraftTarget::Ani.allows_live_ui() == false`; `anicraft-cli` headless only).

For MCP registration and headless vs live routing, see [`docs/mcp.md`](mcp.md).

## 1. Startup, Loopback Binding & Authentication

```sh
# Launch Magic Studio listening on loopback (default 127.0.0.1:9876):
magic --control 9876 &

# Confine live control reads/writes to explicit workspace roots:
magic --control 127.0.0.1:9876 \
  --automation-read-root ~/Videos/Magic \
  --automation-write-root ~/Videos/Magic &
```

- **Loopback Only**: `--control` (`MAGIC_CONTROL_PORT`) accepts a port (`1..=65535`, bound on `127.0.0.1`) or an explicit loopback `SocketAddr` (`127.0.0.1:<port>` or `[::1]:<port>`). Non-loopback addresses exit with code `2`.
- **Non-Activating Startup**: When `--control` is passed, Magic configures `egui::ViewportBuilder::with_active(false)` so automated launches do not steal OS keyboard focus from your editor or terminal.
- **64-Hex Token Authentication**: Startup ensures a 64-hex-character secret in `~/.config/magic/control-token` (`0600` permissions) or reads `MAGIC_CONTROL_TOKEN`. Every request line must include `"token": "<64-hex>"`, verified in constant time (`subtle::ConstantTimeEq`). Never pass `MAGIC_CONTROL_TOKEN` on CLI arguments or commit it in `.mcp.json`.
- **Sibling Bridges**: `CraftTarget::default_port` assigns distinct defaults when Magic launches sibling bridges (`FilmCraft` `9875` / `FILMCRAFT_CONTROL_PORT`, `EffectCraft` `9878` / `EFFECTCRAFT_CONTROL_PORT`); pass an explicit `port` in `ui.filmcraft` / `ui.effectcraft` whenever attaching to a standalone `filmcraft --control 9876` or `effectcraft --control 9877` window.

## 2. Transport & Per-Frame `eframe` Drain

- **Framing**: Newline-delimited JSON (`\n`), capped at `4 MiB` per request line and `16` concurrent loopback connections (`apps/magic/src/control.rs`).
- **Smuggling Defense (`filmcraft#119`)**: Any non-JSON, oversized, unauthenticated, or malformed line replies with `{"id": null, "ok": false, "error": "...; closing the connection"}` and immediately drops the TCP socket so cross-origin browser `fetch()` smuggling cannot pipeline requests.
- **UI Thread Execution**: Requests wake the `eframe` event loop (`ctx.request_repaint()`) and drain inside `eframe::App::logic` (`60s` reply timeout). Synthetic pointer/keyboard/text commands (`ui.click`, `ui.drag`, `ui.scroll`, `ui.key`, `ui.type`) inject `egui::Event`s in `raw_input_hook` and reply after the next `egui` frame processes them.

## 3. Dispatch, Menus & Inspection Methods

| Method | Parameters | Behavior |
|---|---|---|
| `engine.execute` | `{command, params?}` | Executes a headless/engine command on the live `Session` without opening modal pickers; refreshes cached view data and triggers `ctx.request_repaint()`. |
| `engine.commands` | `{filter?, enabledOnly?}` | Lists registered `CommandSpec` entries and their live `(spec.enabled)(&session)` status. |
| `ui.menu.list` | `{}` | Returns the 9-menu [`craft-menu`](https://github.com/storytold/craft-libs/blob/main/crates/menu/README.md) bar (`File`, `Edit`, `View`, `Storyboard`, `Lyrics`, `Character`, `Film`, `Window`, `Help`) with live `enabled` states. |
| `ui.menu.invoke` | `{id, params?}` | Runs a menu command just like clicking it in the `craft-menu` bar (opening a `craft-dialogs` picker or `egui` modal when `params` are omitted). |
| `ui.inspect` (`ui.state`) | `{}` | Returns `{renderer: "eframe", view, requestedView, theme, project, viewport: {width, height, pixelsPerPoint}, fps, lastRenderMs, nativeMenuBar, windowFocused, keyboardFocus, dialog, drafts, playback}`. |
| `ui.drafts` | `{}` | Reports `{dirty, saving, drafts}` from in-process `egui` stage editors. |
| `ui.elements` | `{prefix?, unidentified?}` | Lists `{id, label, enabled, value?, rect: [x, y, w, h]}` in logical points for every widget registered in the last `egui` frame. |
| `ui.set` (`ui.set_view`) | `{view?, time?, theme?, focused?}` or `{id, value\|checked}` | Switches workstation views (`home`, `project`, `storyboard`, `design`/`lyrics`, `motion`, `animation`, `studio`), sets `theme` (`dark`, `light`, `system`), or updates an on-screen `egui` input/checkbox by `id`. |
| `ui.playback` | `{action?: "play"\|"stop"\|"toggle"\|"status"}` | Controls or queries the song transport clock. |

## 4. Synthetic `egui` Input, Dialogs, Screenshots & Lifecycle

| Method | Parameters | Behavior |
|---|---|---|
| `ui.click` | `{id, fx?, fy?}` or `{x, y}`, `{button?, count?, modifiers?}` | Moves pointer and clicks (`count` bounded `0..=10`, default `1`; `fx`/`fy` in `0.0..=1.0`; `modifiers`: `{shift?, alt?, command?, ctrl?}`). |
| `ui.move` | `{id, fx?, fy?}` or `{x, y}` | Injects `egui::Event::PointerMoved` in logical points. |
| `ui.drag` | `{from, to, button?, steps?, modifiers?}` | Interpolates a pointer drag (`steps` bounded `0..=1024`, default `12`, `0\|1 -> 2` per [`filmcraft#2784`](https://github.com/storytold/filmcraft/issues/2784)). |
| `ui.scroll` | `{id?, x?, y?, dx?, dy?, unit?, source?, modifiers?}` | Injects `egui::Event::MouseWheel` (`unit`: `"point"` or `"line"`, `source`: `"wheel"` or `"trackpad"`). |
| `ui.key` | `{key, modifiers?}` | Routes chords through [`craft-keymap`](https://github.com/storytold/craft-libs/blob/main/crates/keymap/README.md) or injects `egui::Event::Key` (`Enter`, `Escape`, `Tab`, `ArrowDown`, `Cmd+S`, `Physical+KeyZ`). |
| `ui.keymap.press` | `{chord, typing?}` | Evaluates a `craft-keymap` chord directly against the command catalog. |
| `ui.type` (`ui.text`) | `{text, id?}` | Optionally focuses widget `id`, then injects `egui::Event::Text(text)`. |
| `ui.focus` | `{id?}` | `ui.focus {}` raises and focuses the OS window (`ViewportCommand::Focus`); `ui.focus {id}` requests `egui` keyboard focus on an enabled widget **without** stealing OS focus. |
| `ui.resize` | `{width, height}` | Resizes logical inner viewport (`320..=7680` x `240..=4320`). |
| `window.snap` | `{}` | Toggles left/right desktop window snap. |
| `ui.dialog.state` / `ui.dialog.override` | `{}` / `{paths?, button?, cancelled?, unavailable?, clear?}` | Inspects or scripts [`craft-dialogs`](https://github.com/storytold/craft-libs) without opening blocking native sheets. Unsaved-close modal buttons register `dialog.unsaved.save`, `dialog.unsaved.dontSave`, and `dialog.unsaved.cancel`. |
| `ui.screenshot` | `{path?, panel?, id?}` | Captures a `wgpu` PNG via [`craft-ui` `CaptureSlot`](https://github.com/storytold/craft-libs/blob/main/crates/ui/README.md) (`10s` presentation timeout), optionally cropped to `panel` (`app-bar`, `stage-assets`, `central`, `app-footer`) or widget `id`, written with `craft-atomic` (`0600` private dir when `path` is omitted). |
| `perf.stats` | `{}` | Returns `{renderer: "eframe-wgpu", fps, lastRenderMs, pixelsPerPoint}`. |
| `ui.filmcraft` / `ui.effectcraft` / `ui.craft` | `{app?, method, params?, port?}` | Proxies live `--control` commands to open studio windows (`AniCraft` refused). |
| `app.hide` / `app.quit` / `app.restart` | `{force?}` | Hides, quits, or restarts cleanly through `lifecycle::verify_clean` and `craft_quit::on_close_request`. |

## 5. Review Checklist of Nice-to-Have `--control` Enhancements

- [ ] **`--control-input-only` startup flag** — Accept synthetic widget input over `--control` while ignoring physical keyboard, pointer, file-drop, and native menu input, reporting `{mode, nativeWindowFocused, effectiveWindowFocused}` under `ui.inspect.input` ([`FilmCraft` `docs/control-protocol.md`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md)).
- [ ] **Non-activating background window bring-forward** — Raise occluded windows (`orderFrontRegardless` on macOS) when `CaptureSlot` needs a presented frame without taking keyboard focus ([`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md), [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md)).
- [ ] **macOS App Nap opt-out while `--control` is active** — Hold an `NSProcessInfo` latency-critical activity while `--control` is listening so macOS never starves `eframe` frame drains when occluded ([`FilmCraft` `docs/control-protocol.md`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md)).
- [ ] **Inline `base64` & `max_side` options on `ui.screenshot`** — Return downscaled base64 PNG payloads inline alongside `CaptureSlot` panel/widget cropping when temp-file writes are unwanted ([`PhotoCraft`](https://github.com/storytold/photocraft/blob/main/docs/control-protocol.md)).
- [ ] **Headless `egui_kittest` widget-ID audit test** — Enforce in `cargo test` that every interactive widget across all 8 Magic views registers a stable automation ID and passes AccessKit label checks offscreen ([`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md), [`PhotoCraft` `AGENTS.md`](https://github.com/storytold/photocraft/blob/main/AGENTS.md)).
- [ ] **`craft_ui::docking` & `craft_keymap::ui::ShortcutsDialog` control commands** — Expose `ui.dock`, `ui.dock.reset`, `ui.docking`, and `shortcuts.*` once user-customizable panel layouts and shortcut profiles are enabled ([`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md), [`craft-keymap`](https://github.com/storytold/craft-libs/blob/main/crates/keymap/README.md)).
- [ ] **Extended `perf.stats` `wgpu` & audio telemetry** — Report `wgpu` adapter/backend info and `craft-audio` preview peak dBFS / underrun counters ([`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md)).
- [ ] **Stage & cue-strip coordinate locators (`ui.viewer.locate` / `ui.timeline.locate`)** — Map Storyboard/Lyrics stage coordinates and song-cue seconds directly to logical window points ([`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md), [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md)).
- [ ] **Read-only MCP resources & async job polling (`jobs.list` / `jobs.cancel`)** — Expose `magic://document`, `magic://storyboard`, and `magic://commands` in `crates/mcp` plus non-blocking job polling for long `<craft>-cli` renders ([`PhotoCraft` `book/src/automation/mcp.md`](https://github.com/storytold/photocraft/blob/main/book/src/automation/mcp.md)).
