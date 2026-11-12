<p align="center">
<<<<<<< HEAD
  <img src="assets/app-icon/icon.png" width="112" height="112" alt="Magic Studio icon">
=======
  <img src="docs/brand/artcraft-logo.svg" alt="ArtCraft" width="72" height="72" />
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
</p>

<h1 align="center">Magic Studio</h1>

<p align="center">
<<<<<<< HEAD
  <b>Song-driven storyboard, character puppet-rig, and lyric timing coordinator built in pure Rust on <code>storytold/craft-libs</code>.</b>
</p>

<p align="center">
  Plan music and educational videos from a recorded audio track, manage printable PDF or slide storyboards,
  prepare anatomical cutout rigs and timed lyric tracks, and share clean project files across the
  <a href="https://github.com/storytold">ArtCraft</a> creative suite. Every UI action is also an engine command
  that <code>magic-cli</code> and AI agents (through MCP) can drive.
</p>

<p align="center">
  <img alt="Status: early development (pre-alpha)" src="https://img.shields.io/badge/status-early%20development%20(pre--alpha)-f26b1d?style=flat-square">
  <img alt="Written in Rust" src="https://img.shields.io/badge/written%20in-Rust-b7410e?style=flat-square">
  <img alt="Platforms" src="https://img.shields.io/badge/platforms-macOS%20%C2%B7%20Windows%20%C2%B7%20Linux%20%C2%B7%20BSD-555?style=flat-square">
  <a href="LICENSE"><img alt="License: Source-Available Non-Resale" src="https://img.shields.io/badge/license-Source--Available%20Non--Resale-35e1c0?style=flat-square"></a>
</p>

> [!NOTE]
> **Pre-Alpha Status & Contributor Call:** Magic Studio is in **pre-alpha (`early development`)**. The legacy cross-app `CraftHub` / `bridge.*` RPC wrapper layer has been stripped so the workspace follows standard ArtCraft conventions (`craft-libs` file envelopes on disk + a single same-binary `--control` channel). Contributors are welcome to help test workflows, refine stage views, and fix pre-alpha bugs.

## Standard ArtCraft Architecture (Zero Cross-App Proxying)

