<<<<<<< HEAD
//! `magic-util`: Reusable, headless Single-Responsibility utilities built on `storytold/craft-libs`.
//!
//! Modules:
//! - [`time`]: Exact rational `Tick` (`254_016_000_000` ticks/s) and `FrameRate` math.
//! - [`anim`]: Pure cubic-Bézier `Easing`, transform `Property`, and symbol `LoopMode` evaluation.
//! - [`media`]: Zero-subprocess file, rendition, container, and audio inspection via `craft-doc`,
//!   `craft-ecproj-format`, `craft-pcraft-format`, `craft-vectorcraft-format`, `craft-audio`,
//!   `craft-media`, and `craft-atomic`.
=======
//! `magic-util`: Headless timing, animation curve, Craft document inspection, audio/container,
//! typography, and localization utilities built on `storytold/craft-libs`.
//!
//! Modules:
//! - [`time`]: Exact rational `Tick` and SMPTE `FrameRate` / `Timecode` math (`craft-timeline`).
//! - [`anim`]: Cubic-Bézier `Easing`, transform `Property`, and symbol `LoopMode` evaluation.
//! - [`media`]: Document preview/outline inspection (`craft-doc`), font discovery (`craft-font-catalog`),
//!   container probing (`craft-media`), and lossless audio I/O & peaks (`craft-audio`).
//! - [`text`]: UAX #14 + kinsoku line segmentation, grapheme-cluster emergency breaks, BiDi, and
//!   hyphenation helpers (`craft-text`).
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable))]

pub mod anim;
pub mod media;
<<<<<<< HEAD
pub mod time;

pub use time::{FrameRate, TICKS_PER_SECOND, Tick, TimeRange};

=======
pub mod text;
pub mod time;

use craft_fonts_build::EmbeddedFace;
use std::sync::LazyLock;

pub use time::{FrameRate, TICKS_PER_SECOND, Tick, TimeRange};

pub static CRAFT_FONTS: &[EmbeddedFace] = include!(concat!(env!("OUT_DIR"), "/craft_fonts.rs"));
include!(concat!(env!("OUT_DIR"), "/craft_i18n.rs"));

/// Offered UI locale codes (`English` default; additional locales come from optional `CRAFT_I18N_DIR`).
pub const OFFERED_LOCALES: &[&str] = &["en"];

/// Shared translation lookup table initialized from optional `CRAFT_I18N_DIR`.
pub static I18N: LazyLock<craft_i18n::I18n> = LazyLock::new(|| {
    let (i18n, problems) = craft_i18n::I18n::new(CRAFT_I18N, OFFERED_LOCALES);
    for problem in &problems {
        log::warn!("translation catalog: {problem}");
    }
    i18n
});

>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
/// Errors returned by `magic-util` media and timing helpers.
#[derive(Debug, thiserror::Error)]
pub enum UtilError {
    #[error("{0}")]
    Invalid(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, UtilError>;

<<<<<<< HEAD
/// Known ArtCraft & open media document extensions recognized on disk without launching sibling apps.
pub fn is_supported_craft_or_media_path(path: &std::path::Path) -> bool {
    if media::is_craft_doc_path(path)
=======
/// Standard Craft suite (`DesignCraft`, `PhotoCraft`, `VectorCraft`, `EffectCraft`, `FilmCraft`)
/// and open media extensions recognized on disk.
pub fn is_supported_craft_or_media_path(path: &std::path::Path) -> bool {
    if media::is_file_or_pcraft_bundle(path)
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
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
<<<<<<< HEAD
                | "deckcraft"
                | "pptx"
                | "pdf"
                | "anicraft"
                | "ani"
                | "tcraft"
=======
                | "indd"
                | "indt"
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
                | "png"
                | "jpg"
                | "jpeg"
                | "webp"
                | "gif"
                | "wav"
<<<<<<< HEAD
=======
                | "aiff"
                | "aif"
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
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
