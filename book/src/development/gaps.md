# Magic Studio Feature Gaps & Host Verification Checklist

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

This document tracks remaining live-host verification items and upstream dependencies without paraphrasing upstream specs. Always verify upstream behavior directly in the corresponding `storytold/*` `README.md` and `docs/` files.

## Completed in `magic-fixes-9`

- [x] Pinned [`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md) via `rev` (`d23f5f36bb9657159ba5468aa9c92d939786922b`) per `craft-libs/README.md` line 23 and removed unused `craft-extensions` (`craft-ext-core`/`craft-ext-store`) dependencies.
- [x] Consolidated projects into `~/Magic/<slug>/` (`audio/`, `assets/`, `effects/lyrics.ecproj`, `effects/motion.ecproj`, `film/<slug>.fcproj`, `exports/`) with migration-only `original-project.zip`.
- [x] Wired direct `storyboard.designcraft` ZIP story/asset updates via `craft-zip` + `craft-atomic` and `template_seed_bytes` template loading.
- [x] Added `--control-input-only`, macOS App Nap guard, `render.frame {max_side?, base64?, write?}` ([`effectcraft/docs/control-protocol.md`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md)), deduplicated `rmsDbfs`/`peakDbfs` telemetry, `magic_menu_bar` title-bar drag area ([`craft-window/README.md`](https://github.com/storytold/craft-libs/blob/main/crates/window/README.md)), decomposed high-cognitive-complexity functions (`rig_track_native_steps`, `trace_silhouette_svg`, `cmd_project_trunk`, `write_state_into_designcraft_zip`, `validate_line_item`, `mode_tab`, `font_family_combo`, `policy::sources`) below Clippy's `cognitive-complexity-threshold = 30`, and added `cargo xtask sources`.

## Open Host-Only & Upstream Items

- [ ] **Host `cargo xtask ci` & Live 3-App Socket Proof (`U01`, `U03`, `U11`, `U16`)**: Run `cargo xtask ci`, live `--control` round-trips across `Magic` (`:9875`), `FilmCraft` (`:9876`, `FILMCRAFT_CONTROL_TOKEN`), and `EffectCraft` (`:9877`), and host GPU screenshots (`1440x900`, `1600x950`, `960x720`).
- [ ] **Read-Only MCP Resource URIs (`U13`)**: Expose `magic://document`, `magic://storyboard`, and `magic://commands` on `crates/mcp/src/server.rs` verified against `rmcp` `3.9` (`modelcontextprotocol/rust-sdk`).
- [ ] **Upstream FilmCraft Linked Comps (`U10`)**: Track [`storytold/filmcraft#388`](https://github.com/storytold/filmcraft/issues/388) (`PR #3201` still open upstream).
- [ ] **Host CLI Capability Checks (`U14`, `U15`, `U20`)**: Verify `EffectCraft` `renderer: "advanced3D"` standard glTF 2.0 `JOINTS_0`/`WEIGHTS_0` skinning, `filmcraft-cli` `transcript.generate` / `sequence.transcribe` (`--features whisper`), and installed host CLI capabilities via `magic-cli exec app.doctor`.
