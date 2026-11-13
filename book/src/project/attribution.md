# Attribution & Upstream Suite

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

Magic Studio builds on community-maintained open-source libraries and interoperates with the ArtCraft creative suite:

- **[`storytold/craft-libs`](https://github.com/storytold/craft-libs)** (`MIT OR Apache-2.0`) — shared document paths (`craft-docpath`), atomic file I/O (`craft-atomic`), `.pcraft`/`.vectorcraft` previews & PDF export (`craft-doc`), font catalog (`craft-font-catalog`), audio & ISOBMFF media inspection (`craft-audio`, `craft-media`), `.ecproj`/`.pcraft`/`.vectorcraft` formats, `craft-ui` (`WidgetPalette`, `IconButton`, `SvgIconSet`, `Fidget`, `CaptureSlot`), `craft-menu`, `craft-dialogs`, `craft-keymap`, `craft-platform`, `craft-quit`, `craft-settings`, `craft-update`, and `craft-window`.
- **[`storytold/egui`](https://github.com/storytold/egui)** & **[`storytold/winit`](https://github.com/storytold/winit)** (`MIT OR Apache-2.0`) — `egui`, `eframe`, `egui_extras`, and cross-platform windowing synchronized with `storytold/craft-libs`.
- **[`storytold/craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations)** (`MIT OR Apache-2.0`) — provider authentication and model routing.
- **[`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions)** & **[`storytold/craft-contrib`](https://github.com/storytold/craft-contrib)** — CI/release workflows and workspace house rules.
- **Standard ArtCraft Applications** (exchanging native files on disk and driven via `<craft>-cli` / `--control`): [`PhotoCraft`](https://github.com/storytold/photocraft), [`VectorCraft`](https://github.com/storytold/vectorcraft), [`DesignCraft`](https://github.com/storytold/designcraft), [`EffectCraft`](https://github.com/storytold/effectcraft), and [`FilmCraft`](https://github.com/storytold/filmcraft).
