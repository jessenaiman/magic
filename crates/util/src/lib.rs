//! `magic-util`: Reusable, headless Single-Responsibility utilities built on `storytold/craft-libs`.
//!
//! Modules:
//! - [`time`]: Exact rational `Tick` (`254_016_000_000` ticks/s) and `FrameRate` math.
//! - [`anim`]: Pure cubic-Bézier `Easing`, transform `Property`, and symbol `LoopMode` evaluation.
//! - [`media`]: Zero-subprocess file, rendition, container, and audio inspection via `craft-doc`,
//!   `craft-ecproj-format`, `craft-pcraft-format`, `craft-vectorcraft-format`, `craft-audio`,
//!   `craft-media`, and `craft-atomic`.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

pub mod anim;
pub mod media;
pub mod time;

pub use time::{FrameRate, TICKS_PER_SECOND, Tick, TimeRange};

/// Errors returned by `magic-util` media and timing helpers.
#[derive(Debug, thiserror::Error)]
pub enum UtilError {
    #[error("{0}")]
    Invalid(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, UtilError>;

/// Known ArtCraft & open media document extensions recognized on disk without launching sibling apps.
pub fn is_supported_craft_or_media_path(path: &std::path::Path) -> bool {
    if media::is_craft_doc_path(path)
        && let Some(ext) = path.extension().and_then(|e| e.to_str())
    {
        let lower = ext.to_ascii_lowercase();
        return matches!(
            lower.as_str(),
            "magic"
                | "fcproj"
                | "fcgt"
                | "ecproj"
                | "ectemplate"
                | "pcraft"
                | "psd"
                | "psb"
                | "ora"
                | "xcf"
                | "kra"
                | "pdn"
                | "vectorcraft"
                | "vcraft"
                | "drawcraft"
                | "svg"
                | "svgz"
                | "ai"
                | "eps"
                | "designcraft"
                | "idml"
                | "deckcraft"
                | "pptx"
                | "pdf"
                | "anicraft"
                | "ani"
                | "tcraft"
                | "png"
                | "jpg"
                | "jpeg"
                | "webp"
                | "gif"
                | "wav"
                | "mp3"
                | "flac"
                | "m4a"
                | "ogg"
                | "mp4"
                | "mov"
                | "mkv"
                | "webm"
        );
    }
    false
}
