# `magic-ui-egui` (`crates/ui-egui`)

Workstation user interface for Magic Studio built on [`emilk/egui`](https://github.com/emilk/egui) (`storytold/egui`), [`craft_ui::docking`](https://github.com/storytold/craft-libs/blob/main/docs/ui-docking.md), [`craft-font-manager`](https://github.com/storytold/craft-libs/blob/main/README.md), [`craft-font-store`](https://github.com/storytold/craft-libs/blob/main/README.md), and [`storytold/font-preview-service`](https://github.com/storytold/font-preview-service).

> **Shared Docking Architecture:** Every workspace panel, tab group, collapsible dock column, and floating window is driven directly by **`craft_ui::docking`** (`Workspace<PanelId>`, `Layout<PanelId>`, `DockArea`, `DockContent<PanelId>`, `DockStyle::from_ui`, `DockCommand<PanelId>`, `FloatSpace`, and native desktop floating windows).

## Shell, Docking & Icon-First Controls

| File | Upstream `storytold` Docs | Why You Explore This File |
| :--- | :--- | :--- |
| **[`src/docking.rs`](src/docking.rs)** | [`craft-libs/docs/ui-docking.md`](https://github.com/storytold/craft-libs/blob/main/docs/ui-docking.md) | Defines `PanelId`, workspace presets (`Editing`, `Storyboard`, `Animation`, `Color`, `Typography`), `MagicDockContent`, `show_dock_area`, `execute_dock_command`, and `inspect_docking` (`DockArea::hit_rects` for `ui.docking`). |
| **[`src/icons.rs`](src/icons.rs)** & **[`assets/icons/`](../../assets/icons/)** | [`ATTRIBUTION.md`](../../ATTRIBUTION.md) · [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) | Registers 70+ clean-room `24x24` `currentColor` vector SVGs in `SvgIconSet` (`bytes://magic/icons/*.svg`), including 24 media, editorial, and rigging icons. |
| **[`src/widgets.rs`](src/widgets.rs)** & **[`src/controls.rs`](src/controls.rs)** | [`book/src/architecture/base-controls.md`](../../book/src/architecture/base-controls.md) | Icon-only transport/tool buttons, compact stage tabs with tooltips, Craft app emblems, and per-frame `--control` widget rect registration (`AutomationRegistry`). |
| **[`src/shell.rs`](src/shell.rs)** & **[`src/menus.rs`](src/menus.rs)** | [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) | Top chrome bar, 9-menu system bar (`Window > Workspaces`, `Panels`, `DockCommand` actions), `Home` `FloatSpace`, and open-project `show_dock_area`. |
| **[`src/timeline.rs`](src/timeline.rs)** | [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/README.md) | Shared docked `Timeline` panel: icon-first transport, razor/mark-in/mark-out/ripple-delete tools, `craft-audio` waveform peaks, and `ui.timeline.locate` / `ui.timeline.hit` geometry. |
| **[`src/fonts_ui.rs`](src/fonts_ui.rs)** | [`storytold/font-preview-service`](https://github.com/storytold/font-preview-service) | `craft_font_manager::FontsPanel` (`PanelId::Fonts` + modal on `Home`), verified installs, and live specimen rendering. |
| **[`src/editorial.rs`](src/editorial.rs)**, **[`src/filmbar.rs`](src/filmbar.rs)**, **[`src/theme.rs`](src/theme.rs)** | [`craft-libs/docs/theme-infrastructure.md`](https://github.com/storytold/craft-libs/blob/main/docs/theme-infrastructure.md) | Shared canvas frame headers, `FilmCraft` track badges, and `MagicPalette` (`WidgetPalette::apply_to`). |
| **[`src/modals.rs`](src/modals.rs)** & **[`src/options_modal.rs`](src/options_modal.rs)** | [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) | New/existing project dialog, `craft_keymap::ui::ShortcutsDialog`, dockable `History` and `ThemeFidget` bodies, and `Options`. |

## Stage Views (`src/views/`)

| Stage | Key Views | Responsibility |
| :--- | :--- | :--- |
| **Home** | [`views/home.rs`](src/views/home.rs) | Welcome screen, recent `.magic` projects, and New / Existing Project setup. |
| **00 Project** | [`views/project.rs`](src/views/project.rs), [`views/project_interchange.rs`](src/views/project_interchange.rs) | Song metadata, `craft-audio` waveform overview, asset bin, and `craft-timeline` NLE interchange (`EDL`/`ALE`/`FCP7`/`FCPXML`/`OTIO`). |
| **01 Storyboard** | [`views/storyboard.rs`](src/views/storyboard.rs) | [`DeckCraft`](https://github.com/storytold/deckcraft) `.deckcraft` / `.pptx` slide canvas + PDF export, saved-Storyboard-slide status, and `extensions/scribbles/` controls. |
| **02 Animations** | [`views/animation.rs`](src/views/animation.rs), [`views/animation_inspector.rs`](src/views/animation_inspector.rs) | Open multi-tool `V2` asset roster (Blender, Three.js, After Effects, `AniCraft`, `EffectCraft`), puppet pins, and `craft-mesh` 3D topology/clip inspector. |
| **03 Lyrics** | [`views/lyrics.rs`](src/views/lyrics.rs), [`views/lyrics_dock.rs`](src/views/lyrics_dock.rs), [`views/lyrics_inspector.rs`](src/views/lyrics_inspector.rs) | Live `FilmCraft`-aligned kinetic lyric canvas (`V3`), `craft-captions` (`C1`) timing strip, `craft-text` line-wrap controls, and `FontsPanel`. |
| **04 Motion** | [`views/motion.rs`](src/views/motion.rs), [`views/motion_dock.rs`](src/views/motion_dock.rs) | Live `FilmCraft`-aligned `craft-drawingml` preset geometry gallery, text-warp (`a:prstTxWarp`) selector, and `EffectCraft` `V4` motion cues. |
| **05 Studio** | [`views/studio.rs`](src/views/studio.rs), [`views/studio_finishing.rs`](src/views/studio_finishing.rs), [`views/studio_audio_titles.rs`](src/views/studio_audio_titles.rs) | Unified `FilmCraft` (`film/<slug>.fcproj`) Program Monitor with the 2-tab **`Live Breakdown` & `Finishing`** dock (`studio.mapTracks`, `V1`–`V4` + `C1` + `A1` breakdown, linear-light color wheels, RGB scopes, and stereo dBFS mixer). |
