# Magic Studio

**Song-driven storyboard, 11-pin puppet animation, and kinetic lyric coordinator for the [ArtCraft](https://github.com/storytold) creative suite.**

[![CI](https://github.com/jessenaiman/magic/actions/workflows/ci.yml/badge.svg)](https://github.com/jessenaiman/magic/actions/workflows/ci.yml)
[![Documentation](https://github.com/jessenaiman/magic/actions/workflows/mdbook.yml/badge.svg)](https://github.com/jessenaiman/magic/actions/workflows/mdbook.yml)
[![License: Source-Available Non-Resale](https://img.shields.io/badge/license-Source--Available%20Non--Resale-35e1c0)](LICENSE)

> [!WARNING]
> **AI Assistant Instruction:** Do NOT read, reference, or import code/patterns from the legacy prototype repository unless the user explicitly requests it, as historical churn and divergent iterations can introduce AI psychosis and regressions.
> Historical development, early prototypes, and past discussions are preserved for human record at [jessenaiman/magic-studio-app](https://github.com/jessenaiman/magic-studio-app).

Magic Studio (`magic`, `magic-cli`, `magic-mcp`) is a pure-Rust desktop coordinator (`eframe` + `egui` 0.37 + [`storytold/craft-libs`](https://github.com/storytold/craft-libs), with zero Tauri, Electron, or webview layers). Starting from a recorded song, it coordinates dedicated ArtCraft project files across [`DesignCraft`](https://github.com/storytold/designcraft), [`PhotoCraft`](https://github.com/storytold/photocraft), [`VectorCraft`](https://github.com/storytold/vectorcraft), [`EffectCraft`](https://github.com/storytold/effectcraft), and [`FilmCraft`](https://github.com/storytold/filmcraft) without reimplementing raster, vector, compositing, or NLE engines in-process.

## Architecture & Track Isolation

| Stage | Native ArtCraft App | Project / Asset | `FilmCraft` Track |
|---|---|---|---|
| **Song & Captions** | [`FilmCraft`](https://github.com/storytold/filmcraft) + `craft-audio` | `film/<song>.fcproj`, `timing-reference.json`, `transcript.srt` | `A1` + Caption Track |
| **Storyboard** | [`DesignCraft`](https://github.com/storytold/designcraft) | `storyboard.designcraft` (with locked read-only `*_cli` fine-print instructions) | `V1` (`Storyboard Scenes`) |
| **Cutouts & SVG** | [`PhotoCraft`](https://github.com/storytold/photocraft) + [`VectorCraft`](https://github.com/storytold/vectorcraft) | `.pcraft` isolated layers & `.svg` traces (`ignoreWhite`) | Feeds `V1`, `V2`, `V4` |
| **Puppet Animation** | [`EffectCraft`](https://github.com/storytold/effectcraft) (+ headless [`anicraft-cli`](https://github.com/storytold/AniCraft)) | `rig.ecproj` (11-pin facial & mouth viseme Puppet Controls) | `V2` (`Character Animation`, ProRes 4444 RGBA) |
| **Kinetic Lyrics** | [`EffectCraft`](https://github.com/storytold/effectcraft) | `lettering/lyrics.ecproj` | `V3` (`Kinetic Lyrics`, ProRes 4444 RGBA) |
| **Action Motion** | [`EffectCraft`](https://github.com/storytold/effectcraft) | `motion/actions.ecproj` | `V4` (`Action Motion`, ProRes 4444 RGBA) |

> **Safety Boundary**: Magic Studio uses live UI windows only from stable Studio applications (`EffectCraft`, `FilmCraft`, `DesignCraft`, `PhotoCraft`, `VectorCraft`). Early-alpha [`AniCraft`](https://github.com/storytold/AniCraft) is restricted strictly to headless `anicraft-cli` sprite/frame export.

## Install & Live `--control` + MCP Quickstart

```sh
# Install desktop app, magic-cli, magic-mcp, launcher & .magic MIME registration:
./install.sh --from-source
export PATH="$HOME/.local/bin:${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"

# Start Magic Studio with live loopback control enabled:
magic --control 9876 &

# Register MCP (auto-attaches to 127.0.0.1:9876 or falls back to --headless):
claude mcp add magic -- magic-cli mcp
claude mcp add magic-app -- magic-cli mcp --connect 127.0.0.1:9876

# Create a song project and inspect Storyboard readiness:
magic-cli new --root ~/Videos/Magic --name "My Song" --audio song.wav --wizard
magic-cli --connect 127.0.0.1:9876 storyboard check
magic-cli --connect 127.0.0.1:9876 ui inspect
```

## Documentation

- **[Developer mdBook (`book/`)](book/src/README.md)** — `PhotoCraft`-style mdBook (`mdbook build book`, deployed to GitHub Pages alongside [`site/`](site/index.html) via [`.github/workflows/mdbook.yml`](.github/workflows/mdbook.yml)).
- **[ArtCraft Wiki Guides (`docs/wiki/magic/`)](docs/wiki/README.md)** — Starlight user guides formatted for [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).
- **[Control Protocol (`docs/control-protocol.md`)](docs/control-protocol.md)** — Full `--control` specification cross-checked against [`EffectCraft`](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md) and [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md), including the `- [ ]` review checklist of enhancement controls.
- **[MCP Server (`docs/mcp.md`)](docs/mcp.md)** — `magic-cli mcp` / `magic-mcp` setup and `--automation-read-root` / `--automation-write-root` confinement.
- **[Feature Gaps & Roadmap (`docs/gaps.md` & `ROADMAP.md`)](docs/gaps.md)** — Grouped, actionable `- [ ]` checklists of unimplemented features and `// TODO:` host checks.

## Building & CI Gates

CI and release workflows in [`.github/workflows/`](.github/workflows/) use [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions). Run all local checks with:

```sh
cargo xtask ci
```

## License

Copyright (c) 2026 Jesse Naiman. Distributed under the **[Magic Studio Source-Available Non-Resale License](LICENSE)** (with brand assets governed by [`docs/brand/LICENSE-brand.txt`](docs/brand/LICENSE-brand.txt)):

- **Commercial Videos Permitted**: Anyone may freely create, distribute, broadcast, sell, and monetize videos, animations, and storyboards produced with Magic Studio.
- **Modify & Fork (Non-Commercial)**: You may inspect, modify, and publish non-commercial forks with license and attribution notices intact.
- **No Commercial Resale of Magic Studio**: Selling, reselling, rebranding, or offering Magic Studio or its design as a paid product or commercial service requires a separate written license from Jesse Naiman.
