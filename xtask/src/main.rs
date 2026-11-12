//! Repository tasks aligned with upstream ArtCraft conventions (`cargo xtask ci | layers | assets | test`).

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

mod ci;
mod policy;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo xtask")]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Run all quality gates in fast-fail order: build groups, fmt, layers, assets, Clippy (`-D warnings`), and workspace tests.
    Ci,
    /// Check Magic's crate dependency DAG, UI toolkit boundary, pure-Rust non-Tauri rule, and sibling-crate isolation.
    Layers,
    /// Verify every bundled file under `assets/`, `docs/brand/`, and `figcraft/` is attributed in `ATTRIBUTION.md`.
    Assets,
    /// Run workspace tests with bounded concurrency (`RUST_TEST_THREADS=4`) and a hang watchdog (`MAGIC_TEST_TIMEOUT_MINUTES`).
    Test,
}

type Result<T> = std::result::Result<T, String>;

fn main() {
    let result = match Cli::parse().task {
        Task::Ci => ci::ci(),
        Task::Layers => policy::layers(),
        Task::Assets => policy::assets(),
        Task::Test => ci::tests(),
    };
    if let Err(error) = result {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}
