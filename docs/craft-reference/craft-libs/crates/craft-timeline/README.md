# `craft-timeline` Reference (`storytold/craft-libs`)

Upstream crate: [`storytold/craft-libs` `crates/timeline`](https://github.com/storytold/craft-libs) (`craft-timeline`, features `["serde", "xml", "otio"]`).

Exact 25,401,600-ticks-per-second rational timebase (`time::Tick`, `rate::FrameRate`, `time::Timecode`) and multi-track NLE interchange codec supporting **CMX 3600 EDL** (`edl`), **Avid Log Exchange** (`ale`), **Final Cut Pro 7 XMEML** (`fcp7`), **Final Cut Pro FCPXML** (`fcpxml`), and **OpenTimelineIO** (`otio`), plus automatic `detect_text` / `decode_text` and conform `report` diagnostics.

## Where Magic Studio Uses `craft-timeline`

- **Exact Timebase & SMPTE Math ([`crates/util/src/time.rs`](../../../../crates/util/src/time.rs))**: Re-exports and wraps `craft_timeline::time` and `craft_timeline::rate` across the workspace.
- **NLE Timeline Interchange & Markers ([`crates/engine/src/cmd/film/timeline_interchange.rs`](../../../../crates/engine/src/cmd/film/timeline_interchange.rs), [`markers.rs`](../../../../crates/engine/src/cmd/film/markers.rs))**: Converts `film/<song>.fcproj` (`V4`, `V3`, `V2`, `V1`, `C1`, `A1`) to and from `EDL`, `ALE`, `FCP7 XML`, `FCPXML`, and `OTIO`.
- **CLI & UI ([`apps/magic-cli/src/main.rs`](../../../../apps/magic-cli/src/main.rs), [`crates/ui-egui/src/views/project_interchange.rs`](../../../../crates/ui-egui/src/views/project_interchange.rs))**: Powers `magic-cli timeline inspect|import|export` and the `00 Project` / `05 Studio` interchange workflows.
