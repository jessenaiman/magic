//! `magic-mcp`: standalone MCP server binary for `magic-cli mcp`, matching the ArtCraft suite
//! (`magic-cli mcp` and `magic-mcp` both speak newline-delimited JSON-RPC 2.0 over stdio).
//!
//! Modes:
//! - `magic-mcp` (tries `127.0.0.1:9876` once, falls back to headless)
//! - `magic-mcp --connect 127.0.0.1:9876` (requires live `magic --control 9876`)
//! - `magic-mcp --headless`

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

use std::io::Write;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "magic-mcp",
    about = "Magic Studio Model Context Protocol server (stdio JSON-RPC 2.0; binary entrypoint for `magic-cli mcp`)"
)]
struct McpCli {
    /// Connect to a running `magic --control <PORT>` window (`127.0.0.1:9876` or `9876`)
    #[arg(long, visible_alias = "bridge")]
    connect: Option<String>,
    /// Run an in-process headless engine session without probing `--control`
    #[arg(long)]
    headless: bool,
    /// Open this `.magic` project on startup (headless mode)
    #[arg(long)]
    project: Option<String>,
    /// Confine every read to this directory (repeatable; implies `--headless`)
    #[arg(long = "automation-read-root")]
    automation_read_roots: Vec<String>,
    /// Confine every write to this directory (repeatable; implies `--headless`)
    #[arg(long = "automation-write-root")]
    automation_write_roots: Vec<String>,
}

fn main() {
    let args = McpCli::parse();
    if let Err(err) = magic_mcp::run_mcp_server(
        "magic-mcp",
        args.connect.as_deref(),
        args.headless,
        &args.automation_read_roots,
        &args.automation_write_roots,
        args.project.as_deref(),
    ) {
        let _ = writeln!(std::io::stderr(), "{err}");
        std::process::exit(1);
    }
}
