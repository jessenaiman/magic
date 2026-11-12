# Contributing & House Rules

Magic Studio follows [`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/CONTRIBUTING.md) and [`House Rules`](https://github.com/storytold/craft-contrib/blob/main/docs/house-rules-core.md):

<<<<<<< HEAD
- **Pure Rust & `< 900` Lines**: Keep every `.rs` file focused on one concern and under 900 lines. Never add Tauri, Electron, webview, JavaScript, or TypeScript UI dependencies.
- **Delegate to ArtCraft CLIs**: Never reimplement pixel matting, SVG tracing, puppet deformation, or NLE timeline rendering in Magic; invoke `photocraft-cli`, `vectorcraft-cli`, `designcraft-cli`, `effectcraft-cli`, and `filmcraft-cli` through `CraftHub`.
- **No Paraphrased Skill Dumps**: Do not add custom animation how-to markdowns or agent log files to the repo; link to upstream ArtCraft documentation and record missing host checks with `// TODO:` comments (never `todo!()`).
- **Zero AI Attribution Trailers**: Never add `Co-Authored-By` trailers or `Generated with ...` footers to commits, PRs, or issues.
=======
- **Single-Responsibility `< 900` Lines & Unfrozen `egui`**: Keep every `.rs` file focused on one concern and under 900 lines. Read [`README.md`](../../../README.md) and [`emilk/egui` `README.md`](https://github.com/emilk/egui) first — never treat `egui` as frozen at a static version; synchronize `Cargo.toml` with the active [`storytold/craft-libs`](https://github.com/storytold/craft-libs) target. Custom GPU shaders (`egui::Shape::Callback`), compatible `egui` extension crates, and [`craft-web-host`](https://github.com/storytold/craft-libs/issues/358) (`?host=parent` `<iframe>` + `craft-web-api`) for official `*-web` ArtCraft apps across the six standard Craft types (`PhotoCraft`, `VectorCraft`, `DesignCraft`, `EffectCraft`, `FilmCraft`) are supported at `L6`/`L7`. external archived project and external projects are converted once into native `.magic` projects on Open / Convert (`project.archive`) and are never hosted live.
- **Direct ArtCraft File & MCP Collaboration**: Never reimplement pixel matting, SVG tracing, puppet deformation, or NLE timeline rendering in Magic, and never re-introduce proxy wrappers; exchange files on disk via `storytold/craft-libs` and drive sibling apps directly through each app's `<app>-cli mcp` / `--control` channel (or `craft-web-host` handoffs).
- **Live-First Verification (Zero `egui_kittest` Pollution)**: Do not maintain upstream `egui` widget test harnesses in Magic Studio. `cargo xtask ci` gates `cargo test` behind `MAGIC_RUN_TESTS=1` until live end-to-end verification passes.
- **No Paraphrased Skill Dumps & Zero AI Attribution Trailers**: Link to upstream ArtCraft documentation, record missing host checks with `// TODO:` comments (never `todo!()`), and never add `Co-Authored-By` or `Generated with ...` trailers.
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
