# Asset & Third-Party Attribution

Every non-code asset bundled in this repository is listed below with its author, source, and license (`cargo xtask assets` fails CI if an asset under `assets/`, `docs/brand/`, or `figcraft/` has no row):

| Path | Author | Source | License | Use |
| :--- | :--- | :--- | :--- | :--- |
| `assets/app-icon/icon.png` | Jesse Naiman | Original Magic Studio artwork | `LicenseRef-Magic-Studio-NonResale` | Desktop window & launcher icon |
| `docs/brand/LICENSE-brand.txt` | ArtCraft Team / Jesse Naiman | Brand usage notice | `LicenseRef-Magic-Studio-NonResale` | Brand mark terms |
| `figcraft/magic-studio-master.figcraft` | Jesse Naiman | Original FigCraft UI design package | `LicenseRef-Magic-Studio-NonResale` | Canonical UI wireframes & layout sheets |
| `figcraft/diagrams/09-mayhem-diagram.svg` | Jesse Naiman | Original vector architecture diagram | `LicenseRef-Magic-Studio-NonResale` | Mayhem telemetry layout diagram |
| `figcraft/diagrams/10-backstage-tui-diagram.svg` | Jesse Naiman | Original vector architecture diagram | `LicenseRef-Magic-Studio-NonResale` | Backstage (`apps/magic-tui`) 4-pane diagram |

## Upstream Open-Source Rust Foundations

- **[storytold/craft-libs](https://github.com/storytold/craft-libs)** (`MIT OR Apache-2.0`): `craft-atomic`, `craft-audio`, `craft-dialogs`, `craft-doc`, `craft-docpath`, `craft-ecproj-format`, `craft-font-catalog`, `craft-keymap`, `craft-media`, `craft-menu`, `craft-pcraft-format`, `craft-platform`, `craft-quit`, `craft-settings`, `craft-ui`, `craft-update`, `craft-vectorcraft-format`, `craft-window`.
- **[storytold/egui](https://github.com/storytold/egui)** & **[storytold/winit](https://github.com/storytold/winit)** (`MIT OR Apache-2.0`): Pure-Rust GPU UI (`egui` 0.37, `eframe` 0.37, `egui_extras` 0.37) and native windowing.
- **[storytold/craft-ai-integrations](https://github.com/storytold/craft-ai-integrations)** (`MIT OR Apache-2.0`): `craft-ai-client`, `craft-ai-core`, and `craft-ai-native`.
- **[storytold/craft-common-actions](https://github.com/storytold/craft-common-actions)** & **[storytold/craft-contrib](https://github.com/storytold/craft-contrib)**: Shared CI workflows and ArtCraft contribution house rules.
