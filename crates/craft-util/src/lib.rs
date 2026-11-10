//! `craft-util`: Reusable, headless Single-Responsibility utilities for working with the
//! system-installed ArtCraft App Suite (`storytold/craft-launcher` and `storytold/craft-libs`).
//!
//! Modules:
//! - [`time`]: Exact rational `Tick` (`254_016_000_000` ticks/s) and `FrameRate` math.
//! - [`anim`]: Pure cubic-Bézier `Easing`, transform `Property`, and symbol `LoopMode` evaluation.
//! - [`target`]: Upstream ArtCraft `--control`, `<product>-cli`, and MCP protocol descriptors.
//! - [`discovery`]: ArtCraft Launcher `installed.json` / `.desktop` / `PATH` binary lookup,
//!   `0600` session token files via `craft-atomic`, and credential scrubbing.
//! - [`media`]: Fast in-process header/outline inspection via `craft-doc`, `craft-ecproj-format`,
//!   `craft-audio`, and `image::image_dimensions`.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

pub mod anim;
pub mod discovery;
pub mod media;
pub mod target;
pub mod time;

pub use target::{ControlAuth, ControlWire, CraftTarget};
pub use time::{FrameRate, TICKS_PER_SECOND, Tick, TimeRange};

/// Errors returned by `craft-util` discovery, token, and media helpers.
#[derive(Debug, thiserror::Error)]
pub enum UtilError {
    #[error("{0}")]
    Invalid(String),
    #[error("external tool `{0}`: {1}")]
    Tool(String, String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, UtilError>;
