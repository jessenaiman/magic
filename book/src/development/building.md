# Building & Quality Gates

## Workspace Gate (`cargo xtask ci`)

<<<<<<< HEAD
Before opening a pull request, run the workspace gate:
=======
Before opening a pull request, run the workspace compile and policy gate:
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

```sh
cargo xtask ci
```

`cargo xtask ci` executes in order:
<<<<<<< HEAD
1. **Build group completeness** (`xtask/src/ci.rs`) — verifies every workspace crate is registered in `SETS` (`core`, `mcp`, `ui`, `apps`).
2. **Formatting** (`cargo fmt --check`).
3. **Architecture layer & sibling isolation check** (`cargo xtask layers`) — enforces `L4 -> L5 -> L6 -> L7`, blocks UI crates in `L4`/`L5`, forbids Tauri/webview crates, and forbids sibling ArtCraft app crate dependencies.
4. **Strict Clippy lint gate** (`cargo clippy --workspace --all-targets -- -D warnings`) — denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, wildcard imports, and direct `std::fs::write`.
5. **Workspace test compilation** (`cargo test --workspace --no-run`).
=======
1. **Build group completeness** (`xtask/src/ci.rs`) — verifies every workspace crate is registered in `SETS`.
2. **Formatting** (`cargo fmt --all --check`).
3. **Architecture layer & sibling isolation check** (`cargo xtask layers`) — enforces `L4 -> L5 -> L6 -> L7`, keeps `L4`/`L5` headless, forbids sibling ArtCraft app crate dependencies, and blocks `egui_kittest` bloat (while allowing `craft-web-host` iframes, child `wry` webviews, and `egui` shaders/extensions at `L6`/`L7`).
4. **Asset attribution check** (`cargo xtask assets`) — verifies `ATTRIBUTION.md` covers bundled assets.
5. **Strict Clippy lint gate** (`cargo clippy --workspace --all-targets -- -D warnings`) — denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, wildcard imports, and direct `std::fs::write`.
6. **Opt-in tests (`MAGIC_RUN_TESTS=1`)** — skipped by default until live multi-app `--control` verification passes.
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

## Continuous Integration (`storytold/craft-common-actions`)

GitHub Actions workflows in `.github/workflows/` delegate directly to [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions):
- `ci.yml` -> `git-sources.yml@main` + `gate.yml@main`
- `packaging-lint.yml` -> `packaging-lint.yml@main`
- `release.yml` -> `git-sources.yml@main` + `refresh-contributors.yml@main` + `release.yml@main`
- `mdbook.yml` -> builds `mdbook build book` (following [`PhotoCraft`](https://github.com/storytold/photocraft)) and deploys the GitHub Pages site + `/book/`.
