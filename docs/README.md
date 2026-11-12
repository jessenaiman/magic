# Magic Studio Developer & Suite Documentation

<<<<<<< HEAD
Magic Studio uses **two official ArtCraft documentation surfaces** — no ad-hoc markdown logs or paraphrased animation skills are kept in this repository:

1. **Developer & Architecture Book (`book/`)** — Built with [mdBook](https://rust-lang.github.io/mdBook/) following [`storytold/photocraft`](https://github.com/storytold/photocraft) (`mdbook build book`, deployed to GitHub Pages by `.github/workflows/mdbook.yml`).
2. **User & Community Wiki (`docs/wiki/magic/`)** — Starlight-formatted user guides ready for [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki) (`overview.md`, `getting-started.md`, `install-update.md`, `workflows.md`, `troubleshooting.md`).

## Reference Specifications in `docs/`

- **[Control Protocol (`--control`)](control-protocol.md)** — Complete loopback JSON-lines protocol for `magic --control 9875`, `magic-cli --connect`, `magic-cli mcp`, and `EffectCraft`/`FilmCraft` bridges, including the review checklist of enhancement controls.
- **[MCP Server (`magic-cli mcp`)](mcp.md)** — Remote (`--connect 127.0.0.1:9875`) vs Headless (`--headless`) setup, `.mcp.json` configuration, and `--automation-read-root` / `--automation-write-root` confinement.
- **[Architecture & Layer Boundaries](architecture.md)** — Single-responsibility crate hierarchy (`magic-util`, `magic-scribbles`, `magic-engine`, `magic-mcp`, `magic-cli`, `magic`) and `FilmCraft` `V1`–`V4` track isolation.
- **[Craft-Wiki App Guides](wiki/README.md)** — Drop-in user documentation pages for [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).
- **[House Rules](house-rules-core.md)** — Engineering standards from [`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/docs/house-rules-core.md).
- **[Brand Asset Notice](brand/LICENSE-brand.txt)** — Trademark and visual identity reservation.

## Upstream ArtCraft Suite Documentation

- **[FilmCraft](https://github.com/storytold/filmcraft)** — [Control Protocol](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md) · [CLI & Automation](https://github.com/storytold/filmcraft/blob/main/docs/agents.md) · [MCP Server](https://github.com/storytold/filmcraft/blob/main/docs/agents.md) · [Captions & Speech](https://github.com/storytold/filmcraft/blob/main/docs/captions.md)
- **[EffectCraft](https://github.com/storytold/effectcraft)** — [Control Protocol](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md) · [CLI](https://github.com/storytold/effectcraft/blob/main/docs/agents.md) · [MCP Server](https://github.com/storytold/effectcraft/blob/main/docs/mcp.md) · [Animation, Puppet Pins & Expressions](https://github.com/storytold/effectcraft/blob/main/docs/agents.md) · [Text & Presets](https://github.com/storytold/effectcraft/blob/main/docs/agents.md)
- **[DesignCraft](https://github.com/storytold/designcraft)** — [MCP & CLI](https://github.com/storytold/designcraft/blob/main/docs/mcp.md)
- **[PhotoCraft](https://github.com/storytold/photocraft)** — [README & mdBook](https://github.com/storytold/photocraft/blob/main/README.md) · [Roadmap](https://github.com/storytold/photocraft/blob/main/docs/roadmap.md)
- **[VectorCraft](https://github.com/storytold/vectorcraft)** — [README & MCP](https://github.com/storytold/vectorcraft/blob/main/README.md)
- **[AniCraft](https://github.com/storytold/AniCraft)** — Headless `anicraft-cli` frame/sprite export only (early-alpha desktop UI disabled).
=======
Magic Studio keeps all architecture, protocol, and upstream reference documentation in **two official ArtCraft surfaces** plus crate-level `README.md` navigation guides — **start here whenever exploring or modifying code**:

1. **Workspace & Crate `README.md` Navigation Maps** — Every crate and binary links directly to **where** each module lives and **why** you explore it:
   - **[Root `README.md`](../README.md)** — End-to-end workspace map & stage-by-stage file index
   - **[`crates/util/README.md`](../crates/util/README.md)** — `craft-timeline` `Tick`/SMPTE, `craft-audio`, `craft-media`, `craft-doc`, `craft-font-catalog`, `craft-text`
   - **[`crates/scribbles/README.md`](../crates/scribbles/README.md)** — Deterministic storyboard & turnaround prompt/grid geometry
   - **[`crates/engine/README.md`](../crates/engine/README.md)** — Headless `Session`, `craft-mesh` 3D pipeline, `craft-drawingml` motion, `craft-zip` archives, `craft-captions`, and `FilmCraft` `.fcproj` / NLE interchange
   - **[`crates/mcp/README.md`](../crates/mcp/README.md)** — Stdio MCP server (`--headless` & `--connect`)
   - **[`crates/ui-egui/README.md`](../crates/ui-egui/README.md)** — `craft_ui::docking` panels/workspaces, 70+ clean-room `assets/icons/*.svg` controls, `FontsPanel`, and stage views
   - **[`apps/magic/README.md`](../apps/magic/README.md)** — Native `eframe` + `wgpu` desktop shell, floating OS dock viewports, `cpal` audio, and `--control 9875`
   - **[`apps/magic-cli/README.md`](../apps/magic-cli/README.md)**, **[`apps/magic-tui/README.md`](../apps/magic-tui/README.md)**, **[`xtask/README.md`](../xtask/README.md)**
2. **Developer Book (`book/`) & Community Wiki (`docs/wiki/magic/`)** — Built with [mdBook](https://rust-lang.github.io/mdBook/) (`mdbook build book`) and Starlight guides ready for [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).

## Reference Specifications in `docs/`

- **[Architecture & Crate Layers (`architecture.md`)](architecture.md)** — `L3` → `L7` downward DAG, `< 900`-line file ceiling, `craft_ui::docking` ownership, and icon-first SVG rules.
- **[Control Protocol (`control-protocol.md`)](control-protocol.md)** — Loopback JSON-lines protocol (`127.0.0.1:9875`) including `ui.docking`, `DockCommand::IDS`, `ui.playback` dBFS meters, `ui.viewer.*`, and `ui.timeline.*`.
- **[MCP Server (`mcp.md`)](mcp.md)** — `magic-cli mcp` (`--connect` vs `--headless`) and `--automation-read-root` / `--automation-write-root` confinement.
- **[Feature Gaps (`gaps.md`)](gaps.md)** — Remaining host-CLI verification hooks and upcoming enhancements.
- **[House Rules (`house-rules-core.md`)](house-rules-core.md)** — Shared standards from [`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/standards/house-rules-core.md).

## Local `docs/craft-reference/` & Upstream `storytold` Documentation

All core `storytold` crate and app specifications are mirrored under [`docs/craft-reference/`](craft-reference/) for offline verification:

- **[`storytold/craft-libs`](https://github.com/storytold/craft-libs)** ([local index](craft-reference/craft-libs/README.md)):
  - **[Panel Docking (`ui-docking.md`)](craft-reference/craft-libs/docs/ui-docking.md)** & **[`craft-ui`](craft-reference/craft-libs/crates/craft-ui/README.md)** — `Workspace<P>`, `Layout<P>`, `DockArea`, `DockStyle::from_ui`, `DockCommand::IDS`, `FloatSpace`, `SvgIconSet`
  - **[`craft-timeline`](craft-reference/craft-libs/crates/craft-timeline/README.md)** — `Tick` (25,401,600/s), `FrameRate`, `Timecode`, `EDL`, `ALE`, `FCP7 XML`, `FCPXML`, `OpenTimelineIO`
  - **[`craft-captions`](craft-reference/craft-libs/crates/craft-captions/README.md)** — `SRT`, `WebVTT`, `TTML`, `SCC`, `MCC`, `EBU-STL`
  - **[`craft-mesh`](craft-reference/craft-libs/crates/craft-mesh/README.md)** — `glTF`/`GLB`, `OBJ`/`MTL`, `FBX`, `STL`, `PLY`, `3MF` scenes, skins, joints, animations
  - **[`craft-drawingml`](craft-reference/craft-libs/crates/craft-drawingml/README.md)** — ECMA-376 preset shapes (`a:prstGeom`), custom geometry, text warps (`a:prstTxWarp`), and theme color schemes
  - **[`craft-zip`](craft-reference/craft-libs/crates/craft-zip/README.md)** — Budget-bounded, `unsafe`-free ZIP reading, strict `preflight`, and deterministic 1980-timestamp writing
  - **[`craft-audio`](craft-reference/craft-libs/crates/craft-audio/README.md)** & **[`craft-media`](craft-reference/craft-libs/crates/craft-media/README.md)** — Pure-Rust PCM decoding, WAV encoding, dBFS peaks, and container probing
  - **[`craft-font-catalog`](craft-reference/craft-libs/crates/craft-font-catalog/README.md)**, **[`craft-font-store`](craft-reference/craft-libs/crates/craft-font-store/README.md)**, **[`craft-font-manager`](craft-reference/craft-libs/crates/craft-font-manager/README.md)**, **[`craft-text`](craft-reference/craft-libs/crates/craft-text/README.md)** & **[`storytold/font-preview-service`](craft-reference/font-preview-service/README.md)**
- **Six Standard ArtCraft Applications**:
  - **[`PhotoCraft`](https://github.com/storytold/photocraft)** ([local](craft-reference/core-craft-apps/photocraft/README.md)) · **[`VectorCraft`](https://github.com/storytold/vectorcraft)** ([local](craft-reference/core-craft-apps/vectorcraft/README.md)) · **[`DesignCraft`](https://github.com/storytold/designcraft)** ([local](craft-reference/core-craft-apps/designcraft/README.md)) · **[`EffectCraft`](https://github.com/storytold/effectcraft)** ([local](craft-reference/core-craft-apps/effectcraft/README.md)) · **[`FilmCraft`](https://github.com/storytold/filmcraft)** ([local](craft-reference/core-craft-apps/filmcraft/README.md))
- **[`storytold/craft-ai-integrations`](https://github.com/storytold/craft-ai-integrations)** ([local](craft-reference/core-craft-apps/craft-ai-integrations/README.md))
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
