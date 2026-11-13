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
    /// Run quality gates in fast-fail order: build groups, fmt, layers, assets, and Clippy (`-D warnings`; tests opt-in via `MAGIC_RUN_TESTS=1`).
    Ci,
    /// Check Magic's crate dependency DAG, headless L4/L5 boundary, sibling-crate isolation, and no `egui_kittest` bloat.
    Layers,
    /// Verify every bundled file under `assets/`, `docs/brand/`, and `figcraft/` is attributed in `ATTRIBUTION.md`.
    Assets,
    /// Verify Rust source file length (`< 900` lines), zero production `.unwrap()`/`todo!()`, and `crates/ui-egui` purity.
    Sources,
    /// Explicitly run workspace tests when opted in after live verification (`RUST_TEST_THREADS=4`, `MAGIC_TEST_TIMEOUT_MINUTES`).
    Test,
}

type Result<T> = std::result::Result<T, String>;

fn main() {
    let result = match Cli::parse().task {
        Task::Ci => ci::ci(),
        Task::Layers => policy::layers(),
        Task::Assets => policy::assets(),
        Task::Sources => policy::sources(),
        Task::Test => ci::tests(),
    };
    if let Err(error) = result {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}
