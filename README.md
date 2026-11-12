<p align="center">
  <img src="assets/app-icon/icon.png" width="112" height="112" alt="Magic Studio icon">
</p>

<h1 align="center">Magic Studio</h1>

<p align="center">
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
