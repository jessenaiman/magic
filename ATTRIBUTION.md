# Asset & Third-Party Attribution

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

Every non-code asset bundled in this repository is listed below with its author, source, and license (`cargo xtask assets` fails CI if an asset under `assets/`, `docs/brand/`, or `figcraft/` has no row):

| Path | Author | Source | License | Use |
| :--- | :--- | :--- | :--- | :--- |
| `assets/app-icon/icon.png` | Jesse Naiman | Original Magic Studio artwork | `LicenseRef-Magic-Studio-NonResale` | Desktop window & launcher icon |
| `docs/brand/LICENSE-brand.txt` | ArtCraft Team / Jesse Naiman | Brand usage notice | `LicenseRef-Magic-Studio-NonResale` | Brand mark terms |
| `figcraft/magic-studio-master.figcraft` | Jesse Naiman | Original FigCraft UI design package | `LicenseRef-Magic-Studio-NonResale` | Canonical UI wireframes & layout sheets |
| `figcraft/diagrams/09-mayhem-diagram.svg` | Jesse Naiman | Original vector architecture diagram | `LicenseRef-Magic-Studio-NonResale` | Mayhem telemetry layout diagram |
| `figcraft/diagrams/10-backstage-tui-diagram.svg` | Jesse Naiman | Original vector architecture diagram | `LicenseRef-Magic-Studio-NonResale` | Backstage (`apps/magic-tui`) 4-pane diagram |

| `assets/icons/archive-zip.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | craft-zip compressed project folder archive icon |
| `assets/icons/bezier-spline.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Cubic Bezier spline & tangent handles icon |
| `assets/icons/captions-cc.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Broadcast closed-captions (SRT/VTT/TTML/CEA-608/708) icon |
| `assets/icons/color-wheel.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Three-way linear color wheel grading icon |
| `assets/icons/keyframe.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | EffectCraft diamond keyframe icon |
| `assets/icons/mark-in.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Sequence / Source Mark-In bracket icon |
| `assets/icons/mark-out.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Sequence / Source Mark-Out bracket icon |
| `assets/icons/mesh-3d.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | 3D triangle mesh topology & GLB/OBJ/STL inspector icon |
| `assets/icons/mute-speaker.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Audio/video track mute speaker icon |
| `assets/icons/next-keyframe.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Step to next keyframe icon |
| `assets/icons/page-single.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Storyboard single-page layout icon |
| `assets/icons/prev-keyframe.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Step to previous keyframe icon |
| `assets/icons/puppet-pin.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | 5-pin puppet deformer pin & mesh lattice icon |
| `assets/icons/razor.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | NLE razor split blade icon |
| `assets/icons/ripple-delete.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Timeline ripple-extract / close-gap icon |
| `assets/icons/scopes-wave.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | 32f linear-light waveform & vectorscope reticle icon |
| `assets/icons/skip-end.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Transport skip-to-end / Out-point icon |
| `assets/icons/skip-start.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Transport skip-to-start / In-point icon |
| `assets/icons/solo-headphones.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Track solo studio monitor headphones icon |
| `assets/icons/spread-two-page.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Storyboard two-page spread layout icon |
| `assets/icons/step-back.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Single-frame step backward icon |
| `assets/icons/step-forward.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Single-frame step forward icon |
| `assets/icons/stop.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Transport stop / commit trim icon |
| `assets/icons/waveform-bars.svg` | Jesse Naiman / Magic Studio | Original clean-room 24x24 vector icon | `MIT OR Apache-2.0` | Decoded PCM audio peaks & loudness meter icon |

## Upstream Open-Source Rust Foundations

- **[storytold/craft-libs](https://github.com/storytold/craft-libs)** (`MIT OR Apache-2.0`): `craft-atomic`, `craft-audio`, `craft-captions`, `craft-dialogs`, `craft-doc`, `craft-docpath`, `craft-drawingml`, `craft-font-catalog`, `craft-font-manager`, `craft-font-store`, `craft-fonts-build`, `craft-i18n`, `craft-keymap`, `craft-media`, `craft-menu`, `craft-mesh`, `craft-quit`, `craft-settings`, `craft-text`, `craft-timeline`, `craft-ui`, `craft-update`, `craft-window`, `craft-xml`, `craft-zip`.
- **[storytold/craft-fonts](https://github.com/storytold/craft-fonts)** & **[storytold/craft-i18n](https://github.com/storytold/craft-i18n)**: Optional build-time `CRAFT_FONTS_DIR` and `CRAFT_I18N_DIR` inputs via `crates/util/build.rs`.
- **[storytold/egui](https://github.com/storytold/egui)** & **[storytold/winit](https://github.com/storytold/winit)** (`MIT OR Apache-2.0`): Pure-Rust GPU UI (`egui` 0.37, `eframe` 0.37, `egui_extras` 0.37) and native windowing.
- **[storytold/craft-ai-integrations](https://github.com/storytold/craft-ai-integrations)** (`MIT OR Apache-2.0`): `craft-ai-client`, `craft-ai-core`, and `craft-ai-native`.
- **[storytold/craft-common-actions](https://github.com/storytold/craft-common-actions)** & **[storytold/craft-contrib](https://github.com/storytold/craft-contrib)**: Shared CI workflows and ArtCraft contribution house rules.
