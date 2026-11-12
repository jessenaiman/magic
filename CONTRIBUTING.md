# Contributing to Magic Studio

Magic Studio follows the [ArtCraft contribution standards (`storytold/craft-contrib`)](https://github.com/storytold/craft-contrib/blob/main/CONTRIBUTING.md) and [House Rules](https://github.com/storytold/craft-contrib/blob/main/docs/house-rules-core.md).

## Quality Gates

Run the workspace gate before submitting a pull request:

```sh
cargo xtask ci
```

`cargo xtask ci` runs:
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
- **No AI/Agent Trailers**: Never include `Co-Authored-By` agent trailers, `Generated with ...` footers, or session logs in commit messages, pull requests, or issues.
- Report honestly which verification steps were executed on your machine and which were not.