Magic Studio (`magic`, `magic-cli`, `magic-mcp`, `magic-tui`) is built on `eframe` + `egui` 0.37, [`storytold/craft-libs`](https://github.com/storytold/craft-libs), and [`storytold/craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations) with zero Tauri, Electron, or webview layers:

1. **Shared File Envelopes via `craft-libs` (No Bundled Sibling Binaries Required)**:
   Magic inspects and writes project assets directly on disk using `craft-doc`, `craft-ecproj-format`, `craft-pcraft-format`, `craft-vectorcraft-format`, `craft-audio`, `craft-media`, `craft-atomic`, and `image`. Opening an asset in [`FilmCraft`](https://github.com/storytold/filmcraft), [`EffectCraft`](https://github.com/storytold/effectcraft), [`DeckCraft`](https://github.com/storytold/deckcraft), [`DesignCraft`](https://github.com/storytold/designcraft), [`PhotoCraft`](https://github.com/storytold/photocraft), or [`VectorCraft`](https://github.com/storytold/vectorcraft) uses standard OS file handoff (`workspace.open`) + atomic save watching (`link_watch`), matching PhotoCraft's *Edit Contents* pattern.
2. **Same-Binary `--control` & MCP (`127.0.0.1:9875`)**:
   `magic --control 9875` exposes a standard loopback JSON-lines control channel exclusively for `magic-cli` and `magic-cli mcp --connect 127.0.0.1:9875`. AI agents drive sibling ArtCraft apps directly through each installed app's own `<app>-cli mcp` server rather than routing commands through Magic.

## Quickstart (`magic`, `magic-cli`, and MCP)

```sh
# Run workspace architecture, formatting, Clippy, and test checks:
cargo xtask ci

# Launch the desktop app with standard loopback control enabled:
cargo run -p magic -- --control 9875 &

# Attach the MCP server to the live window (or omit --connect for headless mode):
claude mcp add magic -- magic-cli mcp
claude mcp add magic-app -- magic-cli mcp --connect 127.0.0.1:9875

# Quick loopback JSON-lines smoke test (no token required by default):
printf '{"id":1,"method":"engine.commands","params":{}}\n' | nc 127.0.0.1 9875

# Create a project and inspect storyboard state:
magic-cli new --root ~/Videos/Magic --name "My Song" --audio song.wav --wizard
magic-cli --connect 127.0.0.1:9875 storyboard check
magic-cli app --port 9875 ui.inspect
```

## Workspace Layout

| Crate / App | Layer | Responsibility |
| :--- | :---: | :--- |
| `crates/util` (`magic-util`) | L4 | Exact `Tick` timing, cubic-Bézier `Easing`, and zero-subprocess `craft-doc` / `craft-media` / `craft-audio` inspection |
| `crates/scribbles` (`magic-scribbles`) | L4 | Deterministic prompt templates and sprite/parts grid geometry |
| `crates/engine` (`magic-engine`) | L5 | Headless [`Session`], command registry, pure-Rust `native_media` cutouts/SVG contours, and `.magic` project state |
| `crates/mcp` (`magic-mcp`) | L6 | Standard ArtCraft MCP server (`command_list`, `command_run`, `command_batch`, `doc_inspect`, `ui_*`) |
| `crates/ui-egui` (`magic-ui-egui`) | L6 | Pure-Rust `egui` 0.37 views (`Home`, `Project`, `Storyboard`, `Animations`, `Lyrics`, `Motion`, `Studio`) |
| `apps/magic` | L7 | Native `eframe` + `wgpu` desktop binary + `--control` JSON-lines listener |
| `apps/magic-cli` | L7 | Headless / `--connect` CLI and `magic-mcp` stdio entrypoint |
| `apps/magic-tui` | L7 | **Backstage** terminal console for headless or live `--control 9875` sessions |
| `xtask` | L7 | ArtCraft `ci`, `layers`, `assets` (`ATTRIBUTION.md`), and bounded `test` gates |

## Documentation

- **[Control Protocol (`docs/control-protocol.md`)](docs/control-protocol.md)** — Loopback JSON-lines `--control` specification (`127.0.0.1:9875`).
- **[MCP Server (`docs/mcp.md`)](docs/mcp.md)** — `magic-cli mcp` / `.mcp.json` configuration and `--automation-read-root` / `--automation-write-root` confinement.
- **[Architecture (`docs/architecture.md`)](docs/architecture.md)** — Crate layer DAG and `craft-libs` file boundaries.
- **[Agent Instructions (`AGENTS.md`)](AGENTS.md)** — House rules and standard ArtCraft MCP workflow.

## License

Copyright (c) 2026 Jesse Naiman. Distributed under the **[Magic Studio Source-Available Non-Resale License](LICENSE)** (brand assets governed by [`docs/brand/LICENSE-brand.txt`](docs/brand/LICENSE-brand.txt)).
=======
  <b>From a recorded song to a finished multi-track video — in one self-contained project folder.</b><br/>
  Pure-Rust desktop coordinator (<a href="https://github.com/storytold/egui">egui / eframe</a> + <a href="https://github.com/storytold/craft-libs">craft_ui::docking</a>) driving host
  <a href="https://github.com/storytold/designcraft">DesignCraft</a>,
  <a href="https://github.com/storytold/photocraft">PhotoCraft</a>,
  <a href="https://github.com/storytold/vectorcraft">VectorCraft</a>,
  <a href="https://github.com/storytold/effectcraft">EffectCraft</a>, and
  <a href="https://github.com/storytold/filmcraft">FilmCraft</a> over <code>--control</code> &amp; CLI/MCP.
</p>

## Overview

**Magic Studio** (`magic`, `magic-cli`, `magic-tui`) orchestrates music-video, lyric-video, and animated storytelling production across the [ArtCraft](https://getartcraft.com) desktop suite. Instead of duplicating full page-layout, raster, vector, compositing, or NLE editors inside a monolith, Magic Studio coordinates the host machine's installed Craft applications through their live `--control` JSON-lines sockets and `<app>-cli` / MCP tools while keeping every audio track, `DesignCraft` storyboard, character rig, kinetic lyric composition, motion overlay, and `FilmCraft` edit inside a single portable `<project>/` folder.

## The 10-Step Magic Studio Workflow

Every song project follows one clear 10-step path from raw audio to published master — in the desktop UI (`magic`), from the CLI (`magic-cli`), or with an agent (`magic-cli mcp`):

| Step | Stage & Goal | What Happens on Disk & in Host Craft Apps | Where & Why to Explore the Code |
| :---: | :--- | :--- | :--- |
| **1** | **Upload Audio** | Copies the recording atomically into `<project>/audio/`, inspects sample rate/duration/peaks with `craft-audio` + `craft-media`, and extracts word-level timing (`timing-reference.json`). | [`cmd/project/new_project.rs`](crates/engine/src/cmd/project/new_project.rs) (atomic audio intake), [`cmd/project/transcript.rs`](crates/engine/src/cmd/project/transcript.rs) (word timing), [`cmd/song/waveform.rs`](crates/engine/src/cmd/song/waveform.rs) (real PCM envelope) |
| **2** | **Create a FilmCraft Project** | Initializes `<project>/film/<song>.fcproj` on rational `craft-timeline` ticks (`254,016,000,000` ticks/s) with `A1` song audio, `C1` captions, and `V1`–`V4` tracks ready for host [`FilmCraft`](https://github.com/storytold/filmcraft). | [`cmd/film/assembly.rs`](crates/engine/src/cmd/film/assembly.rs) (`ensure_song_film`), [`cmd/film/fcproj_doc.rs`](crates/engine/src/cmd/film/fcproj_doc.rs) (`V1`–`V4`/`C1`/`A1` schema), [`crates/util/src/time.rs`](crates/util/src/time.rs) (`craft-timeline` `Tick`) |
| **3** | **Start a Modifiable DesignCraft Storyboard** | Creates `<project>/storyboard.designcraft` as a complete, user-modifiable [`DesignCraft`](https://github.com/storytold/designcraft) design document (`612 × 792 pt` spreads across `Cover`, `Backgrounds`, `Characters`, `Scenes`, `Assets`, `Final`). | [`cmd/storyboard/mod.rs`](crates/engine/src/cmd/storyboard/mod.rs) (`storyboard.new` & `.designcraft` IDML/ZIP package), [`views/storyboard.rs`](crates/ui-egui/src/views/storyboard.rs) (`#view-storyboard` spread & single-page canvas) |
| **4** | **Build the Complete Storyboard** | Fill visual direction, world plates, cast turnarounds, and scene panels inline or directly inside host `DesignCraft` (`designcraft --control 7979`), tracked live by `storyboard.check` red/green readiness lights. | [`cmd/storyboard/crud.rs`](crates/engine/src/cmd/storyboard/crud.rs) (`storyboard.set`, `storyboard.pages`), [`cmd/storyboard/workflow.rs`](crates/engine/src/cmd/storyboard/workflow.rs) (`storyboard.check`, `storyboard.wizard`, `PhotoCraft` LUT grading), [`views/storyboard_page.rs`](crates/ui-egui/src/views/storyboard_page.rs) |
| **5** | **Build the First Draft of a Video with an Agent** | An MCP agent (`magic-cli mcp` + sibling `<app>-cli mcp` + opt-in [`craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations)) generates missing scene plates (`storyboard.bulk_generate`), links rough timing on `V1`–`V4`, and assembles a playable first draft (`video.assemble`). | [`crates/mcp/src/server.rs`](crates/mcp/src/server.rs) (MCP tools & batch runner), [`crates/engine/src/craft_ai.rs`](crates/engine/src/craft_ai.rs) (`SharedStore` & `ConsentLedger`), [`cmd/film/assembly.rs`](crates/engine/src/cmd/film/assembly.rs) (`cmd_video_assemble`) |
| **6** | **Edit Animation** | Isolate character parts in [`PhotoCraft`](https://github.com/storytold/photocraft), trace contours in [`VectorCraft`](https://github.com/storytold/vectorcraft), inspect/convert 3D rigs via `craft-mesh`, and author `V2` character `.ecproj` clips in [`EffectCraft`](https://github.com/storytold/effectcraft). | [`cmd/character/cut.rs`](crates/engine/src/cmd/character/cut.rs) (cutouts), [`cmd/character/mesh_io.rs`](crates/engine/src/cmd/character/mesh_io.rs) (`craft-mesh` 3D), [`animation_model.rs`](crates/engine/src/animation_model.rs), [`cmd/character/`](crates/engine/src/cmd/character/mod.rs) (`V2` rigs), [`views/animation.rs`](crates/ui-egui/src/views/animation.rs) |
| **7** | **Edit Lyrics** | Style word-timed kinetic typography in `<project>/lettering/lyrics.ecproj` (`V3`) with `craft-font-catalog` + `FontsPanel` and sync broadcast captions (`SRT`, `WebVTT`, `TTML`, `CEA-608/708`, `EBU-STL`) onto `C1` via `craft-captions`. | [`cmd/words/native_motion.rs`](crates/engine/src/cmd/words/native_motion.rs) (`lyrics.ecproj`), [`cmd/song/`](crates/engine/src/cmd/song/mod.rs) (word timing), [`views/lyrics.rs`](crates/ui-egui/src/views/lyrics.rs) & [`fonts_ui.rs`](crates/ui-egui/src/fonts_ui.rs) |
| **8** | **Edit Motion Design** | Author beat-synced callouts, `craft-drawingml` preset geometries & text warps (`a:prstGeom`, `a:prstTxWarp`), and `VectorCraft` SVG paths in `<project>/motion/actions.ecproj` on track `V4`. | [`cmd/motion/mod.rs`](crates/engine/src/cmd/motion/mod.rs) (`craft-drawingml` presets & warps), [`crates/engine/src/craft_effects.rs`](crates/engine/src/craft_effects.rs) (EffectCraft effect catalog), [`views/motion.rs`](crates/ui-egui/src/views/motion.rs) |
| **9** | **Save All Files in One Location** | Every `.magic`, `.designcraft`, `.pcraft`, `.vectorcraft`, `.ecproj`, and `.fcproj` file lives under `<project>/` with `craft-atomic` crash-safe writes, `.history/*.autosave` journals, and one-click `.zip` folder archiving (`project.archive` via `craft-zip`). | [`cmd/project/mod.rs`](crates/engine/src/cmd/project/mod.rs) (`file.save` / relative paths), [`cmd/project/archive.rs`](crates/engine/src/cmd/project/archive.rs) (`craft-zip` compressed folder archives), [`views/project.rs`](crates/ui-egui/src/views/project.rs) |
| **10** | **Export and Publish** | Review scopes, color, audio buses (`rmsDbfs` / `peakDbfs`), and markers in `05 Studio`, render the master video (`MP4`/`MOV`/`ProRes`/`AV1`/`WebM`) through host `FilmCraft`, and export `OTIO`/`FCPXML`/`EDL` + sidecar captions. | [`cmd/film/finishing.rs`](crates/engine/src/cmd/film/finishing.rs) (`video.export` & finishing), [`cmd/film/timeline_interchange.rs`](crates/engine/src/cmd/film/timeline_interchange.rs) (`craft-timeline` NLE export), [`views/studio.rs`](crates/ui-egui/src/views/studio.rs) |

### CLI Walkthrough of the 10 Steps

```sh
# 1–3. Upload audio, create FilmCraft project (`film/<slug>.fcproj`), and start `storyboard.designcraft`
magic-cli new --audio ./song.wav --name "Lantern Harbor" \
  --style "Cut-paper gouache" --world "Moonlit pier" --character "Fox conductor" --motion "Starlight arcs"

# 4. Complete and verify the DesignCraft storyboard spreads
magic-cli --project ~/Magic/projects/lantern-harbor/lantern-harbor.magic exec storyboard.check
magic-cli --project ~/Magic/projects/lantern-harbor/lantern-harbor.magic exec storyboard.open app=designcraft

# 5. Build the first draft of the video (or drive via `magic-cli mcp`)
magic-cli --project ~/Magic/projects/lantern-harbor/lantern-harbor.magic exec storyboard.bulk_generate
magic-cli --project ~/Magic/projects/lantern-harbor/lantern-harbor.magic exec video.assemble

# 6–8. Edit animation (V2), lyrics (V3 + C1 captions), and motion design (V4)
magic-cli --project ~/Magic/projects/lantern-harbor/lantern-harbor.magic exec character.new name="Fox" --save
magic-cli --project ~/Magic/projects/lantern-harbor/lantern-harbor.magic exec words.letter preset=chorus_pop --save
magic-cli --project ~/Magic/projects/lantern-harbor/lantern-harbor.magic exec motion.save --save

# 9. Save everything in `<project>/` and optionally archive older projects into a verified `.zip`
magic-cli --project ~/Magic/projects/lantern-harbor/lantern-harbor.magic exec file.save
magic-cli archive ~/Magic/projects/lantern-harbor --out ~/Magic/archives/lantern-harbor.zip

# 10. Export master video via host FilmCraft + sidecar broadcast captions
magic-cli --project ~/Magic/projects/lantern-harbor/lantern-harbor.magic export ./lantern-harbor-master.mp4 --codec h264
```

## Required Host Craft Applications

Magic Studio requires the five ArtCraft creative applications on the host machine and communicates with them over their `--control` TCP sockets and `<app>-cli` binaries (verified any time via `magic-cli exec app.doctor` or **Help ▸ ArtCraft Suite Doctor**):

| Host App | Default `--control` Socket | CLI & MCP Entrypoint | Purpose in Magic Studio |
| :--- | :--- | :--- | :--- |
| **[DesignCraft](https://github.com/storytold/designcraft)** | `designcraft --control 7979` | `designcraft-cli` / `designcraft-cli mcp` | Opens, edits, renders spreads, and packages `storyboard.designcraft` |
| **[PhotoCraft](https://github.com/storytold/photocraft)** | `photocraft --control 7878` | `photocraft-cli` / `photocraft-cli mcp` | Subject isolation, character cutout sheets, and batch LUT grading (`.pcraft`) |
| **[VectorCraft](https://github.com/storytold/vectorcraft)** | `vectorcraft --control 7980` | `vectorcraft-cli` / `vectorcraft-cli mcp` | Silhouette contour tracing and vector motion artwork (`.vectorcraft`, `.svg`) |
| **[EffectCraft](https://github.com/storytold/effectcraft)** | `effectcraft --control 9877` | `effectcraft-cli` / `effectcraft-cli mcp` | Character rigs (`V2`), kinetic lyrics (`V3`), and motion design (`V4`) (`.ecproj`) |
| **[FilmCraft](https://github.com/storytold/filmcraft)** | `filmcraft --control 9876` | `filmcraft-cli` / `filmcraft-cli mcp` | Multi-track timeline assembly (`film/<song>.fcproj`) and final video encoding |

## Workspace Crate Map — Where & Why to Explore

| Crate / App | Layer | Key Entrypoints | Why You Explore Here |
| :--- | :---: | :--- | :--- |
| **[`crates/util`](crates/util/README.md)** (`magic-util`) | `L4` | [`src/lib.rs`](crates/util/src/lib.rs), [`src/time.rs`](crates/util/src/time.rs), [`src/media.rs`](crates/util/src/media.rs), [`build.rs`](crates/util/build.rs) | Inspect `craft-timeline` `Tick`/`Timecode`, `craft-audio`/`craft-media` decoding, `craft-doc` `.pcraft`/`.vectorcraft`/`.ecproj` preview extraction, and optional `CRAFT_FONTS_DIR` / `CRAFT_I18N_DIR` build hooks. |
| **[`crates/scribbles`](crates/scribbles/README.md)** (`magic-scribbles`) | `L4` | [`src/lib.rs`](crates/scribbles/src/lib.rs), [`src/sheet.rs`](crates/scribbles/src/sheet.rs) | Inspect deterministic storyboard prompt templates and turnaround/pose sheet grid geometry. |
| **[`crates/engine`](crates/engine/README.md)** (`magic-engine`) | `L5` | [`src/lib.rs`](crates/engine/src/lib.rs), [`src/craft_api.rs`](crates/engine/src/craft_api.rs), [`src/extensions.rs`](crates/engine/src/extensions.rs), [`src/cmd/`](crates/engine/src/cmd.rs) | Explore headless `Session::execute`, host Craft `--control`/CLI delegation, `storyboard.designcraft`, `craft-zip` archiving, `craft-drawingml` motion, `craft-mesh` 3D, `craft-extensions`, and `FilmCraft` `.fcproj` assembly. |
| **[`crates/mcp`](crates/mcp/README.md)** (`magic-mcp`) | `L6` | [`src/lib.rs`](crates/mcp/src/lib.rs), [`src/server.rs`](crates/mcp/src/server.rs) | Explore the opt-in stdio JSON-RPC 2.0 MCP server (`--headless` in-process or `--connect 127.0.0.1:9875` over live window). |
| **[`crates/ui-egui`](crates/ui-egui/README.md)** (`magic-ui-egui`) | `L6` | [`src/docking.rs`](crates/ui-egui/src/docking.rs), [`src/icons.rs`](crates/ui-egui/src/icons.rs), [`src/shell.rs`](crates/ui-egui/src/shell.rs), [`src/views/`](crates/ui-egui/src/views/mod.rs) | Explore `craft_ui::docking` workspaces/panels/floating windows, custom `24x24` `currentColor` SVGs (`assets/icons/*.svg`), `FontsPanel`, and all seven stage views. |
| **[`apps/magic`](apps/magic/README.md)** (`magic`) | `L7` | [`src/main.rs`](apps/magic/src/main.rs), [`src/control.rs`](apps/magic/src/control.rs), [`src/audio_player.rs`](apps/magic/src/audio_player.rs) | Explore native `eframe` + `wgpu` window startup, OS-native floating dock windows, `cpal` audio playback with live dBFS meters, and the `--control 9875` JSON-lines server. |
| **[`apps/magic-cli`](apps/magic-cli/README.md)** (`magic-cli`) | `L7` | [`src/main.rs`](apps/magic-cli/src/main.rs), [`src/subcommands.rs`](apps/magic-cli/src/subcommands.rs), [`src/mcp_bin.rs`](apps/magic-cli/src/mcp_bin.rs) | Explore headless project automation, `magic-cli archive`, live `magic-cli app --port 9875`, `magic-cli mcp`, and sibling Craft signposts. |
| **[`apps/magic-tui`](apps/magic-tui/README.md)** (`magic-tui`) | `L7` | [`src/main.rs`](apps/magic-tui/src/main.rs) | Explore the `ratatui` terminal dashboard for headless or `--connect 127.0.0.1:9875` sessions. |
| **[`xtask`](xtask/README.md)** (`xtask`) | `Build` | [`src/main.rs`](xtask/src/main.rs), [`src/policy.rs`](xtask/src/policy.rs) | Explore `cargo xtask ci`: downward `L4 -> L7` DAG verification, clippy/test gates, and [`ATTRIBUTION.md`](ATTRIBUTION.md) asset checks. |

## Building & Optional Suite Data Repos

```sh
cargo build --release -p magic -p magic-cli

# Optional: embed full ArtCraft font families and locale catalogs at build time
# (per upstream READMEs, craft-fonts and craft-i18n are build-time env inputs, never Cargo deps)
CRAFT_FONTS_DIR=../craft-fonts CRAFT_I18N_DIR=../craft-i18n cargo build --release -p magic

# Run full workspace policy, layer, attribution, lint, and unit-test gates
cargo xtask ci
```

## Further Documentation

- **[`AGENTS.md`](AGENTS.md)** & **[`docs/house-rules-core.md`](docs/house-rules-core.md)** — Engineering invariants, host Craft app contracts, and upstream `storytold` dependency rules.
- **[`docs/README.md`](docs/README.md)** — Architecture ([`docs/architecture.md`](docs/architecture.md)), `--control 9875` protocol ([`docs/control-protocol.md`](docs/control-protocol.md)), MCP guide ([`docs/mcp.md`](docs/mcp.md)), and remaining roadmap items ([`docs/gaps.md`](docs/gaps.md)).
- **[`ATTRIBUTION.md`](ATTRIBUTION.md)** — Clean-room custom `assets/icons/*.svg` icons, Lucide MIT icons, OFL fonts, and ArtCraft brand notices.

## License

See [`LICENSE`](LICENSE) for repository licensing terms. The ArtCraft name and logos in `docs/brand/` are covered by [`docs/brand/LICENSE-brand.txt`](docs/brand/LICENSE-brand.txt).
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
