---
name: magic
description: Drive Magic Studio via `magic-cli` and `magic-cli mcp` (headless with `magic://` resources or live `magic --control 9875`) across the ArtCraft suite.
---

# Magic Studio (`magic-cli` & `magic-cli mcp`)

## 1. Connect via MCP or CLI (`--control` Live Watching)

- Build & install CLI + desktop binaries: `./install.sh --from-source`
- Create or attach to a project: `magic-cli new [existing_project] --audio song.wav --name "Song Title"`
- Reconcile and map tracks onto `FilmCraft`: `magic-cli --project song.magic studio map-tracks --save`
- Start live desktop session: `magic --control 9875`
- Attach MCP server to live window: `magic-cli mcp --connect 9875` (or `127.0.0.1:9875`)
- Headless stdio session (default, exposes `magic://` resources): `magic-cli mcp`
- Headless with confined roots: `magic-cli mcp --automation-read-root <DIR> --automation-write-root <DIR>`
- Discover commands: `list_commands` (MCP) or `magic-cli commands` / `magic-cli describe <id>` (CLI)

## 2. Upstream ArtCraft Documentation

Consult the maintained `README.md` and `docs/` pages in `storytold/*` repositories:

- **Magic Studio MCP & Control**: [`docs/mcp.md`](../../../docs/mcp.md) and [`docs/control-protocol.md`](../../../docs/control-protocol.md)
- **Shared Libraries (`craft-libs`)**: [README.md](https://github.com/storytold/craft-libs/blob/main/README.md) · [`craft-ai-core` contract](https://github.com/storytold/craft-libs/blob/main/docs/ai-shared-layer.md)
- **DeckCraft (`:7990`, `deckcraft-cli`)**: [README.md](https://github.com/storytold/deckcraft/blob/main/README.md) · [MCP](https://github.com/storytold/deckcraft/blob/main/docs/mcp.md) · [Control Protocol](https://github.com/storytold/deckcraft/blob/main/docs/control-protocol.md)
- **EffectCraft (`:9877`, `effectcraft-cli`)**: [README.md](https://github.com/storytold/effectcraft/blob/main/README.md) · [Agents](https://github.com/storytold/effectcraft/blob/main/docs/agents.md) · [MCP](https://github.com/storytold/effectcraft/blob/main/docs/mcp.md) · [Control Protocol](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md)
- **FilmCraft (`:9876`, `filmcraft-cli`)**: [README.md](https://github.com/storytold/filmcraft/blob/main/README.md) · [Agents](https://github.com/storytold/filmcraft/blob/main/docs/agents.md) · [Control Protocol](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md) · [Captions](https://github.com/storytold/filmcraft/blob/main/docs/captions.md) · [Transcripts](https://github.com/storytold/filmcraft/blob/main/docs/transcripts.md)
- **AniCraft, PhotoCraft & VectorCraft**: [AniCraft README.md](https://github.com/storytold/anicraft/blob/main/README.md) · [PhotoCraft README.md](https://github.com/storytold/photocraft/blob/main/README.md) · [VectorCraft README.md](https://github.com/storytold/vectorcraft/blob/main/README.md)
