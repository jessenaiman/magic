# Installation

## Prerequisites

Install Rust (`rustc 1.95+` via [`rustup`](https://rustup.rs); Linux builds also use standard system windowing/audio headers such as `libxkbcommon-dev`, `libwayland-dev`, `libx11-dev`, and `libasound2-dev`) plus the [ArtCraft](https://github.com/storytold) apps you plan to coordinate on `PATH` (discovered via `magic-cli exec app.doctor`):

- [`DeckCraft`](https://github.com/storytold/deckcraft) (`deckcraft`, `deckcraft-cli`) — `.deckcraft` / `.pptx` creative storyboards and PDF export
- [`AniCraft`](https://github.com/storytold/anicraft) (`anicraft`, `anicraft-cli`), Blender, Three.js, or After Effects — open `02 Animations` (`V2`) workflows
- [`PhotoCraft`](https://github.com/storytold/photocraft) (`photocraft`, `photocraft-cli`) — subject isolation and `.pcraft` layers
- [`VectorCraft`](https://github.com/storytold/vectorcraft) (`vectorcraft`, `vectorcraft-cli`) — SVG tracing and vector illustration
- [`EffectCraft`](https://github.com/storytold/effectcraft) (`effectcraft`, `effectcraft-cli`) — puppet rigs, kinetic lyrics (`V3`), and motion graphics (`V4`)
- [`FilmCraft`](https://github.com/storytold/filmcraft) (`filmcraft`, `filmcraft-cli`) — master `A1` + `C1` + `V1`–`V4` timeline and final delivery

## Install From Source

```sh
./install.sh --from-source
export PATH="$HOME/.local/bin:${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
magic-cli exec app.doctor
```

`./install.sh --from-source` runs `cargo update` first so local builds pull the latest [`storytold/craft-libs`](https://github.com/storytold/craft-libs) `main` commits automatically.

To install only `magic-cli` (`magic-cli mcp`) into `~/.cargo/bin`:

```sh
cargo install --path apps/magic-cli
magic-cli --help
magic-cli mcp --help
```
