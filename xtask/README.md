# `xtask` — Workspace CI & Test Runner (`xtask`)

Workspace verification runner (`cargo xtask ci | test`), writing human- and agent-readable diagnostic logs to `target/ci-report.txt` and `target/ci-report.jsonl`.

## Files

- **[`src/main.rs`](src/main.rs)** & **[`src/ci.rs`](src/ci.rs)** — Runs `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --message-format=short -- -D warnings` (enforcing [`Cargo.toml`](../Cargo.toml) `[workspace.lints]` and [`clippy.toml`](../clippy.toml)), and opt-in `MAGIC_RUN_TESTS=1` (`cargo test --workspace -- --test-threads 4`, or directly via `cargo xtask test`), recording stage output to `target/ci-report.txt` and `target/ci-report.jsonl`.
