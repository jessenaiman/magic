//! Repository tasks for Magic Studio (`cargo xtask ci | test`).

mod ci;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo xtask")]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Run quality gates (`fmt` and `clippy -- -D warnings`; opt-in `MAGIC_RUN_TESTS=1`) and write `target/ci-report.{txt,jsonl}`.
    Ci,
    /// Explicitly run workspace tests (`RUST_TEST_THREADS=4`, `MAGIC_TEST_TIMEOUT_MINUTES`).
    Test,
}

type Result<T> = std::result::Result<T, String>;

fn main() {
    let result = match Cli::parse().task {
        Task::Ci => ci::ci(),
        Task::Test => ci::tests(),
    };
    if let Err(error) = result {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}
