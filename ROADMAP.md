# Magic Studio Roadmap

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

Magic Studio coordinates host [`DesignCraft`](https://github.com/storytold/designcraft/blob/main/README.md), [`PhotoCraft`](https://github.com/storytold/photocraft/blob/main/README.md), [`VectorCraft`](https://github.com/storytold/vectorcraft/blob/main/README.md), [`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/README.md), and [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/README.md) on top of [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md).

## Completed Pre-Alpha Foundation (`magic-fixes-9`)

- [x] **Self-Contained `~/Magic/<slug>/` Workspace Architecture** ([`crates/engine/src/workspace.rs`](crates/engine/src/workspace.rs), [`cmd/project/`](crates/engine/src/cmd/project/mod.rs)): 5-folder project layout (`audio/`, `assets/`, `effects/`, `film/`, `exports/`), `~/.local/share/magic/{templates,recovery,logs}` seeding, and migration-only `original-project.zip` (`project.migrate` via `craft-zip`).
- [x] **Direct `storyboard.designcraft` Package Updates** ([`crates/engine/src/cmd/storyboard/`](crates/engine/src/cmd/storyboard/mod.rs)): Unlocked text/font slots and `assets/*` entries written atomically into `storyboard.designcraft` via `craft-zip` + `craft-atomic`, with spread operations delegated to host `DesignCraft`.
- [x] **`craft_ui::docking` Workstation & Zero-Polling Backstage TUI** ([`crates/ui-egui/src/docking.rs`](crates/ui-egui/src/docking.rs), [`apps/magic-tui/`](apps/magic-tui/README.md)): Shared docking presets, 70+ clean-room `assets/icons/*.svg` controls, and 5-tab ANSI snapshot `magic-tui` with zero third-party TUI bloat.
- [x] **Control Channel & CI Source Hygiene** ([`apps/magic/src/control.rs`](apps/magic/src/control.rs), [`apps/magic/src/capture.rs`](apps/magic/src/capture.rs), [`xtask/src/policy.rs`](xtask/src/policy.rs)): `--control-input-only`, macOS App Nap opt-out, `render.frame` (`max_side`, `base64`, `write`), `ui.playback` `rmsDbfs`/`peakDbfs`, marker `sync_warnings`, and `cargo xtask ci` (`layers`, `assets`, `sources`, `clippy -D warnings`).

## Remaining Host-Verification & Upstream Parity Milestones

- [ ] **Live 3-App `--control` & Host GPU Screenshot Verification**: Run [`HANDOFF-magic-fixes-9.md`](HANDOFF-magic-fixes-9.md) on host GPU with live `FilmCraft` (`:9876`, `FILMCRAFT_CONTROL_TOKEN`), `EffectCraft` (`:9877`), and `DesignCraft` (`:7979`), plus 10-step workflow screenshots.
- [ ] **Upstream FilmCraft Linked Comps & Whisper Transcripts**: Track upstream [`storytold/filmcraft#388`](https://github.com/storytold/filmcraft/issues/388) for native linked `.ecproj` comps and verify `filmcraft-cli` `transcript.generate` / `sequence.transcribe` (`--features whisper`) per [`filmcraft/docs/transcripts.md`](https://github.com/storytold/filmcraft/blob/main/docs/transcripts.md).
- [ ] **Release Signing & Community Wiki**: Wire [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions/blob/main/README.md) Linux/macOS/Windows signing secrets and submit `docs/wiki/magic/` to [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).
