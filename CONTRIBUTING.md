# Contributing to Magic Studio

<<<<<<< HEAD
Magic Studio follows the [ArtCraft contribution standards (`storytold/craft-contrib`)](https://github.com/storytold/craft-contrib/blob/main/CONTRIBUTING.md) and [House Rules](https://github.com/storytold/craft-contrib/blob/main/docs/house-rules-core.md).

## Quality Gates

Run the workspace gate before submitting a pull request:
=======
Magic Studio follows the [ArtCraft contribution standards (`storytold/craft-contrib`)](https://github.com/storytold/craft-contrib/blob/main/CONTRIBUTING.md) and [House Rules](docs/house-rules-core.md).

## Quality Gates

Run the workspace compile and lint gate before submitting a pull request:
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

```sh
cargo xtask ci
```

`cargo xtask ci` runs:
<<<<<<< HEAD
1. `SETS` workspace crate coverage check (`xtask/src/ci.rs`)
2. `cargo fmt --check`
3. `cargo xtask layers` (enforcing `magic-util` / `magic-scribbles` L4 -> `magic-engine` L5 -> `magic-mcp` / `magic-ui-egui` L6 -> `magic` / `magic-cli` / `magic-tui` L7, and forbidding sibling app crate imports)
4. `cargo clippy --workspace --all-targets -- -D warnings` (with `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, and `unreachable` denied in production code)
5. `cargo test --workspace --no-run`

## Commit & Pull Request Rules

- Pure Rust only (`eframe` + `egui` 0.37 + `storytold/craft-libs`) — no Tauri, webview, JavaScript, or TypeScript.
- Keep every module single-responsibility and under 900 lines.
- Delegate raster cutout, vector tracing, motion compositing, and video editing to installed ArtCraft applications through `CraftHub` rather than hand-rolling duplicate implementations.
- Leave explicit `// TODO:` comments (not `todo!()` panics) where follow-up host verification is needed.
=======
1. `SETS` workspace crate coverage check ([`xtask/src/ci.rs`](xtask/src/ci.rs))
2. `cargo fmt --all --check`
3. `cargo xtask layers` — enforcing `magic-util` / `magic-scribbles` (`L4`) -> `magic-engine` (`L5`) -> `magic-mcp` / `magic-ui-egui` (`L6`) -> `magic` / `magic-cli` / `magic-tui` (`L7`), keeping `L4`/`L5` UI-free, and forbidding sibling Craft app crates in `Cargo.toml` ([`xtask/src/policy.rs`](xtask/src/policy.rs))
4. `cargo xtask assets` — verifying [`ATTRIBUTION.md`](ATTRIBUTION.md) coverage across `assets/`, `docs/brand/`, and `figcraft/`
5. `cargo clippy --workspace --all-targets -- -D warnings` (with `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, and `unreachable` denied in production code)

## Commit & Pull Request Rules

- **Study `README.md` & Upstream `storytold` Docs First**: Read [`README.md`](README.md), [`AGENTS.md`](AGENTS.md), and [`storytold/craft-libs`](https://github.com/storytold/craft-libs) before changing UI or dependency manifests. Keep `Cargo.toml` synchronized with the active `storytold/craft-libs`, `storytold/craft-extensions`, and `storytold/craft-ai-integrations` contracts, and keep `craft-fonts` (`CRAFT_FONTS_DIR`) + `craft-i18n` (`CRAFT_I18N_DIR`) strictly as optional `build.rs` environment inputs.
- **Host Craft Applications via `--control` & CLI**: Exchange native project files on disk and drive host `DesignCraft`, `PhotoCraft`, `VectorCraft`, `EffectCraft`, and `FilmCraft` through their `--control` TCP sockets and `<app>-cli` / MCP tools ([`crates/engine/src/craft_api.rs`](crates/engine/src/craft_api.rs)).
- **Single-Responsibility `< 900` Lines**: Keep every `.rs` module focused on one concern and strictly under 900 lines.
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
- **No AI/Agent Trailers**: Never include `Co-Authored-By` agent trailers, `Generated with ...` footers, or session logs in commit messages, pull requests, or issues.
- Report honestly which verification steps were executed on your machine and which were not.
