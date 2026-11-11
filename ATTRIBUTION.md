# Attribution & Third-Party Notices

Magic Studio builds directly on community-maintained open-source Rust crates and interoperates with the ArtCraft creative suite:

- **[storytold/craft-libs](https://github.com/storytold/craft-libs)** (`MIT OR Apache-2.0`): `craft-atomic`, `craft-audio`, `craft-dialogs`, `craft-doc`, `craft-docpath`, `craft-ecproj-format`, `craft-font-catalog`, `craft-keymap`, `craft-media`, `craft-menu`, `craft-pcraft-format`, `craft-platform`, `craft-quit`, `craft-settings`, `craft-ui`, `craft-update`, `craft-vectorcraft-format`, `craft-window`.
- **[storytold/egui](https://github.com/storytold/egui)** & **[storytold/winit](https://github.com/storytold/winit)** (`MIT OR Apache-2.0`): Pure-Rust GPU UI (`egui` 0.37, `eframe` 0.37, `egui_extras` 0.37) and native windowing.
- **[storytold/craft-ai-integrations](https://github.com/storytold/craft-ai-integrations)** (`MIT OR Apache-2.0`): `craft-ai-codex` OAuth / provider integration.
- **[storytold/craft-common-actions](https://github.com/storytold/craft-common-actions)** & **[storytold/craft-contrib](https://github.com/storytold/craft-contrib)**: Shared CI/release workflows, workspace architecture layers, and contribution house rules.
- **Sibling ArtCraft Applications** (invoked as external processes via `--control` and `<product>-cli`, never linked as Cargo crate dependencies):
  - [FilmCraft](https://github.com/storytold/filmcraft)
  - [EffectCraft](https://github.com/storytold/effectcraft)
  - [DesignCraft](https://github.com/storytold/designcraft)
  - [PhotoCraft](https://github.com/storytold/photocraft)
  - [VectorCraft](https://github.com/storytold/vectorcraft)
  - [AniCraft](https://github.com/storytold/AniCraft) (`anicraft-cli` headless CLI only)
