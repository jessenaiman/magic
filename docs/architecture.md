# Architecture & Workspace Crates

Magic Studio uses a modular Cargo workspace (`#![forbid(unsafe_code)]` across every crate and `[workspace.lints]` disallowing `.unwrap()`, `.expect()`, and `.todo()` in shipped code), verified with `cargo xtask ci` (`cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`).

## Workspace Layers

1. **Shared Upstream Foundations ([`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md), [`storytold/craft-extensions`](https://github.com/storytold/craft-extensions/blob/main/README.md), [`storytold/craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations/blob/main/README.md), [`storytold/egui`](https://github.com/storytold/egui), [`storytold/winit`](https://github.com/storytold/winit))**:
   - **Standard AI Layer**: `craft-ai-core`, `craft-ai-client`, `craft-ai-native`, `craft-ai-features`, `craft-ai-agents`, and `craft-ai-egui` for shared connections, built-in feature roles, job history, and `ConnectionPanel`.
   - **Shared Extensions**: `craft-ext-core` & `craft-ext-store` for read-only discovery of installed themes, plug-ins, and resources (`app.extensions`).
   - **Document, Time, Captions, 3D & Media**: `craft-doc`, `craft-zip`, `craft-drawingml`, `craft-atomic`, `craft-docpath`, `craft-settings`, `craft-timeline`, `craft-captions`, `craft-mesh`, `craft-audio`, `craft-media`.
   - **Typography & Workstation UI**: `craft-font-catalog`, `craft-font-store`, `craft-font-manager`, `craft-text`, `craft-ui` (`craft_ui::docking`, `SvgIconSet`, `CaptureSlot`, `WidgetPalette`), `craft-menu`, `craft-window`, `craft-dialogs`, `craft-keymap`, `craft-platform`, `craft-quit`.
2. **Headless Utilities ([`crates/util`](../crates/util/))**:
   - [`crates/util/src/time.rs`](../crates/util/src/time.rs), [`media.rs`](../crates/util/src/media.rs), [`text.rs`](../crates/util/src/text.rs), [`anim.rs`](../crates/util/src/anim.rs).
3. **Headless Engine ([`crates/engine`](../crates/engine/))**:
   - `Session`, declarative command registry, standalone `designcraft-cli` storyboard coordination (`storyboard.{status,pages,new,render,open}` over `.designcraft` files), `app.extensions` (`craft-ext-store`), standard `craft_ai` integration, open multi-tool `02 Animations` (`V2`) + `craft-mesh`, `03 Lyrics` (`V3` + `C1`), `04 Motion` (`V4` via `effectcraft-cli`), and `05 Studio` (`film/<song>.fcproj` with `A1` Song Master audio + `video.map_tracks` + `filmcraft-cli`).
   - Independent of `egui` and windowing crates so `magic-cli` and `magic-cli mcp` run headlessly.
4. **View Contract & Isolated View Crates**:
   - [`crates/view-api`](../crates/view-api/): defines the [`StageView`](../crates/view-api/src/lib.rs) and [`HomeView`](../crates/view-api/src/lib.rs) traits, `disabled_todo_button`, shared `UiShellState` / `ViewCtx`, docking layout tokens (`DockWorkspaceState`, `PanelId`), palette, timeline primitives, and shared widgets.
   - [`crates/view-project`](../crates/view-project/): `MagicHomeView` + `ProjectView`.
   - [`crates/view-storyboard`](../crates/view-storyboard/): standalone `01 Storyboard` view backed exclusively by `storyboard.designcraft` (`storyboard.{new,pages,render,open}`).
   - [`crates/view-animation`](../crates/view-animation/): `02 Animations` multi-tool view.
   - [`crates/view-lyrics`](../crates/view-lyrics/): `03 Lyrics` live typography & `FontsPanel` view.
   - [`crates/view-motion`](../crates/view-motion/): `04 Motion` vector/effect authoring view.
   - [`crates/view-studio`](../crates/view-studio/): `05 Studio` `FilmCraft` breakdown & finishing dock.
5. **MCP & Shell Host ([`crates/mcp`](../crates/mcp/), [`crates/ui-egui`](../crates/ui-egui/))**:
   - [`crates/mcp/`](../crates/mcp/): JSON-RPC 2.0 stdio MCP server exposing commands, UI bridges, and read-only `magic://` resources.
   - [`crates/ui-egui/`](../crates/ui-egui/): thin workstation shell (`StageHost` dispatching `DockArea<PanelId>` panels into `StageView`/`HomeView` traits, menus, modals, and `Options` powered by `craft-ai-egui` + `craft-ext-store`).
6. **Binaries & CI Runner ([`apps/magic`](../apps/magic/), [`apps/magic-cli`](../apps/magic-cli/), [`xtask`](../xtask/))**:
   - Native `eframe` + `wgpu` desktop app with `--control 9875` (`apps/magic`), headless/`--connect` CLI & MCP entrypoint (`apps/magic-cli`), and `fmt`/`clippy` runner (`xtask`).
