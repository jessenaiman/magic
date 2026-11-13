# Workspace Architecture Overview

Magic Studio is organized as a layered Rust Cargo workspace (`#!(forbid(unsafe_code))` across every crate and `[workspace.lints]` forbidding `.unwrap()`/`.expect()` in production paths), verified via `cargo xtask ci`.

## Workspace Crates

1. **Upstream Foundations ([`storytold/craft-libs`](https://github.com/storytold/craft-libs), [`storytold/egui`](https://github.com/storytold/egui), [`storytold/winit`](https://github.com/storytold/winit))**:
   - **AI Contract**: `craft-ai-core` ([`docs/ai-shared-layer.md`](https://github.com/storytold/craft-libs/blob/main/docs/ai-shared-layer.md)).
   - **Document & Package Formats**: `craft-doc`, `craft-zip`, `craft-drawingml`, `craft-atomic`, `craft-docpath`, `craft-settings`.
   - **Time, Captions, 3D & Media**: `craft-timeline` (`["serde", "xml", "otio"]`), `craft-captions`, `craft-mesh`, `craft-audio`, `craft-media`.
   - **Typography & Workstation UI**: `craft-font-catalog`, `craft-font-store`, `craft-font-manager`, `craft-text`, `craft-ui` (`craft_ui::docking`, `SvgIconSet`, `CaptureSlot`, `WidgetPalette`), `craft-menu`, `craft-window`, `craft-dialogs`, `craft-keymap`, `craft-platform`, `craft-quit`.
2. **Headless Utilities ([`crates/util`](../../../crates/util/README.md))**:
   - [`crates/util/src/time.rs`](../../../crates/util/src/time.rs): `craft-timeline` `Tick`, `FrameRate`, and SMPTE `Timecode` wrappers.
   - [`crates/util/src/media.rs`](../../../crates/util/src/media.rs): `craft-audio` / `craft-media` / `craft-doc` inspection and PCM waveform peak extraction.
3. **Headless Engine ([`crates/engine`](../../../crates/engine/README.md))**:
   - `Session`, declarative command registry, `DeckCraft` `.deckcraft` / `.pptx` + PDF storyboards, user-editable `extensions/scribbles/` (`craft-ai-core`) with the saved-Storyboard-slide gate, open multi-tool `02 Animations` (`V2`) + `craft-mesh` 3D pipeline, kinetic lyrics (`V3` + `C1`), `craft-drawingml` motion (`V4`), and `05 Studio` (`film/<slug>.fcproj`, `studio.mapTracks`, `craft-timeline` NLE interchange).
   - Free of `egui` and windowing dependencies.
4. **MCP & `egui` Workstation UI ([`crates/mcp`](../../../crates/mcp/README.md), [`crates/ui-egui`](../../../crates/ui-egui/README.md))**:
   - [`crates/mcp/`](../../../crates/mcp/README.md): JSON-RPC 2.0 stdio MCP server with read-only `magic://` resources.
   - [`crates/ui-egui/src/docking.rs`](../../../crates/ui-egui/src/docking.rs): owns panel layouts via `craft_ui::docking`, including the 2-tab `Live Breakdown` & `Finishing` dock in `05 Studio`.
5. **Binaries & CI Runner ([`apps/magic`](../../../apps/magic/README.md), [`apps/magic-cli`](../../../apps/magic-cli/README.md), [`apps/magic-tui`](../../../apps/magic-tui/README.md), [`xtask`](../../../xtask/README.md))**:
   - Native `eframe` + `wgpu` desktop coordinator + `--control 9875` (`apps/magic`), CLI & MCP server (`apps/magic-cli`), Backstage terminal viewer (`apps/magic-tui`), and `fmt`/`clippy`/`test` runner (`xtask`).
