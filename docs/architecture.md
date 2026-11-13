# Architecture & Workspace Crates

Magic Studio uses a clean layered Rust Cargo workspace (`#!(forbid(unsafe_code))` across every crate and `[workspace.lints]` disallowing `.unwrap()`/`.expect()` in shipped code), verified with `cargo xtask ci` (`cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`).

## Workspace Layers

1. **Shared Upstream Foundations ([`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md), [`storytold/egui`](https://github.com/storytold/egui), [`storytold/winit`](https://github.com/storytold/winit))**:
   - **AI Contract**: `craft-ai-core` ([`docs/ai-shared-layer.md`](https://github.com/storytold/craft-libs/blob/main/docs/ai-shared-layer.md)) for connections, models, tools, jobs, and consent.
   - **Document & Package Formats**: `craft-doc`, `craft-zip`, `craft-drawingml`, `craft-atomic`, `craft-docpath`, `craft-settings`.
   - **Time, Captions, 3D & Media**: `craft-timeline` (`["serde", "xml", "otio"]`), `craft-captions`, `craft-mesh`, `craft-audio`, `craft-media`.
   - **Typography & Workstation UI**: `craft-font-catalog`, `craft-font-store`, `craft-font-manager`, `craft-text`, `craft-ui` (`craft_ui::docking`, `SvgIconSet`, `CaptureSlot`, `WidgetPalette`), `craft-menu`, `craft-window`, `craft-dialogs`, `craft-keymap`, `craft-platform`, `craft-quit`.
2. **Headless Utilities ([`crates/util`](../crates/util/README.md))**:
   - [`crates/util/src/time.rs`](../crates/util/src/time.rs): `craft-timeline` `Tick`, `FrameRate`, and SMPTE `Timecode` helpers.
   - [`crates/util/src/media.rs`](../crates/util/src/media.rs): `craft-audio`, `craft-media`, and `craft-doc` probing plus PCM waveform peak extraction.
   - [`crates/util/src/text.rs`](../crates/util/src/text.rs) & [`src/anim.rs`](../crates/util/src/anim.rs): `craft-font-catalog` lookup, UAX #14 / #9 text helpers, and cubic-Bezier keyframe curves.
3. **Headless Engine ([`crates/engine`](../crates/engine/README.md))**:
   - `Session`, declarative command registry, [`DeckCraft`](https://github.com/storytold/deckcraft) `.deckcraft` / `.pptx` + PDF storyboard coordination, user-editable `extensions/scribbles/` (`craft-ai-core`) guarded by the saved-Storyboard-slide gate, open multi-tool `02 Animations` (`V2`) + `craft-mesh` 3D pipeline, `03 Lyrics` (`V3` + `C1`), `04 Motion` (`V4` `craft-drawingml`), and `05 Studio` (`film/<song>.fcproj` + `studio.mapTracks` + `craft-timeline` NLE interchange).
   - Remains independent of `egui` and windowing crates so `magic-cli` and headless `magic-cli mcp` run without a display server.
4. **MCP & `egui` Workstation UI ([`crates/mcp`](../crates/mcp/README.md), [`crates/ui-egui`](../crates/ui-egui/README.md))**:
   - [`crates/mcp/`](../crates/mcp/README.md): JSON-RPC 2.0 stdio MCP server exposing commands, UI bridges, and read-only `magic://` resources.
   - [`crates/ui-egui/src/docking.rs`](../crates/ui-egui/src/docking.rs): panel layout via `craft_ui::docking` (`Workspace<PanelId>`, `DockArea`, `FloatSpace`), including the 2-tab `Live Breakdown` & `Finishing` dock in `05 Studio` and live `FilmCraft`-aligned editing views across `03 Lyrics`, `04 Motion`, and `05 Studio`.
5. **Binaries & CI Runner ([`apps/magic`](../apps/magic/README.md), [`apps/magic-cli`](../apps/magic-cli/README.md), [`apps/magic-tui`](../apps/magic-tui/README.md), [`xtask`](../xtask/README.md))**:
   - Native `eframe` + `wgpu` desktop app with `--control 9875` (`apps/magic`), headless/`--connect` CLI & MCP entrypoint (`apps/magic-cli`), Backstage terminal viewer (`apps/magic-tui`), and `fmt`/`clippy`/`test` runner (`xtask`).
