# Contributing to Magic Studio

Magic Studio follows the [ArtCraft contribution standards (`storytold/craft-contrib`)](https://github.com/storytold/craft-contrib/blob/main/CONTRIBUTING.md), [`docs/house-rules-core.md`](docs/house-rules-core.md), and the standard compiler/Clippy lint setup in [`Cargo.toml`](Cargo.toml).

## Quality Gates (`cargo xtask ci`)

Run the workspace gate before pushing a branch or opening a pull request into `main`:

```sh
cargo xtask ci
```

`cargo xtask ci` runs:
1. `cargo fmt --all --check` ([`rustfmt.toml`](rustfmt.toml), `max_width = 160`)
2. `cargo clippy --workspace --all-targets --message-format=short -- -D warnings` — enforcing `[workspace.lints]` (`unsafe_code = "forbid"`, `nonstandard_style`, `disallowed_names`, `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `unreachable`, `cognitive_complexity`, `too_many_arguments`, `wildcard_imports`) with [`clippy.toml`](clippy.toml)
3. Optional unit/integration tests (`cargo test --workspace -- --test-threads 4`) when `MAGIC_RUN_TESTS=1` is set (or directly via `cargo xtask test`)

Every run writes stage timings and compiler/Clippy diagnostics to `target/ci-report.txt` and `target/ci-report.jsonl`.

## Dependencies, Branches & Pull Requests

- **Shared `storytold/craft-libs` Dependencies**: Workspace crates depend directly on `{ git = "https://github.com/storytold/craft-libs" }` (including `craft-ai-core`) so `cargo update` and `./install.sh --from-source` track `main` cleanly.
- **Upstream Documentation**: Link directly to live `https://github.com/storytold/<repo>` `README.md` and `docs/` pages rather than maintaining offline snapshots.
- **Pull Requests Only**: Never push directly to `main`. Open a focused pull request into `main` with no agent attribution trailers (`docs/house-rules-core.md` Rule 8 & Rule 15).
