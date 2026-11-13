# `xtask` — Architecture & Policy Gates (`xtask`, Layer 7)

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

Workspace verification runner enforcing ArtCraft architectural and Rust 1.95 hygiene invariants (`cargo xtask ci | layers | assets | sources | test`).

## Files

- **[`src/main.rs`](src/main.rs)** & **[`src/ci.rs`](src/ci.rs)** — Fast-fail gate runner (`build groups`, `cargo fmt --all --check`, `layers`, `assets`, `sources`, `clippy --all-targets -- -D warnings`, and opt-in `MAGIC_RUN_TESTS=1`).
- **[`src/policy.rs`](src/policy.rs)** — Downward `L4 -> L7` DAG check, headless `L4`/`L5` UI-crate isolation, sibling-app crate ban, [`ATTRIBUTION.md`](../ATTRIBUTION.md) asset check, and `< 900`-line / zero-`.unwrap()` / pure-`ui-egui` source verification.
