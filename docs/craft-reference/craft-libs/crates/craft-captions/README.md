# `craft-captions` Reference (`storytold/craft-libs`)

Upstream crate: [`storytold/craft-libs` `crates/captions`](https://github.com/storytold/craft-libs) (`craft-captions`).

Broadcast and web caption/subtitle reader, validator, and writer supporting **SubRip (`SRT`)**, **WebVTT (`VTT`)**, **TTML / IMSC1 (`TTML`)**, **CEA-608 Scenarist Closed Caption (`SCC`)**, **CEA-708 MacCaption (`MCC`)**, and **EBU-STL (`STL`)**.

## Where Magic Studio Uses `craft-captions`

- **`FilmCraft` `C1` Caption Track & Export ([`crates/engine/src/cmd/film/fcproj_doc.rs`](../../../../crates/engine/src/cmd/film/fcproj_doc.rs), [`fcproj_stage.rs`](../../../../crates/engine/src/cmd/film/fcproj_stage.rs))**: Syncs timed lyric/caption cues onto `FilmCraft` track `C1` and imports/exports `SRT`, `WebVTT`, `TTML`, `SCC`, `MCC`, and `EBU-STL` sidecars without custom regex parsers.
- **`05 Studio` Captions Dock Panel ([`crates/ui-egui/src/views/studio_audio_titles.rs`](../../../../crates/ui-egui/src/views/studio_audio_titles.rs))**: Drives the `PanelId::Captions` (`captions-cc` SVG icon) cue table and broadcast format selector.
