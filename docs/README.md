# Magic Studio Developer & Suite Documentation

Magic Studio keeps its architecture, protocol, and user documentation across crate `README.md` guides, reference pages in `docs/`, the mdBook in `book/`, and links to live `storytold/*` repositories:

1. **Workspace & Crate `README.md` Guides**:
   - **[Root `README.md`](../README.md)** — Product overview, stage workflows, `magic-cli` / MCP quickstart, and Crafting Apps directory
   - **[`crates/util/README.md`](../crates/util/README.md)** — `craft-timeline` `Tick`/SMPTE, `craft-audio`, `craft-media`, `craft-doc`, `craft-font-catalog`, `craft-text`
   - **[`crates/engine/README.md`](../crates/engine/README.md)** — Headless `Session`, `DeckCraft` storyboard & `extensions/scribbles/` gate, open `02 Animations` (`craft-mesh` + `FilmCraft` media), `craft-drawingml` motion, `craft-captions`, and `FilmCraft` `.fcproj` / NLE interchange
   - **[`crates/mcp/README.md`](../crates/mcp/README.md)** — Stdio MCP server (`magic-cli mcp` & `--connect 9875`) with read-only `magic://` resources
   - **[`crates/ui-egui/README.md`](../crates/ui-egui/README.md)** — `craft_ui::docking` panels/workspaces, 2-tab `05 Studio` (`Live Breakdown` & `Finishing`) dock, SVG icon controls, and `FontsPanel`
   - **[`apps/magic/README.md`](../apps/magic/README.md)** — Native `eframe` + `wgpu` desktop shell, floating OS dock viewports, `cpal` audio, and `--control 9875`
   - **[`apps/magic-cli/README.md`](../apps/magic-cli/README.md)**, **[`apps/magic-tui/README.md`](../apps/magic-tui/README.md)**, **[`xtask/README.md`](../xtask/README.md)**
2. **Developer Book (`book/`) & Community Wiki (`docs/wiki/magic/`)** — Built with [mdBook](https://rust-lang.github.io/mdBook/) (`mdbook build book`) and Starlight guides formatted for [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).

## Reference Specifications in `docs/`

- **[Architecture (`architecture.md`)](architecture.md)** — Workspace crates, `extensions/scribbles/`, `craft_ui::docking` ownership, and SVG icon conventions.
- **[Control Protocol (`control-protocol.md`)](control-protocol.md)** — Loopback JSON-lines protocol (`127.0.0.1:9875`) including `ui.docking`, `DockCommand::IDS`, `ui.playback` dBFS meters, `ui.viewer.*`, and `ui.timeline.*`.
- **[MCP Server (`mcp.md`)](mcp.md)** — `magic-cli mcp` (`--connect` vs headless), `magic://` resource URIs, and `--automation-read-root` / `--automation-write-root` confinement.
- **[Feature Gaps (`gaps.md`)](gaps.md)** — Remaining live host verification hooks and upstream tracking items.
- **[House Rules (`house-rules-core.md`)](house-rules-core.md)** — Shared standards from [`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/standards/house-rules-core.md).

## Upstream `storytold` Repositories

- **[`storytold/craft-libs`](https://github.com/storytold/craft-libs/blob/main/README.md)**:
  - **[`craft-ai-core` (`docs/ai-shared-layer.md`)](https://github.com/storytold/craft-libs/blob/main/docs/ai-shared-layer.md)** — Pure, wasm-safe AI contract (connections, models, tools, requests, jobs, consent, history, route spec)
  - **[Panel Docking (`docs/ui-docking.md`)](https://github.com/storytold/craft-libs/blob/main/docs/ui-docking.md)** & **[Theme Infrastructure (`docs/theme-infrastructure.md`)](https://github.com/storytold/craft-libs/blob/main/docs/theme-infrastructure.md)** — `Workspace<P>`, `Layout<P>`, `DockArea`, `DockStyle::from_ui`, `DockCommand::IDS`, `FloatSpace`, `SvgIconSet`
  - **`craft-timeline`**, **`craft-captions`**, **`craft-mesh`**, **`craft-drawingml`**, **`craft-zip`**, **`craft-audio`**, **`craft-media`**, **`craft-font-catalog`**, **`craft-font-store`**, **`craft-font-manager`**, **`craft-text`** ([`craft-libs/README.md`](https://github.com/storytold/craft-libs/blob/main/README.md))
- **Crafting Applications**:
  - **[`DeckCraft`](https://github.com/storytold/deckcraft/blob/main/README.md)** · **[`AniCraft`](https://github.com/storytold/anicraft/blob/main/README.md)** · **[`PhotoCraft`](https://github.com/storytold/photocraft/blob/main/README.md)** · **[`VectorCraft`](https://github.com/storytold/vectorcraft/blob/main/README.md)** · **[`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/README.md)** · **[`DesignCraft`](https://github.com/storytold/designcraft/blob/main/README.md)** · **[`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/README.md)**
