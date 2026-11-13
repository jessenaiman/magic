# Contributing to Magic Studio

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

Magic Studio follows the [ArtCraft contribution standards (`storytold/craft-contrib`)](https://github.com/storytold/craft-contrib/blob/main/CONTRIBUTING.md) and [`docs/house-rules-core.md`](docs/house-rules-core.md).

## Quality Gates

Run the workspace compile, source-hygiene, and lint gate before pushing a branch or opening a pull request into `main`:

```sh
cargo xtask ci
```

`cargo xtask ci` runs:
1. `SETS` workspace crate coverage check ([`xtask/src/ci.rs`](xtask/src/ci.rs))
2. `cargo fmt --all --check` (`.rustfmt.toml`, `max_width = 160`)
3. `cargo xtask layers` — enforcing `magic-util` / `magic-scribbles` (`L4`) -> `magic-engine` (`L5`) -> `magic-mcp` / `magic-ui-egui` (`L6`) -> `magic` / `magic-cli` / `magic-tui` (`L7`), keeping `L4`/`L5` UI-free, and forbidding sibling Craft app crates in `Cargo.toml` ([`xtask/src/policy.rs`](xtask/src/policy.rs))
4. `cargo xtask assets` — verifying [`ATTRIBUTION.md`](ATTRIBUTION.md) coverage across `assets/`, `docs/brand/`, and `figcraft/`
5. `cargo xtask sources` — enforcing `< 900` lines per `.rs` file, zero production `.unwrap()`/`todo!()`/`unimplemented!()`, and zero socket I/O or JSX/TSX strings in `crates/ui-egui/`
6. `cargo clippy --all-targets -- -D warnings` across all workspace packages (with tests opt-in via `MAGIC_RUN_TESTS=1`)

## Commit, Branch & Documentation Rules

- **Read Actual `README.md` Words First**: Always read [`README.md`](README.md) and the upstream `https://github.com/storytold/<repo>/blob/main/README.md` first before touching code, and update Markdown files **only as the final step** after code changes pass verification.
- **No Paraphrased Upstream Docs**: Link directly to maintained `https://github.com/storytold/<repo>` `README.md` and `docs/` pages rather than copying or paraphrasing them. Stamp every modified Markdown file with `Last Reviewed & Updated` and reviewer attribution.
- **Keep `storytold/craft-libs` Pinned per Upstream `README.md`**: Per [`storytold/craft-libs/README.md`](https://github.com/storytold/craft-libs/blob/main/README.md), workspace dependencies use `{ git = "https://github.com/storytold/craft-libs", rev = "<commit>" }` (never maintain a local fork of `craft-libs`).
- **Sequential PR Branches Only**: Never push to `main`. Push sequentially named branches (such as `magic-fixes-9`) and open a Pull Request into `main` without AI/agent attribution trailers.
