---
name: magic
description: Drive Magic Studio via `magic-cli mcp` (live `magic --control 9876` or `--headless`) and official ArtCraft CLI tools.
---

# Magic Studio (`magic-cli` / `magic-mcp`)

## 1. Connect via MCP or CLI (`--control` Live Watching)

- Start live desktop session: `magic --control 9876`
- Attach MCP server to live window: `magic-cli mcp --connect 127.0.0.1:9876` (or `magic-mcp --connect 127.0.0.1:9876`)
- Auto-attach or fall back to headless: `magic-cli mcp`
- Headless with confined roots: `magic-cli mcp --headless --automation-read-root <DIR> --automation-write-root <DIR>`
- Discover commands: `list_commands` (MCP) or `magic-cli commands` / `magic-cli describe <id>` (CLI)

## 2. Official ArtCraft Documentation & Tutorials (Authoritative References)

Do not paraphrase or invent custom animation rules; consult the upstream documentation directly:

- **Magic Studio MCP & Control**: [`docs/mcp.md`](../../../docs/mcp.md) and [`docs/control-protocol.md`](../../../docs/control-protocol.md)
- **EffectCraft Compositing, Puppet Pins, Text & Expressions**:
  - [EffectCraft CLI](https://github.com/storytold/effectcraft/blob/main/docs/cli.md)
  - [EffectCraft MCP Server](https://github.com/storytold/effectcraft/blob/main/docs/mcp-server.md)
  - [Animation, Keyframes, Puppet Pins & Expressions](https://github.com/storytold/effectcraft/blob/main/docs/animation-and-expressions.md)
  - [Text Animators & Presets](https://github.com/storytold/effectcraft/blob/main/docs/text-and-presets.md)
  - [Masks, Track Mattes & 3D Layers](https://github.com/storytold/effectcraft/blob/main/docs/masks-mattes-and-3d.md)
  - [Effects & Layer Styles](https://github.com/storytold/effectcraft/blob/main/docs/effects-and-styles.md)
- **FilmCraft Timeline, Captions & Export**:
  - [FilmCraft CLI & Automation](https://github.com/storytold/filmcraft/blob/main/docs/cli-and-automation.md)
  - [FilmCraft MCP Server](https://github.com/storytold/filmcraft/blob/main/docs/mcp-server.md)
  - [Captions & Speech Transcription](https://github.com/storytold/filmcraft/blob/main/docs/captions-and-speech.md)
  - [Markers & Cue Interchange](https://github.com/storytold/filmcraft/blob/main/docs/markers.md)
- **DesignCraft, PhotoCraft, VectorCraft & Headless AniCraft CLI**:
  - [DesignCraft MCP & Scripting](https://github.com/storytold/designcraft/blob/main/docs/mcp.md)
  - [PhotoCraft Subject Isolation (`layer.removeBackground`)](https://github.com/storytold/photocraft/blob/main/README.md)
  - [VectorCraft Image Trace (`imageTrace.make`)](https://github.com/storytold/vectorcraft/blob/main/README.md)
  - [AniCraft Headless CLI (`anicraft-cli`, early alpha — never launch GUI)](https://github.com/storytold/AniCraft)
