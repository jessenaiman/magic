<p align="center">
  <a href="https://getartcraft.com/">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/storytold/filmcraft/main/docs/brand/artcraft-logo-white.svg">
      <img alt="ArtCraft" src="https://raw.githubusercontent.com/storytold/filmcraft/main/docs/brand/artcraft-logo.svg" width="200">
    </picture>
  </a>
</p>

<p align="center">
  <img src="assets/app-icon/icon.png" width="128" height="128" alt="Magic Studio app icon">
</p>

<h1 align="center">Magic Studio</h1>

<p align="center">
  <b>Song-to-video production studio; creative storyboards, multi-tool character animation, kinetic lyrics and FilmCraft finishing, built in pure Rust.</b>
</p>

<p align="center">
  Turn a recorded song or an existing project into a finished music video, lyric video or animated short across
  <a href="https://github.com/storytold/deckcraft">DeckCraft</a>,
  <a href="https://github.com/storytold/effectcraft">EffectCraft</a> and
  <a href="https://github.com/storytold/filmcraft">FilmCraft</a>.
  Native on macOS, Windows and Linux. Every button and stage action is also a command that the CLI,
  a JSON control channel and AI agents (through MCP) can drive.
</p>

<p align="center">
  <a href="#license-and-credits"><img alt="License: Non-Resale" src="https://img.shields.io/badge/license-Non--Resale-8b5cf6?style=flat-square"></a>
  <img alt="Written in pure Rust" src="https://img.shields.io/badge/pure-Rust-6a3fd6?style=flat-square&logo=rust&logoColor=white">
  <img alt="Runs on macOS, Windows and Linux" src="https://img.shields.io/badge/runs%20on-macOS%20%7C%20Windows%20%7C%20Linux-8b5cf6?style=flat-square">
  <a href="ROADMAP.md"><img alt="Status: alpha" src="https://img.shields.io/badge/status-alpha-6a3fd6?style=flat-square"></a>
  <br>
  <a href="https://sonarcloud.io/project/overview?id=jessenaiman_magic"><img alt="Scanned by SonarCloud" src="https://img.shields.io/badge/Scanned_by-SonarCloud-4b9fd5?style=flat-square&logo=sonarqubecloud&logoColor=white"></a>
  <a href="https://coderabbit.ai"><img alt="Reviewed by CodeRabbit" src="https://img.shields.io/coderabbit/prs/github/jessenaiman/magic?style=flat-square&label=Reviewed%20by%20CodeRabbit&logo=coderabbit"></a>
</p>

<p align="center">
  <a href="https://discord.gg/artcraft"><img alt="Join the ArtCraft community on Discord" src="https://img.shields.io/badge/Join%20us%20on%20Discord-5865F2?style=for-the-badge&logo=discord&logoColor=white" height="40"></a>
</p>

<p align="center">
  <a href="https://github.com/jessenaiman/magic"><b>Magic Studio on GitHub</b></a> ·
  <a href="https://getartcraft.com/">ArtCraft</a> ·
  <a href="https://getartcraft.com/apps">All Crafting Apps</a>
</p>

> [!NOTE]
> **ArtCraft is a community of artists from all walks of life.** Digital, generative, music,
> games &mdash; if you make things, you're one of us. **[Come say hi on Discord](https://discord.gg/artcraft).**

<p align="center">
  <a href="#storyboard-and-scribbles">Storyboard</a> ·
  <a href="#open-multi-tool-animations">Animations</a> ·
  <a href="#live-editing-views-and-studio">Live Views &amp; Studio</a> ·
  <a href="#built-for-agents">Agents</a> ·
  <a href="#get-started">Get started</a> ·
  <a href="#the-crafting-apps">The Crafting Apps</a> ·
  <a href="#documentation">Docs</a> ·
  <a href="#license-and-credits">License</a>
</p>

<br>

