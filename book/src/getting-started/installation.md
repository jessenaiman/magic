# Installation

## Prerequisites

Install the sibling [ArtCraft](https://github.com/storytold) tools you plan to orchestrate on `PATH` (or in standard `/Applications`, `~/.local/bin`, or `%LOCALAPPDATA%\Programs\ArtCraft` locations discovered by `magic-cli exec app.doctor`):

- [`DesignCraft`](https://github.com/storytold/designcraft) (`designcraft`, `designcraft-cli`) — storyboard spreads, PDF/PPTX export, and live layout
- [`PhotoCraft`](https://github.com/storytold/photocraft) (`photocraft`, `photocraft-cli`) — subject isolation, part slicing, and `.pcraft` layers
- [`VectorCraft`](https://github.com/storytold/vectorcraft) (`vectorcraft`, `vectorcraft-cli`) — background-free SVG tracing (`imageTrace.make`)
- [`EffectCraft`](https://github.com/storytold/effectcraft) (`effectcraft`, `effectcraft-cli`) — 2.5D puppet rigs, facial viseme pins, kinetic lyrics, and song-action motion
- [`FilmCraft`](https://github.com/storytold/filmcraft) (`filmcraft`, `filmcraft-cli`) — master `A1` + `V1`–`V4` timeline, captions, and final delivery

> **AniCraft Safety Note**: Early-alpha [`AniCraft`](https://github.com/storytold/AniCraft) is optional and invoked **headlessly only** (`anicraft-cli`) to export PNG sprite sequences into `EffectCraft`. Magic never opens the `anicraft` desktop GUI.

## Install From Source

With Rust 1.88+, [Bun](https://bun.sh), and your platform's Tauri v2 prerequisites installed:

```sh
./install.sh --from-source
export PATH="$HOME/.local/bin:${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
magic-cli exec app.doctor
```

To install only the headless CLI and MCP server binaries:

```sh
cargo install --path apps/magic-cli --locked
magic-cli --help
magic-mcp --help
```
