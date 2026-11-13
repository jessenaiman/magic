# Contributing & House Rules

Read [`README.md`](../../../README.md), [`CONTRIBUTING.md`](../../../CONTRIBUTING.md), and [`docs/house-rules-core.md`](../../../docs/house-rules-core.md) ([`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/standards/house-rules-core.md)) before changing code, and navigate directly to each maintained `https://github.com/storytold/<repo>/blob/main/README.md`.

- **Formatting & Clippy (`cargo xtask ci`)**: Runs `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --message-format=short -- -D warnings` (plus `MAGIC_RUN_TESTS=1` workspace tests), writing full logs to `target/ci-report.txt` and `target/ci-report.jsonl`.
- **Shared `storytold/craft-libs` Dependencies**: Workspace dependencies track `https://github.com/storytold/craft-libs` (including `craft-ai-core`) on `main`; refresh `Cargo.lock` with `cargo update`.
- **Pull Requests Only**: Never push directly to `main`; open a pull request into `main` without agent attribution trailers (`docs/house-rules-core.md` Rule 8 & Rule 15).
