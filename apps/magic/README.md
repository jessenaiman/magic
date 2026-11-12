<<<<<<< HEAD
# Magic Studio (`apps/magic`)

Pure-Rust `eframe` 0.37 + `egui` 0.37 + `storytold/craft-libs` + `storytold/craft-ai-integrations` desktop workstation coordinator for Magic Studio.

```sh
# Launch Magic Studio with authenticated loopback --control on 127.0.0.1:9875
cargo run -p magic -- --control 9875

# Connect CLI or MCP server (FilmCraft uses :9876, EffectCraft uses :9877)
magic-cli --connect 127.0.0.1:9875 ui inspect
magic-cli mcp --connect 127.0.0.1:9875
magic-cli bridge status --ensure-tokens
=======
# `magic` Desktop Application (`apps/magic`, Layer 7)

Native `eframe` + `wgpu` desktop binary for Magic Studio. Owns OS window decorations ([`craft-window`](../../docs/craft-reference/craft-libs/crates/craft-window/README.md)), native desktop floating dock viewports ([`craft_ui::docking`](../../docs/craft-reference/craft-libs/docs/ui-docking.md)), non-blocking file pickers ([`craft-dialogs`](../../docs/craft-reference/craft-libs/crates/craft-dialogs/README.md)), real-time `cpal` + [`craft-audio`](../../docs/craft-reference/craft-libs/crates/craft-audio/README.md) song playback, file-handoff watching across the five host ArtCraft apps, and the `127.0.0.1:9875` `--control` TCP server.

## Where & Why to Explore Code

| File | Upstream `storytold` Docs | Why You Explore This File |
| :--- | :--- | :--- |
| **[`src/main.rs`](src/main.rs)** | [`craft-window/README.md`](../../docs/craft-reference/craft-libs/crates/craft-window/README.md) · [`craft-libs/docs/ui-docking.md`](../../docs/craft-reference/craft-libs/docs/ui-docking.md) | Explore for `eframe::App` lifecycle, `UiAction::DockCommand` dispatch, `OpenFontManager` routing to `PanelId::Fonts`, `song.waveform` hydration, drag-and-drop file routing (including `.glb`/`.gltf`/`.obj`/`.fbx`/`.stl`/`.ply`/`.3mf` meshes from docked or native floating windows to `character.layer.new3dModel`), and `craft-quit` unsaved-work guards. |
| **[`src/control.rs`](src/control.rs)** | [`docs/control-protocol.md`](../../docs/control-protocol.md) · [`FilmCraft` `docs/control-protocol.md`](../../docs/craft-reference/core-craft-apps/filmcraft/docs/control-protocol.md) · [`EffectCraft` `docs/control-protocol.md`](../../docs/craft-reference/core-craft-apps/effectcraft/docs/control-protocol.md) | Explore when testing or extending the loopback JSON-lines `--control` channel: `engine.execute`, `ui.docking` (`DockArea::hit_rects` + floating window rects + `DockCommand::IDS`), `ui.inspect` (includes `docking` tree), `ui.menu.invoke` (supports `DockCommand::IDS`), `ui.playback` (real L/R `rmsDbfs` & `peakDbfs`), `ui.viewer.locate`/`hit`, `ui.timeline.locate`/`hit`, and `ui.screenshot`. |
| **[`src/audio_player.rs`](src/audio_player.rs)** | [`craft-audio/README.md`](../../docs/craft-reference/craft-libs/crates/craft-audio/README.md) | Explore when inspecting `craft_audio::io::decode_file` song loading, lock-free `cpal` playhead streaming, and real-time stereo RMS/peak dBFS metering. |
| **[`src/file_sync.rs`](src/file_sync.rs)** | [`craft-atomic/README.md`](../../docs/craft-reference/craft-libs/crates/craft-atomic/README.md) | Explore to see how external saves from `DesignCraft`, `PhotoCraft`, `VectorCraft`, `EffectCraft`, and `FilmCraft` automatically refresh Magic's stage state. |
| **[`src/diagnostics.rs`](src/diagnostics.rs)** | — | Explore for `perf.stats` frame-time ring buffers, memory telemetry, and diagnostic snapshots. |

## Quick Launch & Live `--control` Verification

```sh
# Launch Magic Studio with loopback --control on 127.0.0.1:9875:
cargo run -p magic -- --control 9875

# Query live craft_ui::docking panel/tab/divider hit rectangles:
magic-cli app --port 9875 ui.docking

# Float or focus a dock panel via canonical DockCommand::IDS:
printf '{"id":1,"method":"ui.menu.invoke","params":{"id":"window.panel.scopes"}}\n' | nc 127.0.0.1 9875
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
```
