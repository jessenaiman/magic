# `magic` Desktop Application (`apps/magic`)

Native `eframe` + `wgpu` desktop binary for Magic Studio. Owns OS window decorations (`craft-window`), native desktop floating dock viewports ([`craft_ui::docking`](https://github.com/storytold/craft-libs/blob/main/docs/ui-docking.md)), non-blocking file pickers (`craft-dialogs`), real-time `cpal` + `craft-audio` song playback, file-handoff watching across [`DeckCraft`](https://github.com/storytold/deckcraft), [`AniCraft`](https://github.com/storytold/anicraft), [`PhotoCraft`](https://github.com/storytold/photocraft), [`VectorCraft`](https://github.com/storytold/vectorcraft), [`EffectCraft`](https://github.com/storytold/effectcraft), and [`FilmCraft`](https://github.com/storytold/filmcraft), and the `127.0.0.1:9875` `--control` TCP server.

## Where & Why to Explore Code

| File | Upstream `storytold` Docs | Why You Explore This File |
| :--- | :--- | :--- |
| **[`src/main.rs`](src/main.rs)** | [`craft-libs/docs/ui-docking.md`](https://github.com/storytold/craft-libs/blob/main/docs/ui-docking.md) | `eframe::App` lifecycle, `UiAction::DockCommand` dispatch, `OpenFontManager` routing to `PanelId::Fonts`, `song.waveform` hydration, drag-and-drop media/mesh routing, and `craft-quit` unsaved-work guards. |
| **[`src/control.rs`](src/control.rs)** | [`docs/control-protocol.md`](../../docs/control-protocol.md) · [`FilmCraft` `docs/control-protocol.md`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md) · [`DeckCraft` `docs/control-protocol.md`](https://github.com/storytold/deckcraft/blob/main/docs/control-protocol.md) | Loopback JSON-lines `--control` channel: `engine.execute`, `ui.docking` (`DockArea::hit_rects` + floating window rects + `DockCommand::IDS`), `ui.inspect`, `ui.menu.invoke`, `ui.playback` (L/R `rmsDbfs` & `peakDbfs`), `ui.viewer.locate`/`hit`, `ui.timeline.locate`/`hit`, and `ui.screenshot`. |
| **[`src/audio_player.rs`](src/audio_player.rs)** | [`storytold/craft-libs/README.md`](https://github.com/storytold/craft-libs/blob/main/README.md) | `craft_audio::io::decode_file` song loading, lock-free `cpal` playhead streaming, and real-time stereo RMS/peak dBFS metering. |
| **[`src/file_sync.rs`](src/file_sync.rs)** | [`storytold/craft-libs/README.md`](https://github.com/storytold/craft-libs/blob/main/README.md) | External file-save watcher refreshing Magic Studio's stage views when `DeckCraft`, `AniCraft`, `PhotoCraft`, `VectorCraft`, `EffectCraft`, or `FilmCraft` write updated project files. |
| **[`src/diagnostics.rs`](src/diagnostics.rs)** | — | `perf.stats` frame-time ring buffers, memory telemetry, and diagnostic snapshots. |

## Quick Launch & Live `--control` Verification

```sh
# Launch Magic Studio with loopback --control on 127.0.0.1:9875:
cargo run -p magic -- --control 9875

# Query live craft_ui::docking panel/tab/divider hit rectangles:
magic-cli app --port 9875 ui.docking

# Float or focus a dock panel via canonical DockCommand::IDS:
printf '{"id":1,"method":"ui.menu.invoke","params":{"id":"window.panel.scopes"}}
' | nc 127.0.0.1 9875
```
