# Building & Quality Gates

## Workspace Gate (`cargo xtask ci`)

Before opening a pull request, run the workspace gate:

```sh
cargo xtask ci
```

`cargo xtask ci` executes in order:
1. **Build group completeness** (`xtask/src/ci.rs`) — verifies every workspace crate is registered in `SETS` (`core`, `mcp`, `ui`, `apps`).
2. **Formatting** (`cargo fmt --check`).
3. **Architecture layer & sibling isolation check** (`cargo xtask layers`) — enforces `L4 -> L5 -> L6 -> L7`, blocks UI crates in `L4`/`L5`, forbids Tauri/webview crates, and forbids sibling ArtCraft app crate dependencies.
4. **Strict Clippy lint gate** (`cargo clippy --workspace --all-targets -- -D warnings`) — denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, wildcard imports, and direct `std::fs::write`.
5. **Workspace test compilation** (`cargo test --workspace --no-run`).

## Continuous Integration (`storytold/craft-common-actions`)

GitHub Actions workflows in `.github/workflows/` delegate directly to [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions):
- `ci.yml` -> `git-sources.yml@main` + `gate.yml@main`
- `packaging-lint.yml` -> `packaging-lint.yml@main`
- `release.yml` -> `git-sources.yml@main` + `refresh-contributors.yml@main` + `release.yml@main`
- `mdbook.yml` -> builds `mdbook build book` (following [`PhotoCraft`](https://github.com/storytold/photocraft)) and deploys the GitHub Pages site + `/book/`.
