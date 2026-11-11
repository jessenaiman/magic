# Native `egui` & `craft-ui` Widget System

Magic Studio's workstation UI (`crates/ui-egui`, Layer 6) is written 100% in Rust on `eframe` 0.37 + `egui` 0.37 (`storytold/egui`) and [`storytold/craft-libs`](https://github.com/storytold/craft-libs) with zero HTML, CSS, TypeScript, or WebView dependencies.

## Shared `craft-libs` Foundations

| Crate | Role in `magic-ui-egui` & `apps/magic` |
|---|---|
| [`craft-ui`](https://github.com/storytold/craft-libs/blob/main/crates/ui/README.md) | `theme::WidgetPalette::apply_to`, `icons::SvgIconSet` (`bytes://magic/icons/`), `buttons::IconButton`, `tabs::Tab`, `layout::split_rect`, and `capture::CaptureSlot` (`wgpu` PNG screenshots). |
| [`craft-menu`](https://github.com/storytold/craft-libs/blob/main/crates/menu/README.md) | Platform-aware 9-menu system bar (`File`, `Edit`, `View`, `Storyboard`, `Lyrics`, `Character`, `Film`, `Window`, `Help`) rendered via `MenuBarUi` on Linux/Windows/Web and `craft_menu::mac` on macOS. |
| [`craft-window`](https://github.com/storytold/craft-libs/blob/main/crates/window/README.md) | Compositor decoration probing (`Choice::probe(Prefix::new("MAGIC"))`), `decorate(viewport, choice.frame())`, and `title_bar::empty_area` / `sync_movable`. |
| [`craft-dialogs`](https://github.com/storytold/craft-libs/blob/main/crates/dialogs/README.md) | Non-blocking native file & folder pickers (`Dialogs::take_answers`, `Dialogs::show_pending(Some(frame))`, and `set_override` for `--control` tests). |
| [`craft-keymap`](https://github.com/storytold/craft-libs/blob/main/crates/keymap/README.md) | Command shortcut catalog and per-frame `egui::Dispatcher` chord routing. |
| [`craft-quit`](https://github.com/storytold/craft-libs) | Clean window-close and macOS Quit menu handling (`craft_quit::on_close_request(ctx)` after `ViewportCommand::CancelClose` unsaved-work guards). |

## Base Control Invariants

1. **Single-Word + 14px SVG Icon Buttons (`widgets::action_button`)**: Interactive toolbar and rail buttons pair a `24x24` monochrome SVG from `SvgIconSet` with at most one concise word (`Save`, `Publish`, `Rig`, `Cut`, `Export`) and attach full context via `.on_hover_text(...)`.
2. **Uniform Workstation Palette (`theme::MagicPalette`)**: Derived from `Magic Studio Design.figcraft` (`#0d1117` stage canvas, `#141922` panel surface, `#5bd69b` emerald accent, `#eabb62` review amber, `#58a6ff` sky, `#a371f7` EffectCraft violet) and applied to all five `egui` widget states through `WidgetPalette::apply_to`.
3. **Per-Frame Automation Registry (`widgets::AutomationRegistry`)**: Every button, tab, text field, checkbox, and numeric drag registers its stable automation ID, label, enabled state, and logical `egui::Rect` each frame so `ui.elements`, `ui.click`, `ui.drag`, `ui.type`, `ui.focus`, and cropped `ui.screenshot {id}` operate deterministically over `--control`.
