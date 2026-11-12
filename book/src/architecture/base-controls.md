<<<<<<< HEAD
# Native `egui` & `craft-ui` Widget System

Magic Studio's workstation UI (`crates/ui-egui`, Layer 6) is written 100% in Rust on `eframe` 0.37 + `egui` 0.37 (`storytold/egui`) and [`storytold/craft-libs`](https://github.com/storytold/craft-libs) with zero HTML, CSS, TypeScript, or WebView dependencies.
=======
# Native `egui`, `craft_ui::docking` & Icon-First Widget System

Magic Studio's workstation UI ([`crates/ui-egui`](../../../crates/ui-egui/README.md), Layer 6) is built in Rust on `eframe` + [`emilk/egui`](https://github.com/emilk/egui) (`storytold/egui`) and [`storytold/craft-libs`](https://github.com/storytold/craft-libs), using **`craft_ui::docking`** for all panel arrangement and a 70+ SVG icon-first control set.
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

## Shared `craft-libs` Foundations

| Crate | Role in `magic-ui-egui` & `apps/magic` |
|---|---|
<<<<<<< HEAD
| [`craft-ui`](https://github.com/storytold/craft-libs/blob/main/crates/craft-ui/README.md) | `theme::WidgetPalette::apply_to`, `icons::SvgIconSet` (`bytes://magic/icons/`), `buttons::IconButton`, `tabs::Tab`, `layout::split_rect`, and `capture::CaptureSlot` (`wgpu` PNG screenshots). |
| [`craft-menu`](https://github.com/storytold/craft-libs/blob/main/crates/craft-menu/README.md) | Platform-aware 9-menu system bar (`File`, `Edit`, `View`, `Storyboard`, `Lyrics`, `Character`, `Film`, `Window`, `Help`) rendered via `MenuBarUi` on Linux/Windows/Web and `craft_menu::mac` on macOS. |
| [`craft-window`](https://github.com/storytold/craft-libs/blob/main/crates/craft-window/README.md) | Compositor decoration probing (`Choice::probe(Prefix::new("MAGIC"))`), `decorate(viewport, choice.frame())`, and `title_bar::empty_area` / `sync_movable`. |
| [`craft-dialogs`](https://github.com/storytold/craft-libs/blob/main/crates/craft-dialogs/README.md) | Non-blocking native file & folder pickers (`Dialogs::take_answers`, `Dialogs::show_pending(Some(frame))`, and `set_override` for `--control` tests). |
| [`craft-keymap`](https://github.com/storytold/craft-libs/blob/main/crates/craft-keymap/README.md) | Command shortcut catalog and per-frame `egui::Dispatcher` chord routing. |
| [`craft-quit`](https://github.com/storytold/craft-libs) | Clean window-close and macOS Quit menu handling (`craft_quit::on_close_request(ctx)` after `ViewportCommand::CancelClose` unsaved-work guards). |

## Base Control Invariants

1. **Single-Word + 14px SVG Icon Buttons (`widgets::action_button`)**: Interactive toolbar and rail buttons pair a `24x24` monochrome SVG from `SvgIconSet` with at most one concise word (`Save`, `Publish`, `Rig`, `Cut`, `Export`) and attach full context via `.on_hover_text(...)`.
2. **Uniform Workstation Palette (`theme::MagicPalette`)**: Derived from `Magic Studio Design.figcraft` (`#0d1117` stage canvas, `#141922` panel surface, `#5bd69b` emerald accent, `#eabb62` review amber, `#58a6ff` sky, `#a371f7` EffectCraft violet) and applied to all five `egui` widget states through `WidgetPalette::apply_to`.
3. **Per-Frame Automation Registry (`widgets::AutomationRegistry`)**: Every button, tab, text field, checkbox, and numeric drag registers its stable automation ID, label, enabled state, and logical `egui::Rect` each frame so `ui.elements`, `ui.click`, `ui.drag`, `ui.type`, `ui.focus`, and cropped `ui.screenshot {id}` operate deterministically over `--control`.
=======
| [`craft-ui`](https://github.com/storytold/craft-libs/blob/main/crates/craft-ui/README.md) | `docking::{Workspace, Layout, Node, DockArea, DockContent, DockStyle, DockCommand, FloatSpace}`, `theme::WidgetPalette::apply_to`, `icons::SvgIconSet` (`bytes://magic/icons/`), `buttons::IconButton`, and `capture::CaptureSlot` (`wgpu` PNG screenshots). |
| [`craft-font-manager`](https://github.com/storytold/craft-libs/blob/main/crates/craft-font-manager/README.md) | `FontsPanel` embedded inside dockable `PanelId::Fonts` (`crates/ui-egui/src/fonts_ui.rs`), backed by `craft-font-catalog`, `craft-font-store`, and [`storytold/font-preview-service`](https://github.com/storytold/font-preview-service). |
| [`craft-menu`](https://github.com/storytold/craft-libs/blob/main/crates/craft-menu/README.md) | Platform-aware 9-menu system bar (`File`, `Edit`, `View`, `Storyboard`, `Lyrics`, `Character`, `Film`, `Window`, `Help`) including `Window > Workspaces`, panel toggles, and `DockCommand::IDS`. |
| [`craft-window`](https://github.com/storytold/craft-libs/blob/main/crates/craft-window/README.md) | Compositor decoration probing (`Choice::probe(Prefix::new("MAGIC"))`), `decorate(viewport, choice.frame())`, and native floating dock viewports. |
| [`craft-dialogs`](https://github.com/storytold/craft-libs/blob/main/crates/craft-dialogs/README.md) | Non-blocking native file & folder pickers (`Dialogs::take_answers`, `Dialogs::show_pending(Some(frame))`, and `set_override` for `--control` tests). |
| [`craft-keymap`](https://github.com/storytold/craft-libs/blob/main/crates/craft-keymap/README.md) | Command shortcut catalog, `ShortcutsDialog`, and per-frame `egui::Dispatcher` chord routing. |
| [`craft-quit`](https://github.com/storytold/craft-libs) | Clean window-close and macOS Quit menu handling (`craft_quit::on_close_request(ctx)`). |

## Base Control Invariants

1. **Icon-First Vector SVG Controls (`icons::SvgIconSet` + `widgets::icon_only_btn` / `action_button`)**: Transport controls, razor/ripple-delete/mark-in/mark-out timeline tools, keyframe steppers, scope/color-wheel toggles, mute/solo mixer strips, and compact top-bar chrome use clean-room `24x24` `currentColor` SVGs under `assets/icons/*.svg` paired with rich hover tooltips rather than verbose text labels or ASCII glyphs.
2. **Panel Docking (`docking::show_dock_area`)**: All panels (`Canvas`, `ProjectBrowser`, `Assets`, `Inspector`, `Timeline`, `AudioMixer`, `Effects`, `Scopes`, `Captions`, `Fonts`, `AiStudio`, `History`, `ThemeFidget`) live in `craft_ui::docking::Workspace<PanelId>` with zero custom ad-hoc panel rect math or manual child panes layout shims.
3. **Per-Frame Automation Registry (`widgets::AutomationRegistry` + `DockArea::hit_rects`)**: Every widget and dock target exposes deterministic logical `egui::Rect` coordinates to `ui.elements`, `ui.docking`, `ui.click`, `ui.drag`, and cropped `ui.screenshot`.
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