Magic Studio (`magic`, `magic-cli`, `magic-tui`) is a song-driven production coordinator built on [`storytold/craft-libs`](https://github.com/storytold/craft-libs) and `eframe` / `egui` 0.37. Instead of locking artists into a closed format, every project folder holds real creative files — native [DeckCraft](https://github.com/storytold/deckcraft) storyboard decks, open 2D/3D animation renders, [EffectCraft](https://github.com/storytold/effectcraft) lyric and motion compositions, and a sample-accurate [FilmCraft](https://github.com/storytold/filmcraft) multi-track sequence (`254,016,000,000` ticks per second).

<br>

## Storyboard and Scribbles

- **Creative storyboards in DeckCraft (`01 Storyboard`).** Build decks with themes, layouts, shapes, pictures, speaker notes, section dividers and scene timing in native `.deckcraft` or PowerPoint `.pptx` files, present them full screen, and export production PDFs (slides, notes pages and handouts) through [`DeckCraft`](https://github.com/storytold/deckcraft) (`deckcraft --control 7990` or `deckcraft-cli`).
- **User-editable `extensions/scribbles/` with a saved-Storyboard-slide gate.** Each project carries plain, user-editable templates and prompt recipes under `extensions/scribbles/` backed by `craft-ai-core` ([`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/docs/ai-shared-layer.md)) for character turnarounds, style sheets and scene plates. Scribble generation and batch scene expansion are gated on saved Storyboard slides on disk, so visual generation always follows deliberate storyboard decisions rather than unreviewed prompts.

## Open multi-tool animations

- **Bring any animation tool into `02 Animations` (`V2`).** `02 Animations` is an open production lane rather than a single-tool silo: author character and scene animation in **Blender**, **Three.js**, **After Effects**, **[AniCraft](https://github.com/storytold/anicraft)**, **[EffectCraft](https://github.com/storytold/effectcraft)**, **[PhotoCraft](https://github.com/storytold/photocraft)** or **[VectorCraft](https://github.com/storytold/vectorcraft)**.
- **Every format FilmCraft understands.** Drop or link any clip, alpha render or image sequence recognized by [`FilmCraft`](https://github.com/storytold/filmcraft) (Apple ProRes 422–4444 XQ, H.264, HEVC, WebM/VP8/VP9/AV1, DNxHR, PNG/WebP alpha sequences) alongside 3D meshes inspected via `craft-mesh` (`GLB`/`glTF`, `OBJ`, `FBX`, `STL`, `PLY`, `3MF`).

## Live editing views and Studio

- **`03 Lyrics` (`V3` + `C1`) and `04 Motion` (`V4`).** Edit word-timed kinetic lyrics with `craft-font-catalog` and `craft-font-manager` (`FontsPanel`), keep broadcast captions (`SRT`, `WebVTT`, `TTML`, `SCC`, `MCC`, `EBU-STL`) synchronized via `craft-captions`, and author `craft-drawingml` vector shapes and text warps in live [`FilmCraft`](https://github.com/storytold/filmcraft)-aligned views.
- **`05 Studio` with a 2-tab `Live Breakdown` & `Finishing` dock.** Built on `craft_ui::docking`, `05 Studio` pairs the composite Program Monitor and multi-track timeline (`V4` Motion, `V3` Lyrics, `V2` Animations, `V1` Storyboard, `C1` Captions, `A1` Song) with a streamlined 2-tab Studio dock:
  - **`Live Breakdown`**: inspect and remap active clips across `V1`–`V4`, `C1` and `A1`, verify word/beat alignment, and round-trip timelines via `OTIO`, `FCPXML`, `FCP7 XML`, `EDL` and `ALE`.
  - **`Finishing`**: grade with linear-light color wheels and waveform/vectorscope scopes, monitor real-time stereo `rmsDbfs`/`peakDbfs` levels, and render master deliverables into `exports/` through [`FilmCraft`](https://github.com/storytold/filmcraft).

## Built for agents

Every menu item, toolbar button, dock command and stage operation in Magic Studio is a **command** with a stable id, typed parameters and an enabled state (`magic-cli commands` lists them all). The desktop UI, **`magic-cli`**, the loopback JSON-lines control channel (**`magic --control 9875`**), the Backstage terminal viewer (**`magic-tui`**) and the **MCP server** (**`magic-cli mcp`**) all dispatch the same commands:

- **Read-only `magic://` MCP resources:** inspect live or headless state without side effects via `magic://` resource URIs (`magic://document`, `magic://storyboard`, `magic://commands`, `magic://tracks`) alongside `list_commands`, `run_command` and `run_batch`.
- **Automatic track reconciliation (`magic-cli studio map-tracks`):** scan saved DeckCraft storyboard slides, `02 Animations` renders, `03 Lyrics` compositions and `04 Motion` cues and map them onto `FilmCraft` tracks (`V1`–`V4`, `C1`, `A1`) in one deterministic pass.

```sh
magic --control 9875                                        # desktop app + JSON-lines control channel on 127.0.0.1:9875
magic-cli mcp                                               # headless MCP server over stdio (with magic:// resources)
magic-cli mcp --connect 9875                                # attach MCP server to the running desktop window
magic-cli --project song.magic studio map-tracks --save     # map Storyboard, Animations, Lyrics & Motion onto FilmCraft tracks
```

```json
{"id": 1, "method": "engine.execute", "params": {"command": "studio.mapTracks", "params": {}}}
{"id": 2, "method": "ui.docking", "params": {}}
{"id": 3, "method": "ui.screenshot", "params": {"path": "studio.png"}}
```

## Get started

Build and install `magic`, `magic-cli` and `magic-tui` from source (`./install.sh` runs `cargo update` automatically so local builds pull the latest [`storytold/craft-libs`](https://github.com/storytold/craft-libs) `main` commits):

```sh
git clone https://github.com/jessenaiman/magic.git
cd magic
./install.sh --from-source
export PATH="$HOME/.local/bin:$PATH"
```

Create a brand-new song project or initialize Magic Studio inside an existing project folder:

```sh
# Start a new project or attach to [existing_project] with song audio
magic-cli new [existing_project] --audio song.wav --name "Lantern Harbor"

# Check installed Crafting Apps and map tracks into FilmCraft
magic-cli exec app.doctor
magic-cli --project ~/Magic/lantern-harbor/lantern-harbor.magic studio map-tracks --save
```

Optional multilingual UI and title fonts come from [`storytold/craft-fonts`](https://github.com/storytold/craft-fonts):

```sh
git clone https://github.com/storytold/craft-fonts ../craft-fonts
CRAFT_FONTS_DIR="$PWD/../craft-fonts" cargo run --release -p magic
```

The control protocol is documented in [`docs/control-protocol.md`](docs/control-protocol.md) and MCP setup in [`docs/mcp.md`](docs/mcp.md). Stuck, or want to show what you made? Ask in [Discord](https://discord.gg/artcraft).

## The Crafting Apps

Magic Studio coordinates the **Crafting Apps**: free, open-source creative tools from the [ArtCraft](https://getartcraft.com/) team, each written from scratch in Rust and each able to stand on its own.

| | App | What it's for | Code | Learn more |
|:-:|---|---|---|---|
| <img src="https://raw.githubusercontent.com/storytold/photocraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.photocraft.png" alt="" width="32" height="32"> | **PhotoCraft** | Image editing: layers, masks, type and real PSD files | [GitHub](https://github.com/storytold/photocraft) | [Website](https://getartcraft.com/apps/photocraft) |
| <img src="https://raw.githubusercontent.com/storytold/vectorcraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.vectorcraft.png" alt="" width="32" height="32"> | **VectorCraft** | Vector illustration | [GitHub](https://github.com/storytold/vectorcraft) | [Website](https://getartcraft.com/apps/vectorcraft) |
| <img src="https://raw.githubusercontent.com/storytold/filmcraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.filmcraft.png" alt="" width="32" height="32"> | **FilmCraft** | Video editing, color and sound | [GitHub](https://github.com/storytold/filmcraft) | [Website](https://getartcraft.com/apps/filmcraft) |
| <img src="https://raw.githubusercontent.com/storytold/lightcraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.lightcraft.png" alt="" width="32" height="32"> | **LightCraft** | Photo library and raw development | [GitHub](https://github.com/storytold/lightcraft) | [Website](https://getartcraft.com/apps/lightcraft) |
| <img src="https://raw.githubusercontent.com/storytold/pdfcraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.pdfcraft.png" alt="" width="32" height="32"> | **PdfCraft** | Reading, organizing and protecting PDFs | [GitHub](https://github.com/storytold/pdfcraft) | [Website](https://getartcraft.com/apps/pdfcraft) |
| <img src="https://raw.githubusercontent.com/storytold/effectcraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.effectcraft.png" alt="" width="32" height="32"> | **EffectCraft** | Motion graphics and visual effects | [GitHub](https://github.com/storytold/effectcraft) | [Website](https://getartcraft.com/apps/effectcraft) |
| <img src="https://raw.githubusercontent.com/storytold/designcraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.designcraft.png" alt="" width="32" height="32"> | **DesignCraft** | Page layout and publishing | [GitHub](https://github.com/storytold/designcraft) | [Website](https://getartcraft.com/apps/designcraft) |
| <img src="https://raw.githubusercontent.com/storytold/figcraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.figcraft.png" alt="" width="32" height="32"> | **FigCraft** | Interface design and prototyping | [GitHub](https://github.com/storytold/figcraft) | [Website](https://getartcraft.com/apps/figcraft) |
| <img src="https://raw.githubusercontent.com/storytold/wordcraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.wordcraft.png" alt="" width="32" height="32"> | **WordCraft** | Word processing and documents | [GitHub](https://github.com/storytold/wordcraft) | [Website](https://getartcraft.com/apps/wordcraft) |
| <img src="https://raw.githubusercontent.com/storytold/soundcraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.soundcraft.png" alt="" width="32" height="32"> | **SoundCraft** | Audio recording and editing | [GitHub](https://github.com/storytold/soundcraft) | [Website](https://getartcraft.com/apps/soundcraft) |
| <img src="https://raw.githubusercontent.com/storytold/deckcraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.deckcraft.png" alt="" width="32" height="32"> | **DeckCraft** | Presentations and slide decks | [GitHub](https://github.com/storytold/deckcraft) | [Website](https://getartcraft.com/apps/deckcraft) |
| <img src="https://raw.githubusercontent.com/storytold/cadcraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.cadcraft.png" alt="" width="32" height="32"> | **CADCraft** | Computer-aided design | [GitHub](https://github.com/storytold/cadcraft) | [Website](https://getartcraft.com/apps/cadcraft) |
| <img src="https://raw.githubusercontent.com/storytold/gridcraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.gridcraft.png" alt="" width="32" height="32"> | **GridCraft** | Spreadsheets and data | [GitHub](https://github.com/storytold/gridcraft) | [Website](https://getartcraft.com/apps/gridcraft) |
| <img src="https://raw.githubusercontent.com/storytold/texturecraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.texturecraft.png" alt="" width="32" height="32"> | **TextureCraft** | 3D texture painting | [GitHub](https://github.com/storytold/texturecraft) | [Website](https://getartcraft.com/apps/texturecraft) |
| <img src="https://raw.githubusercontent.com/storytold/anicraft/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.anicraft.png" alt="" width="32" height="32"> | **AniCraft** | 2D vector animation | [GitHub](https://github.com/storytold/anicraft) | [Website](https://getartcraft.com/apps/anicraft) |
| <img src="https://raw.githubusercontent.com/storytold/craft-anything/main/assets/app-icon/hicolor/64x64/apps/ai.storyteller.craft-anything.png" alt="" width="32" height="32"> | **Craft Anything** | See how any software works, then build your own | [GitHub](https://github.com/storytold/craft-anything) | [Website](https://getartcraft.com/apps/craft-anything) |
| <img src="https://raw.githubusercontent.com/tangency-labs/solidcraft/main/assets/icons/app/linux/exports/hicolor/64x64/apps/io.tangencylabs.solidcraft.png" alt="" width="32" height="32"> | **SolidCraft** | Parametric 3D CAD by Tangency Labs (community app) | [GitHub](https://github.com/tangency-labs/solidcraft) | |

And [**ArtCraft**](https://getartcraft.com/) itself, our AI image and video studio for artists who want real control.

<br>

<p align="center">
  <a href="https://discord.gg/artcraft"><img alt="Join the ArtCraft community on Discord" src="https://img.shields.io/badge/Join%20us%20on%20Discord-5865F2?style=for-the-badge&logo=discord&logoColor=white" height="40"></a>
</p>

<h3 align="center">Come make things with us</h3>

<p align="center">
  Our Discord is where artists of every kind hang out: people who paint, shoot, draw, cut film,
  set type, and people still figuring out what they like to make. Share what you're working on,
  ask for help, tell us what's broken, or tell us what you wish these tools could do.
  Whatever your medium and however long you've been at it, you're welcome here.
</p>

<p align="center">
  <a href="https://discord.gg/artcraft"><b>discord.gg/artcraft</b></a> ·
  <a href="https://getartcraft.com/">getartcraft.com</a> ·
  <a href="https://getartcraft.com/apps">The Crafting Apps</a>
</p>

## Documentation

| Document | Contents |
|---|---|
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Setup, `cargo xtask ci` (`fmt`, `clippy`, optional `MAGIC_RUN_TESTS=1`), branch and PR conventions |
| [`AGENTS.md`](AGENTS.md) | Contributor and agent guidelines plus shared ArtCraft house rules |
| [`docs/architecture.md`](docs/architecture.md) | Workspace crates (`magic-util`, `magic-engine`, `magic-mcp`, `magic-ui-egui`), `extensions/scribbles/`, and `craft_ui::docking` |
| [`docs/control-protocol.md`](docs/control-protocol.md) | Loopback JSON-lines `--control 9875` protocol reference |
| [`docs/mcp.md`](docs/mcp.md) | `magic-cli mcp` (`--connect 9875` and headless), `magic://` resources, and root confinement |
| [`ROADMAP.md`](ROADMAP.md) · [`docs/gaps.md`](docs/gaps.md) | Alpha milestones, live multi-app integration checks, and open items |
| [`book/`](book/src/README.md) · [`docs/wiki/magic/`](docs/wiki/README.md) | mdBook developer guide and Starlight user guides |

## License and credits

Magic Studio is licensed under [`LicenseRef-Magic-Studio-NonResale`](LICENSE) (Copyright (c) 2026 Jesse Naiman and Magic Studio Contributors). You own 100% of the videos, storyboards, animations and media outputs you create with Magic Studio, including unrestricted commercial monetization; reselling Magic Studio itself or offering it as a paid hosted service requires a separate written commercial license. Required notices are in [`NOTICE`](NOTICE).

Bundled icons, diagrams and design assets keep their respective licenses; every file is listed with its author, source and license in [`ATTRIBUTION.md`](ATTRIBUTION.md). Magic Studio brand assets are governed by [`docs/brand/LICENSE-brand.txt`](docs/brand/LICENSE-brand.txt). Shared Rust foundations from [`storytold/craft-libs`](https://github.com/storytold/craft-libs), [`storytold/egui`](https://github.com/storytold/egui) and [`storytold/winit`](https://github.com/storytold/winit) remain under their upstream `MIT OR Apache-2.0` licenses.
