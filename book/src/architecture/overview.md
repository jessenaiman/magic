# Architecture & Crate Layers

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

Magic Studio enforces a strict downward-only crate DAG verified by `cargo xtask ci` ([`xtask/src/policy.rs`](../../../xtask/src/policy.rs)), `#![forbid(unsafe_code)]` across every crate, zero `.unwrap()`/`.expect()` in production paths, and a strict `< 900`-line ceiling on every `.rs` source file.

## Layer Hierarchy & Where to Explore

1. **L3 — Shared Upstream Foundations ([`storytold/craft-libs`](https://github.com/storytold/craft-libs), [`storytold/font-preview-service`](https://github.com/storytold/font-preview-service), [`storytold/craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations))**:
   - **Document & Archive Formats**: `craft-doc`, `craft-zip`, `craft-drawingml`, `craft-xml`, `craft-atomic`, `craft-docpath`, `craft-settings`.
   - **Time, Captions, 3D & Media**: `craft-timeline` (`["serde", "xml", "otio"]`), `craft-captions`, `craft-mesh`, `craft-audio`, `craft-media`.
   - **Typography & Workstation UI**: `craft-font-catalog`, `craft-font-store`, `craft-font-manager`, `craft-text`, `craft-ui` (`craft_ui::docking`, `SvgIconSet`, `CaptureSlot`, `WidgetPalette`), `craft-menu`, `craft-window`, `craft-dialogs`, `craft-keymap`, `craft-quit`.
2. **L4 — Headless Primitives ([`crates/util`](../../../crates/util/README.md), [`crates/scribbles`](../../../crates/scribbles/README.md))**:
   - [`crates/util/src/time.rs`](../../../crates/util/src/time.rs): `craft-timeline` `Tick`, `FrameRate`, and SMPTE `Timecode` wrappers.
   - [`crates/util/src/media.rs`](../../../crates/util/src/media.rs): zero-subprocess `craft-audio` / `craft-media` / `craft-doc` inspection and PCM waveform peak extraction.
   - [`crates/scribbles/`](../../../crates/scribbles/README.md): deterministic storyboard prompts and sprite/turnaround grid slicing.
3. **L5 — Headless Engine ([`crates/engine`](../../../crates/engine/README.md))**:
   - [`crates/engine/src/lib.rs`](../../../crates/engine/src/lib.rs) & [`crates/engine/src/cmd.rs`](../../../crates/engine/src/cmd.rs): `Session`, declarative command catalog, `storyboard.designcraft` authoring, 2D puppet + `craft-mesh` 3D character pipelines (`mesh_io.rs`, `model3d.rs`), kinetic lyrics, `craft-drawingml` motion comps, unified `FilmCraft` `film/<song>.fcproj` (`V4`/`V3`/`V2`/`V1`/`C1`/`A1`) + `craft-timeline` NLE interchange, and one-time archived project `.zip` convert-on-open.
   - Never depends on `egui`, windowing crates, or sibling ArtCraft app crates.
4. **L6 — MCP & `egui` Workstation UI ([`crates/mcp`](../../../crates/mcp/README.md), [`crates/ui-egui`](../../../crates/ui-egui/README.md))**:
   - [`crates/ui-egui/src/docking.rs`](../../../crates/ui-egui/src/docking.rs): owns all panel arrangements via `craft_ui::docking` (`Workspace<PanelId>`, `DockArea`, `FloatSpace`, presets `Editing`/`Storyboard`/`Animation`/`Color`/`Typography`).
   - [`crates/ui-egui/src/icons.rs`](../../../crates/ui-egui/src/icons.rs) & [`assets/icons/`](../../../assets/icons/): 70+ clean-room `24x24` `currentColor` SVGs enabling icon-first transport, razor/trim/keyframe editing, scope/mixer toggles, and compact stage tabs.
5. **L7 — Binaries & CI Gates ([`apps/magic`](../../../apps/magic/README.md), [`apps/magic-cli`](../../../apps/magic-cli/README.md), [`apps/magic-tui`](../../../apps/magic-tui/README.md), [`xtask`](../../../xtask/README.md))**:
   - Native `eframe` + `wgpu` desktop window + native floating dock viewports + `--control 9875` (`apps/magic`), headless/`--connect` CLI & MCP launcher (`apps/magic-cli`), Backstage TUI (`apps/magic-tui`), and `layers`/`assets` policy verification (`xtask`).
