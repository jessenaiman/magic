<p align="center">
  <img src="docs/brand/artcraft-logo.svg" alt="ArtCraft" width="72" height="72" />
</p>

<h1 align="center">Magic Studio</h1>

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

<p align="center">
  <b>From a recorded song to a finished multi-track video — in one self-contained <code>~/Magic/&lt;slug&gt;/</code> project folder.</b><br/>
  Pure-Rust desktop coordinator (<a href="https://github.com/storytold/egui">egui / eframe</a> + <a href="https://github.com/storytold/craft-libs/blob/main/README.md">storytold/craft-libs</a>) driving host
  <a href="https://github.com/storytold/designcraft/blob/main/README.md">DesignCraft</a>,
  <a href="https://github.com/storytold/photocraft/blob/main/README.md">PhotoCraft</a>,
  <a href="https://github.com/storytold/vectorcraft/blob/main/README.md">VectorCraft</a>,
  <a href="https://github.com/storytold/effectcraft/blob/main/README.md">EffectCraft</a>, and
  <a href="https://github.com/storytold/filmcraft/blob/main/README.md">FilmCraft</a> over <code>--control</code> &amp; CLI/MCP.
</p>

## Overview & Upstream `README.md` Navigation Rule

**Magic Studio** (`magic`, `magic-cli`, `magic-tui`) orchestrates music-video, lyric-video, and animated storytelling production across the [ArtCraft](https://getartcraft.com) desktop suite while keeping every song project self-contained inside `~/Magic/<slug>/` (`audio/`, `assets/`, `effects/`, `film/`, `exports/`, `<slug>.magic`, and `storyboard.designcraft`).

> **Rule for Contributors & Agents:** Always read the **actual words** of this `README.md` first, then navigate directly to the maintained upstream `storytold/*` `README.md` for any shared library or host application before changing code (never infer contracts from paraphrased notes):
> - Shared Rust libraries (`rev = "<commit>"` in `Cargo.toml`): **[`storytold/craft-libs/README.md`](https://github.com/storytold/craft-libs/blob/main/README.md)**
> - Shared AI runtime: **[`storytold/craft-ai-integrations/README.md`](https://github.com/storytold/craft-ai-integrations/blob/main/README.md)**
> - Host Craft apps: **[`DesignCraft` `README.md`](https://github.com/storytold/designcraft/blob/main/README.md)** · **[`PhotoCraft` `README.md`](https://github.com/storytold/photocraft/blob/main/README.md)** · **[`VectorCraft` `README.md`](https://github.com/storytold/vectorcraft/blob/main/README.md)** · **[`EffectCraft` `README.md`](https://github.com/storytold/effectcraft/blob/main/README.md)** · **[`FilmCraft` `README.md`](https://github.com/storytold/filmcraft/blob/main/README.md)**
> - Contribution & CI standards: **[`docs/house-rules-core.md`](docs/house-rules-core.md)** ([`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/README.md)) · **[`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions/blob/main/README.md)**
> - Next desktop build & PR handoff (`magic-fixes-9` -> `main`): **[`HANDOFF-magic-fixes-9.md`](HANDOFF-magic-fixes-9.md)**

## The 10-Step Magic Studio Workflow

| Step | Stage & Goal | What Happens on Disk & in Host Craft Apps | Where to Explore Code |
| :---: | :--- | :--- | :--- |
| **1** | **Upload Audio** | Copies the recording atomically into `~/Magic/<slug>/audio/`, inspects sample rate/duration/peaks with `craft-audio` + `craft-media`, and writes `timing-reference.json`. | [`cmd/project/new_project.rs`](crates/engine/src/cmd/project/new_project.rs), [`cmd/project/transcript.rs`](crates/engine/src/cmd/project/transcript.rs), [`cmd/song/waveform.rs`](crates/engine/src/cmd/song/waveform.rs) |
| **2** | **Create a FilmCraft Project** | Initializes `~/Magic/<slug>/film/<slug>.fcproj` (`schema_version: 17`) on `craft-timeline` ticks (`254,016,000,000` ticks/s) with `A1` audio, `C1` captions, and `V1`–`V4` tracks for [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/README.md). | [`cmd/film/assembly.rs`](crates/engine/src/cmd/film/assembly.rs), [`cmd/film/fcproj_doc.rs`](crates/engine/src/cmd/film/fcproj_doc.rs), [`crates/util/src/time.rs`](crates/util/src/time.rs) |
| **3** | **Start a Modifiable DesignCraft Storyboard** | Seeds `~/Magic/<slug>/storyboard.designcraft` from `~/.local/share/magic/templates/storyboard.designcraft` (or custom `project.storyboard_template`) as a native [`DesignCraft`](https://github.com/storytold/designcraft/blob/main/README.md) ZIP package. | [`cmd/storyboard/mod.rs`](crates/engine/src/cmd/storyboard/mod.rs), [`cmd/storyboard/crud.rs`](crates/engine/src/cmd/storyboard/crud.rs), [`views/storyboard.rs`](crates/ui-egui/src/views/storyboard.rs) |
| **4** | **Build the Complete Storyboard** | Updates unlocked story slots and `assets/*` directly inside `storyboard.designcraft` (`storyboard.set`) or edits spreads live in `designcraft --control 7979`, tracked by `storyboard.check`. | [`cmd/storyboard/crud.rs`](crates/engine/src/cmd/storyboard/crud.rs), [`cmd/storyboard/workflow.rs`](crates/engine/src/cmd/storyboard/workflow.rs) |
| **5** | **Build the First Draft with an Agent** | Runs `magic-cli mcp` + sibling `<app>-cli mcp` + opt-in [`craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations/blob/main/README.md) (`storyboard.bulk_generate`, `video.assemble`). | [`crates/mcp/src/server.rs`](crates/mcp/src/server.rs), [`crates/engine/src/craft_ai.rs`](crates/engine/src/craft_ai.rs) |
| **6** | **Edit Animation (`V2`)** | Isolates cutouts in [`PhotoCraft`](https://github.com/storytold/photocraft/blob/main/README.md), traces contours in [`VectorCraft`](https://github.com/storytold/vectorcraft/blob/main/README.md), validates 3D rigs via `craft-mesh`, and authors `V2` `.ecproj` clips in [`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/README.md). | [`cmd/character/cut.rs`](crates/engine/src/cmd/character/cut.rs), [`cmd/character/mesh_io.rs`](crates/engine/src/cmd/character/mesh_io.rs), [`views/animation.rs`](crates/ui-egui/src/views/animation.rs) |
| **7** | **Edit Lyrics (`V3` + `C1`)** | Styles word-timed typography in `~/Magic/<slug>/effects/lyrics.ecproj` (`V3`) with `craft-font-catalog` + `FontsPanel` and syncs `C1` captions via `craft-captions`. | [`cmd/words/mod.rs`](crates/engine/src/cmd/words/mod.rs), [`cmd/words/native_motion.rs`](crates/engine/src/cmd/words/native_motion.rs), [`views/lyrics.rs`](crates/ui-egui/src/views/lyrics.rs) |
| **8** | **Edit Motion Design (`V4`)** | Authors `craft-drawingml` preset geometries, text warps, and `VectorCraft` SVGs in `~/Magic/<slug>/effects/motion.ecproj` on track `V4`. | [`cmd/motion/mod.rs`](crates/engine/src/cmd/motion/mod.rs), [`views/motion.rs`](crates/ui-egui/src/views/motion.rs) |
| **9** | **Keep Every File Self-Contained** | Stores all project files under `~/Magic/<slug>/` via `craft-atomic` and `craft-docpath` (`project.migrate` writes `original-project.zip` via `craft-zip` only when importing legacy external folders). | [`crates/engine/src/workspace.rs`](crates/engine/src/workspace.rs), [`cmd/project/archive.rs`](crates/engine/src/cmd/project/archive.rs), [`views/project.rs`](crates/ui-egui/src/views/project.rs) |
| **10** | **Export and Publish** | Reviews scopes, color, `rmsDbfs`/`peakDbfs` meters, and `sync_warnings` markers in `05 Studio`, renders masters into `~/Magic/<slug>/exports/` via `FilmCraft`, and exports `OTIO`/`FCPXML`/`EDL`/`ALE`. | [`cmd/film/finishing.rs`](crates/engine/src/cmd/film/finishing.rs), [`cmd/film/timeline_interchange.rs`](crates/engine/src/cmd/film/timeline_interchange.rs), [`views/studio.rs`](crates/ui-egui/src/views/studio.rs) |

### CLI Walkthrough

```sh
# 1–3. Create ~/Magic/lantern-harbor/ with audio/, assets/, effects/, film/, exports/, and storyboard.designcraft
magic-cli new --audio ./song.wav --name "Lantern Harbor"   --style "Cut-paper gouache" --world "Moonlit pier" --character "Fox conductor" --motion "Starlight arcs"

# 4–5. Verify storyboard readiness and assemble the initial FilmCraft sequence
magic-cli --project ~/Magic/lantern-harbor/lantern-harbor.magic exec storyboard.check
magic-cli --project ~/Magic/lantern-harbor/lantern-harbor.magic exec video.assemble

# 6–8. Author V2 character rig, V3 effects/lyrics.ecproj, and V4 effects/motion.ecproj
magic-cli --project ~/Magic/lantern-harbor/lantern-harbor.magic exec character.new name="Fox" --save
magic-cli --project ~/Magic/lantern-harbor/lantern-harbor.magic exec words.letter preset=chorus_pop --save
magic-cli --project ~/Magic/lantern-harbor/lantern-harbor.magic exec motion.save --save

# 9–10. Save self-contained project and export master video into ~/Magic/lantern-harbor/exports/
magic-cli --project ~/Magic/lantern-harbor/lantern-harbor.magic exec file.save
magic-cli --project ~/Magic/lantern-harbor/lantern-harbor.magic export ~/Magic/lantern-harbor/exports/lantern-harbor-master.mp4 --codec h264
```

## Required Host Craft Applications

| Host App | Default `--control` Socket | CLI & MCP Entrypoint | Upstream `README.md` |
| :--- | :--- | :--- | :--- |
| **DesignCraft** | `designcraft --control 7979` | `designcraft-cli` / `designcraft-cli mcp` (`--automation-write-root` for headless disk writes) | [`storytold/designcraft/README.md`](https://github.com/storytold/designcraft/blob/main/README.md) |
| **PhotoCraft** | `photocraft --control 7878` | `photocraft-cli` / `photocraft-cli mcp` | [`storytold/photocraft/README.md`](https://github.com/storytold/photocraft/blob/main/README.md) |
| **VectorCraft** | `vectorcraft --control 7980` | `vectorcraft-cli` / `vectorcraft-cli mcp` | [`storytold/vectorcraft/README.md`](https://github.com/storytold/vectorcraft/blob/main/README.md) |
| **EffectCraft** | `effectcraft --control 9877` | `effectcraft-cli` / `effectcraft-cli mcp` | [`storytold/effectcraft/README.md`](https://github.com/storytold/effectcraft/blob/main/README.md) |
| **FilmCraft** | `filmcraft --control 9876` (`FILMCRAFT_CONTROL_TOKEN`) | `filmcraft-cli` / `filmcraft-cli mcp` | [`storytold/filmcraft/README.md`](https://github.com/storytold/filmcraft/blob/main/README.md) |

## Workspace Crate Map

| Crate / App | Layer | Key Entrypoints | Local Guide |
| :--- | :---: | :--- | :--- |
| **`magic-util`** | `L4` | [`crates/util/src/lib.rs`](crates/util/src/lib.rs), [`src/time.rs`](crates/util/src/time.rs), [`src/media.rs`](crates/util/src/media.rs) | [`crates/util/README.md`](crates/util/README.md) |
| **`magic-scribbles`** | `L4` | [`crates/scribbles/src/lib.rs`](crates/scribbles/src/lib.rs), [`src/sheet.rs`](crates/scribbles/src/sheet.rs) | [`crates/scribbles/README.md`](crates/scribbles/README.md) |
| **`magic-engine`** | `L5` | [`crates/engine/src/lib.rs`](crates/engine/src/lib.rs), [`src/workspace.rs`](crates/engine/src/workspace.rs), [`src/craft_api.rs`](crates/engine/src/craft_api.rs), [`src/cmd/`](crates/engine/src/cmd.rs) | [`crates/engine/README.md`](crates/engine/README.md) |
| **`magic-mcp`** | `L6` | [`crates/mcp/src/lib.rs`](crates/mcp/src/lib.rs), [`src/server.rs`](crates/mcp/src/server.rs) | [`crates/mcp/README.md`](crates/mcp/README.md) |
| **`magic-ui-egui`** | `L6` | [`crates/ui-egui/src/docking.rs`](crates/ui-egui/src/docking.rs), [`src/shell.rs`](crates/ui-egui/src/shell.rs), [`src/views/`](crates/ui-egui/src/views/mod.rs) | [`crates/ui-egui/README.md`](crates/ui-egui/README.md) |
| **`magic`** | `L7` | [`apps/magic/src/main.rs`](apps/magic/src/main.rs), [`src/control.rs`](apps/magic/src/control.rs), [`src/capture.rs`](apps/magic/src/capture.rs) | [`apps/magic/README.md`](apps/magic/README.md) |
| **`magic-cli`** | `L7` | [`apps/magic-cli/src/main.rs`](apps/magic-cli/src/main.rs), [`src/subcommands.rs`](apps/magic-cli/src/subcommands.rs) | [`apps/magic-cli/README.md`](apps/magic-cli/README.md) |
| **`magic-tui`** | `L7` | [`apps/magic-tui/src/main.rs`](apps/magic-tui/src/main.rs), [`src/snapshot.rs`](apps/magic-tui/src/snapshot.rs), [`src/ui.rs`](apps/magic-tui/src/ui.rs) | [`apps/magic-tui/README.md`](apps/magic-tui/README.md) |
| **`xtask`** | `Build` | [`xtask/src/main.rs`](xtask/src/main.rs), [`src/ci.rs`](xtask/src/ci.rs), [`src/policy.rs`](xtask/src/policy.rs) | [`xtask/README.md`](xtask/README.md) |
