# Magic Studio Roadmap

<<<<<<< HEAD
Magic Studio coordinates the [ArtCraft](https://github.com/storytold) creative suite — [`DesignCraft`](https://github.com/storytold/designcraft), [`PhotoCraft`](https://github.com/storytold/photocraft), [`VectorCraft`](https://github.com/storytold/vectorcraft), [`EffectCraft`](https://github.com/storytold/effectcraft), and [`FilmCraft`](https://github.com/storytold/filmcraft) — to turn a recorded song into a storyboarded, puppet-animated, kinetic-lyric video.

Detailed technical checklists for unimplemented requests and `// TODO:` host checks live in [`docs/control-protocol.md`](docs/control-protocol.md) and [`ROADMAP.md`](ROADMAP.md).

## Completed Foundation

- [x] Pure-Rust desktop shell (`eframe` + `egui` 0.37 + [`storytold/craft-libs`](https://github.com/storytold/craft-libs)) in `crates/ui-egui` and `apps/magic` with `craft-menu` system menus, inner stage navigation, `craft_ui::capture::CaptureSlot` cross-platform screenshots, `craft-keymap`, `craft-dialogs`, and `craft-quit` — zero Tauri/webview dependencies.
- [x] Single-Responsibility workspace architecture (`magic-util`, `magic-scribbles`, `magic-engine`, `magic-mcp`, `magic-ui-egui`, `magic-cli`, `magic`, `magic-tui`) enforced by `cargo xtask layers`.
- [x] Song-first `magic-cli new --audio` intake writing `timing-reference.json` and initializing a synced [`FilmCraft`](https://github.com/storytold/filmcraft) `.fcproj` timeline (`A1` Song, `V1` Storyboard Scenes, `V2` Character Animation, `V3` Kinetic Lyrics, `V4` Action Motion).
- [x] [`DesignCraft`](https://github.com/storytold/designcraft) `storyboard.designcraft` master template with locked, read-only fine-print `magic-cli` instructions (`*_cli`), traffic-light readiness gating (`storyboard.check`), and title-page-driven `storyboard.bulk_generate`.
- [x] [`EffectCraft`](https://github.com/storytold/effectcraft) 11-pin facial & viseme Puppet Controls (`character.puppet.rig_face`, `character.puppet.expression`, `character.puppet.lipsync`, `character.puppet.follow`, `character.puppet.record`).
- [x] [`PhotoCraft`](https://github.com/storytold/photocraft) subject isolation (`photo.remove_background`) and [`VectorCraft`](https://github.com/storytold/vectorcraft) background-free tracing (`vector.trace`), plus strict headless-only safety isolation for early-alpha [`AniCraft`](https://github.com/storytold/AniCraft).
- [x] `magic-cli mcp` / `magic-mcp` stdio server (`--connect` / `--bridge` + `--headless`) and `magic --control 9875` live desktop watching with `--automation-read-root` and `--automation-write-root` confinement.
- [x] Magic Studio Source-Available Non-Resale License (`LICENSE` + `docs/brand/LICENSE-brand.txt`) allowing unrestricted commercial video production while prohibiting commercial resale of Magic Studio.

## Next Milestones

### Milestone A — Live `--control` & Stage Viewport Parity
- [ ] Add `--control-input-only`, `ui.viewer.locate` / `ui.viewer.hit`, `ui.timeline.locate` / `ui.timeline.hit`, `render.frame {base64: true, max_side?}`, and live dBFS audio meters in `ui.playback`.
- [ ] Opt out of macOS App Nap while `--control` is active and expose MCP read-only resources (`magic://document`, `magic://storyboard`, `magic://commands`).

### Milestone B — 3D Character Breakout & Host CLI Verification
- [ ] Complete `TextureCraft` CLI inspection and glTF skeletal channel mapping in `crates/engine/src/cmd/character/model3d.rs`.
- [ ] Verify `vectorcraft-cli` flat PNG export verb in `storyboard/crud.rs` and add opt-in host integration tests across all installed `*craft-cli` binaries.

### Milestone C — Phoneme Lip-Sync, Duet Stems & Community Release
- [ ] Upgrade `character.puppet.lipsync` with sub-word phoneme alignment and multi-character vocal stem routing.
- [ ] Configure signed multi-platform installers via [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions) and publish user guides to [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).
=======
Magic Studio coordinates the five host [ArtCraft](https://github.com/storytold) creative applications — [`DesignCraft`](https://github.com/storytold/designcraft), [`PhotoCraft`](https://github.com/storytold/photocraft), [`VectorCraft`](https://github.com/storytold/vectorcraft), [`EffectCraft`](https://github.com/storytold/effectcraft), and [`FilmCraft`](https://github.com/storytold/filmcraft) — on top of [`storytold/craft-libs`](https://github.com/storytold/craft-libs) to turn a recorded song into a finished multi-track video.

Start in [`README.md`](README.md) and [`docs/README.md`](docs/README.md) for module-by-module **Where & Why to Explore Code** links, and see [`docs/gaps.md`](docs/gaps.md) for remaining host-verification items.

## Completed Foundation

- [x] **`craft_ui::docking` Workstation & Icon-First SVG Chrome** ([`crates/ui-egui/src/docking.rs`](crates/ui-egui/src/docking.rs), [`crates/ui-egui/src/icons.rs`](crates/ui-egui/src/icons.rs)): Full panel docking (`Workspace<PanelId>`, `DockArea`, `DockCommand::IDS`, `FloatSpace`, native OS floating viewports, `ui.docking` hit rects) and 70+ clean-room `24x24` `currentColor` SVGs (`assets/icons/*.svg`).
- [x] **Pure `DesignCraft` Storyboard (`storyboard.designcraft`)** ([`crates/engine/src/cmd/storyboard/`](crates/engine/src/cmd/storyboard/mod.rs), [`crates/ui-egui/src/views/storyboard.rs`](crates/ui-egui/src/views/storyboard.rs)): Complete editable `DesignCraft` design document (`Cover`, `Backgrounds`, `Characters`, `Scenes`, `Assets`, `Final`) synchronized with host `DesignCraft` (`--control 7979` / `designcraft-cli`).
- [x] **Full `storytold` Shared Library Stack**:
  - `craft-timeline` (`["serde", "xml", "otio"]`) for exact `Tick`/SMPTE math and `EDL`/`ALE`/`FCP7 XML`/`FCPXML`/`OTIO` NLE interchange ([`crates/util/src/time.rs`](crates/util/src/time.rs), [`crates/engine/src/cmd/film/timeline_interchange.rs`](crates/engine/src/cmd/film/timeline_interchange.rs)).
  - `craft-captions` (`SRT`/`WebVTT`/`TTML`/`SCC`/`MCC`/`EBU-STL`) and unified `05 Studio` ↔ `FilmCraft` (`film/<song>.fcproj` `V4`/`V3`/`V2`/`V1`/`C1`/`A1`) editing ([`crates/engine/src/cmd/film/fcproj_doc.rs`](crates/engine/src/cmd/film/fcproj_doc.rs)).
  - `craft-mesh` (`GLB`/`GLTF`/`OBJ`/`FBX`/`STL`/`PLY`/`3MF`) 3D character inspection, conversion, and `EffectCraft` `advanced3D` `layer.newModel` rigs ([`crates/engine/src/cmd/character/mesh_io.rs`](crates/engine/src/cmd/character/mesh_io.rs), [`crates/engine/src/cmd/character/model3d.rs`](crates/engine/src/cmd/character/model3d.rs)).
  - `craft-audio` + `craft-media` + `craft-doc` real PCM waveform envelopes, live L/R dBFS meters, and Craft document preview extraction ([`crates/util/src/media.rs`](crates/util/src/media.rs), [`crates/engine/src/cmd/song/waveform.rs`](crates/engine/src/cmd/song/waveform.rs)).
  - `craft-drawingml` preset geometry, text warps, and theme colors in `04 Motion` ([`crates/engine/src/cmd/motion/mod.rs`](crates/engine/src/cmd/motion/mod.rs)).
  - `craft-zip` bounded `.zip` project folder archiving (`project.archive` in [`crates/engine/src/cmd/project/archive.rs`](crates/engine/src/cmd/project/archive.rs)).
  - `craft-extensions` (`craft-ext-core` + `craft-ext-store`, `third-party` off in [`crates/engine/src/extensions.rs`](crates/engine/src/extensions.rs)), `craft-fonts-build` + `craft-i18n` (`CRAFT_FONTS_DIR` / `CRAFT_I18N_DIR` in [`crates/util/build.rs`](crates/util/build.rs)), and `craft-font-manager::FontsPanel` ([`crates/ui-egui/src/fonts_ui.rs`](crates/ui-egui/src/fonts_ui.rs)).
- [x] **Live `--control 9875` & MCP Automation**: `magic --control 9875`, `magic-cli mcp`, `magic-tui`, and `cargo xtask ci` (`layers` + `assets` + `< 900` lines per `.rs` file).

## Next Milestones

### Milestone A — `--control-input-only` & MCP Resource URIs
- [ ] Add `--control-input-only`, `render.frame {base64: true, max_side?}`, macOS App Nap opt-out, and read-only `magic://` MCP resources.

### Milestone B — Interactive 3D Viewport & Per-Bone glTF Channel Overrides
- [ ] Add `egui_wgpu::CallbackTrait` 3D orbit preview in `02 Animations` and map `.anim.json` joint curves onto `EffectCraft` `advanced3D` nodes.

### Milestone C — Sub-Word Phoneme Lip-Sync, Duet Stems & Signed Releases
- [ ] Add sub-word viseme extraction, multi-character vocal stem tracks, signed installers via [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions), and [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki) user guides.
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
