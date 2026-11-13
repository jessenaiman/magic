# Native `egui`, `craft_ui::docking` & Icon-First Widget System

Magic Studio's workstation UI ([`crates/ui-egui`](../../../crates/ui-egui/README.md)) is built in pure Rust on `eframe` + [`emilk/egui`](https://github.com/emilk/egui) (`storytold/egui`) and [`storytold/craft-libs`](https://github.com/storytold/craft-libs), using **`craft_ui::docking`** for all panel arrangement — including the 2-tab **`Live Breakdown` & `Finishing`** dock in `05 Studio` — and a 70+ SVG icon-first control set.

## Shared `craft-libs` Foundations

| Crate | Role in `magic-ui-egui` & `apps/magic` |
|---|---|
| [`craft-ui`](https://github.com/storytold/craft-libs/blob/main/README.md) | `docking::{Workspace, Layout, Node, DockArea, DockContent, DockStyle, DockCommand, FloatSpace}`, `theme::WidgetPalette::apply_to`, `icons::SvgIconSet` (`bytes://magic/icons/`), `buttons::IconButton`, and `capture::CaptureSlot` (`wgpu` PNG screenshots). |
| [`craft-font-manager`](https://github.com/storytold/craft-libs/blob/main/README.md) | `FontsPanel` embedded inside dockable `PanelId::Fonts` (`crates/ui-egui/src/fonts_ui.rs`), backed by `craft-font-catalog`, `craft-font-store`, and [`storytold/font-preview-service`](https://github.com/storytold/font-preview-service). |
| [`craft-menu`](https://github.com/storytold/craft-libs/blob/main/README.md) | Platform-aware 9-menu system bar (`File`, `Edit`, `View`, `Storyboard`, `Lyrics`, `Character`, `Film`, `Window`, `Help`) including `Window > Workspaces`, panel toggles, and `DockCommand::IDS`. |
| [`craft-window`](https://github.com/storytold/craft-libs/blob/main/README.md) | Compositor decoration probing (`Choice::probe(Prefix::new("MAGIC"))`), `decorate(viewport, choice.frame())`, and native floating dock viewports. |
| [`craft-dialogs`](https://github.com/storytold/craft-libs/blob/main/README.md) | Non-blocking native file & folder pickers (`Dialogs::take_answers`, `Dialogs::show_pending(Some(frame))`, and `set_override` for `--control` tests). |
| [`craft-keymap`](https://github.com/storytold/craft-libs/blob/main/README.md) | Command shortcut catalog, `ShortcutsDialog`, and per-frame `egui::Dispatcher` chord routing. |
| [`craft-quit`](https://github.com/storytold/craft-libs/blob/main/README.md) | Clean window-close and macOS Quit menu handling (`craft_quit::on_close_request(ctx)`). |

## Base Control Invariants

1. **Icon-First Vector SVG Controls (`icons::SvgIconSet` + `widgets::icon_only_btn` / `action_button`)**: Transport controls, razor/ripple-delete/mark-in/mark-out timeline tools, keyframe steppers, scope/color-wheel toggles, mute/solo mixer strips, and compact top-bar chrome use clean-room `24x24` `currentColor` SVGs under `assets/icons/*.svg` paired with hover tooltips.
2. **Panel Docking (`docking::show_dock_area`)**: All panels (`Canvas`, `ProjectBrowser`, `Assets`, `Inspector`, `Timeline`, `AudioMixer`, `Effects`, `Scopes`, `Captions`, `Fonts`, `AiStudio`, `History`, `ThemeFidget`) and the 2-tab `05 Studio` dock (`Live Breakdown` & `Finishing`) run on `craft_ui::docking::Workspace<PanelId>`.
3. **Per-Frame Automation Registry (`widgets::AutomationRegistry` + `DockArea::hit_rects`)**: Every widget and dock target exposes deterministic logical `egui::Rect` coordinates to `ui.elements`, `ui.docking`, `ui.click`, `ui.drag`, and cropped `ui.screenshot`.
