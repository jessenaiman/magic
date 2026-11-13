# Building & Quality Gates

## Local Build & Install

```sh
./install.sh --from-source
```

`./install.sh --from-source` runs `cargo update` automatically so local builds pull the latest [`storytold/craft-libs`](https://github.com/storytold/craft-libs) `main` commits, then builds `magic-cli`, `magic`, and `magic-tui` in `--release` mode.

## Workspace Gate (`cargo xtask ci`)

Before opening a pull request, run the workspace formatting and Clippy gate:

```sh
cargo xtask ci
```

`cargo xtask ci` executes in order:
1. **Formatting** (`cargo fmt --all --check`).
2. **Clippy lint gate** (`cargo clippy --workspace --all-targets --message-format=short -- -D warnings`) — enforces `[workspace.lints]` in [`Cargo.toml`](../../../Cargo.toml) (`unsafe_code = "forbid"`, `nonstandard_style`, `disallowed_names`, `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, `cognitive_complexity`, `too_many_arguments`, `wildcard_imports`).
3. **Opt-in workspace tests (`MAGIC_RUN_TESTS=1`)** — runs `cargo test --workspace -- --test-threads 4` when `MAGIC_RUN_TESTS=1` is set (or directly via `cargo xtask test`).

Every run writes full stage output to `target/ci-report.txt` and `target/ci-report.jsonl`.

## Continuous Integration (`storytold/craft-common-actions`)

GitHub Actions workflows in `.github/workflows/` delegate to [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions):
- `ci.yml` -> `git-sources.yml@main` + `gate.yml@main`
- `packaging-lint.yml` -> `packaging-lint.yml@main`
- `release.yml` -> `git-sources.yml@main` + `refresh-contributors.yml@main` + `release.yml@main`
- `mdbook.yml` -> builds `mdbook build book` and deploys GitHub Pages.
