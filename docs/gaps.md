# Magic Studio Feature Gaps & Unimplemented Requests

This document tracks every missing feature, unimplemented request, `// TODO:` host-verification hook, and upstream ArtCraft parity enhancement in Magic Studio, following the gap-tracking convention in [`EffectCraft` `docs/gaps.md`](https://github.com/storytold/effectcraft/blob/main/docs/gaps.md) and [`PhotoCraft` `ROADMAP.md`](https://github.com/storytold/photocraft/blob/main/ROADMAP.md).

## 1. `--control` Protocol & Live Automation Enhancements

Nice-to-have control methods and runtime flags identified when cross-checking `apps/magic/src/{control.rs,main.rs}` against [`EffectCraft` `docs/control-protocol.md`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md) and [`FilmCraft` `docs/control-protocol.md`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md).

- [ ] Add `--control-input-only` flag to `apps/magic` so UI test harnesses can restrict the `--control` socket to `ui.*` input/inspection calls and block direct `engine.execute` mutations.
- [ ] Add `ui.screenshot {panel?, id?}` cropping so agents can capture a single workspace panel or DOM element rectangle without post-cropping.
- [ ] Implement macOS (`WKWebView::takeSnapshotWithConfiguration`) and Windows (`ICoreWebView2::CapturePreview`) backends in `apps/magic/src/capture.rs` (currently Linux `WebKitGTK` only).
- [ ] Add `render.frame {base64: true, max_side?}` inline downscaled PNG encoding alongside file-path output.
- [ ] Add `ui.viewer.locate` / `ui.viewer.hit` stage coordinate mapping in Magic's preview viewport (`EffectCraft` viewer parity).
- [ ] Add `ui.timeline.locate` / `ui.timeline.hit` time-and-track coordinate mapping in Magic's timeline strip (`FilmCraft` / `EffectCraft` timeline parity).
- [ ] Include real-time L/R peak and RMS dBFS audio meter readings in `ui.playback {action: "status"}` (`FilmCraft` playback meter parity).
- [ ] Opt out of macOS App Nap while `--control` is active and support non-activating background window bring-forward during live watching.
- [ ] Expose read-only MCP resources (`magic://document`, `magic://storyboard`, `magic://commands`) in `crates/mcp` alongside MCP tools.

## 2. 3D Character Pipeline (`character.model3d.*` & `TextureCraft`)

Follow-up implementation steps for `crates/engine/src/cmd/character/model3d.rs` (replacing the cut-off legacy 3D prototype with native ArtCraft CLI delegation).

- [ ] Verify exact `texturecraft-cli` mesh/UV/PBR inspection subcommands on the host and replace the fallback `.glb` JSON header check in `character.model3d.rig` (`// TODO:` in `crates/engine/src/cmd/character/model3d.rs`).
- [ ] Wire optional external image-to-3D `.glb` generation provider (or local `TextureCraft` asset import workflow) into `character.model3d.breakout` when only a 2D turnaround sheet is present (`// TODO:` in `crates/engine/src/cmd/character/model3d.rs`).
- [ ] Map `.anim.json` skeletal joint tracks directly onto `EffectCraft` `advanced3D` `.glb` node transforms once `effectcraft-cli` exposes per-bone glTF animation channel overrides (`// TODO:` in `crates/engine/src/cmd/character/model3d.rs`).
- [ ] Add interactive 3D orbit/camera preset controls in `AnimationView.tsx` driving `EffectCraft` 3D camera layers (`layer.newCamera`).

## 3. Sibling ArtCraft CLI Host Verification (`// TODO:` Hooks)

Targeted host CLI checks flagged with `// TODO:` comments in `crates/engine` so zero commands rely on unverified syntax.

- [ ] Confirm exact `vectorcraft-cli` flat PNG export command (`export` vs `document.export`) against `vectorcraft-cli commands` in `crates/engine/src/cmd/storyboard/crud.rs` (`resolve_flat_image_and_ref`).
- [ ] Verify headless `anicraft-cli` sprite-sheet and frame-sequence CLI flags across new `AniCraft` pre-alpha builds while keeping `CraftTarget::Ani.allows_live_ui() == false` locked until `AniCraft` reaches beta stability.
- [ ] Add automated integration smoke tests in `crates/engine/tests/` that run skipped-unless-installed checks against real `designcraft-cli`, `photocraft-cli`, `vectorcraft-cli`, `effectcraft-cli`, and `filmcraft-cli` binaries.

## 4. Audio Transcription, Lip-Sync & Music Timing

Enhancements to song intake (`audio.analyze`, `workshop.import_transcript`) and `EffectCraft` facial puppet lip-sync (`character.puppet.lipsync`).

- [ ] Add sub-word phoneme / viseme timing extraction (`AA`, `EE`, `OH`, `OO`, `MBP`, `FV`, `WQ`, `rest`) directly from `craft-audio` / Whisper word timestamps so rapid multi-syllable words animate crisp jaw and lip shapes without manual curve tweaking.
- [ ] Support multi-character duet vocal stems (`A1` lead song + isolated character vocal stems) so two rigged characters on `V2` lip-sync independently.
- [ ] Add beat-snapped scene cut and action-cue quantization (`1/4`, `1/2`, `1 bar`) when seeding `FilmCraft` markers from `timing-reference.json`.

## 5. Storyboard (`DesignCraft`) & Bulk Generation Workflow

Enhancements to `storyboard.designcraft`, readiness gating, and reference-locked image generation.

- [ ] Add side-by-side visual diff preview in `StoryboardView` before committing `storyboard.bulk_generate` scene renders into `.designcraft` frames.
- [ ] Support custom per-project `.designcraft` trim sizes (`16:9` 1080p/4K widescreen, `9:16` vertical shorts, `1:1` booklet companion) in `storyboard.new`.
- [ ] Export a printable lyric-and-action booklet PDF preset inspired by children's songbook liner notes directly from `storyboard.export`.

## 6. Packaging, Release Signing & `craft-wiki` Publishing

Distribution and community documentation milestones using [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions) and [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).

- [ ] Configure macOS Developer ID + notarization secrets and Windows Authenticode signing secrets in GitHub repository settings for `.github/workflows/release.yml` (`storytold/craft-common-actions/release.yml@main`).
- [ ] Verify Linux `.deb`, `.rpm`, and Flatpak/AppStream metadata (`packaging/linux/io.github.jessenaiman.magic.xml`) with `packaging-lint.yml`.
- [ ] Submit `docs/wiki/magic/*.md` and `magic` app entry to [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki) once the repository is switched to public under the Magic Studio Source-Available Non-Resale License.
