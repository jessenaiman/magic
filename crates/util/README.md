# `magic-util` (`crates/util`)

Headless, UI-free media, timing, typography, embedded font/locale build hooks, and `craft-doc` document inspection primitives shared by `magic-engine`, `magic-ui-egui`, and the workspace binaries.

## Where & Why to Explore Code

| Module | Upstream `storytold` Docs | Why You Explore This File |
| :--- | :--- | :--- |
| **[`src/lib.rs`](src/lib.rs)** & **[`build.rs`](build.rs)** | [`storytold/craft-libs/README.md`](https://github.com/storytold/craft-libs/blob/main/README.md) | Crate entrypoint re-exporting `anim`, `media`, `text`, `time`, `CRAFT_FONTS` (`CRAFT_FONTS_DIR` via `craft-fonts-build`), and `I18N` (`CRAFT_I18N_DIR` via `craft-i18n`) with `#![forbid(unsafe_code)]`. |
| **[`src/time.rs`](src/time.rs)** | [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) | Exact `craft-timeline` `Tick` math (`TICKS_PER_SECOND` = `254,016,000,000`), rational `FrameRate` presets (`23.976`, `24`, `25`, `29.97` DF/NDF, `30`, `50`, `59.94`, `60`), SMPTE `Timecode` parsing/formatting, and timeline quantization. |
| **[`src/media.rs`](src/media.rs)** | [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) | Audio/video container probing (`isobmff`, `matroska`, `ogg`, `wav`, `aiff`, `flac`), multi-bin PCM waveform peaks via `craft_audio`, and `.deckcraft`, `.ecproj`, `.pcraft`, `.vectorcraft`, and `.fcproj` preview inspection via `craft_doc`. |
| **[`src/text.rs`](src/text.rs)** | [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) | Font family resolution via `craft_font_catalog::Catalog::global()` and lyric/caption line breaking with Unicode UAX #14, CJK kinsoku rules, and UAX #9 BiDi directionality via `craft-text`. |
| **[`src/anim.rs`](src/anim.rs)** | [`EffectCraft` `docs/agents.md`](https://github.com/storytold/effectcraft/blob/main/docs/agents.md) | Cubic-Bezier keyframe easing curves (`linear`, `easeIn`, `easeOut`, `easeInOut`, `snappy`) shared between timeline previews and `EffectCraft` keyframe export. |
