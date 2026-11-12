# Architecture & Crate Layers

Magic Studio enforces a strict downward-only crate DAG verified by `cargo xtask ci` (`xtask/src/policy.rs`):

- **L3 (`storytold/craft-libs` & `storytold/craft-ai-integrations`)**: Shared file formats (`craft-doc`, `craft-ecproj-format`, `craft-pcraft-format`, `craft-vectorcraft-format`), `craft-audio`, `craft-media`, `craft-atomic`, `craft-settings`, `craft-ui`, `craft-keymap`, `craft-window`, `craft-dialogs`.
- **L4 (`magic-util`, `magic-scribbles`)**: Pure rational `Tick` timing, cubic-Bézier curves, zero-subprocess Craft document inspection, and prompt/grid helpers.
- **L5 (`magic-engine`)**: Headless `Session`, command registry, pure-Rust `native_media` cutouts/silhouettes, and `.magic` project persistence. Never depends on UI crates or sibling ArtCraft product crates.
- **L6 (`magic-mcp`, `magic-ui-egui`)**: Standard stdio MCP server and pure-Rust `egui` 0.37 workstation views.
- **L7 (`magic`, `magic-cli`, `magic-tui`, `xtask`)**: Native `eframe` desktop binary (`--control 9875`), CLI, TUI, and CI policy runner.
