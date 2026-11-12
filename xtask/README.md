# `xtask` — Architecture & Policy Gates (`xtask`, Layer 7)

Workspace verification runner enforcing ArtCraft architectural invariants (`cargo xtask ci`, `cargo xtask layers`, `cargo xtask assets`).

## Where & Why to Explore Code

| File | Related Docs | Why You Explore This File |
| :--- | :--- | :--- |
| **[`src/main.rs`](src/main.rs)** | [`docs/architecture.md`](../docs/architecture.md) · [`CONTRIBUTING.md`](../CONTRIBUTING.md) | Explore to see subcommand orchestration for `ci` (`rustfmt --check`, `layers`, `assets`, `clippy -D warnings`, and opt-in `MAGIC_RUN_TESTS=1` test runs). |
| **[`src/policy.rs`](src/policy.rs)** | [`ATTRIBUTION.md`](../ATTRIBUTION.md) · [`AGENTS.md`](../AGENTS.md) | Explore to see the exact `CRATE_LAYERS` (`L4` -> `L5` -> `L6` -> `L7`) downward-only DAG check, UI-crate isolation below `L6`, sibling-app dependency ban, `egui_kittest` ban, and the `assets/` + `docs/brand/` + `figcraft/` attribution gate against [`ATTRIBUTION.md`](../ATTRIBUTION.md). |
