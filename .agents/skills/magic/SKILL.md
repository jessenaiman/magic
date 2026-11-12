---
name: magic
description: Drive Magic Studio via `magic-cli mcp` (live `magic --control 9875` or `--headless`) and official ArtCraft CLI tools.
---

# Magic Studio (`magic-cli` / `magic-mcp`)

## 1. Connect via MCP or CLI (`--control` Live Watching)

- Start live desktop session: `magic --control 9875`
- Attach MCP server to live window: `magic-cli mcp --connect 127.0.0.1:9875` (or `magic-mcp --connect 127.0.0.1:9875`)
<<<<<<< HEAD
- Auto-attach or fall back to headless: `magic-cli mcp`
=======
- Headless stdio session (default): `magic-cli mcp`
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
- Headless with confined roots: `magic-cli mcp --headless --automation-read-root <DIR> --automation-write-root <DIR>`
- Discover commands: `list_commands` (MCP) or `magic-cli commands` / `magic-cli describe <id>` (CLI)

## 2. Verified Upstream ArtCraft Documentation

Consult the real upstream files in `storytold/*` repositories rather than invented filenames:

- **Magic Studio MCP & Control**: [`docs/mcp.md`](../../../docs/mcp.md) and [`docs/control-protocol.md`](../../../docs/control-protocol.md)
- **EffectCraft (`:9877`, `effectcraft-cli`)**:
  - [EffectCraft Agents & CLI Cookbook](https://github.com/storytold/effectcraft/blob/main/docs/agents.md)
  - [EffectCraft Control Protocol](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md)
  - [EffectCraft MCP Server (`--autosave` / `--bridge 9877`)](https://github.com/storytold/effectcraft/blob/main/docs/mcp.md)
  - [EffectCraft Effects Catalog](https://github.com/storytold/effectcraft/blob/main/docs/effects.md)
  - [EffectCraft Footage & AEP Import](https://github.com/storytold/effectcraft/blob/main/docs/footage.md)
- **FilmCraft (`:9876`, `FILMCRAFT_CONTROL_TOKEN`, `filmcraft-cli`)**:
  - [FilmCraft Agents, CLI & MCP Guide](https://github.com/storytold/filmcraft/blob/main/docs/agents.md)
  - [FilmCraft Control Protocol](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md)
  - [FilmCraft Captions](https://github.com/storytold/filmcraft/blob/main/docs/captions.md) and [Transcripts](https://github.com/storytold/filmcraft/blob/main/docs/transcripts.md)
  - [FilmCraft Graphics & Titles](https://github.com/storytold/filmcraft/blob/main/docs/graphics.md)
<<<<<<< HEAD
- **DesignCraft, PdfCraft, PhotoCraft, VectorCraft & Headless AniCraft CLI**:
  - [DesignCraft Agents & Script Guide](https://github.com/storytold/designcraft/blob/main/docs/agents.md) and [MCP](https://github.com/storytold/designcraft/blob/main/docs/mcp.md)
  - [PdfCraft Automation & `pdfcraft-cli ui`](https://github.com/storytold/pdfcraft/blob/main/crates/automation/README.md) and [MCP](https://github.com/storytold/pdfcraft/blob/main/docs/mcp.md)
  - [PhotoCraft Control Protocol](https://github.com/storytold/photocraft/blob/main/docs/control-protocol.md) and [MCP](https://github.com/storytold/photocraft/blob/main/docs/mcp.md)
  - [VectorCraft Control Protocol](https://github.com/storytold/vectorcraft/blob/main/docs/control-protocol.md) and [MCP](https://github.com/storytold/vectorcraft/blob/main/docs/mcp.md)
  - [AniCraft Headless CLI (`anicraft-cli`, early alpha — never launch GUI)](https://github.com/storytold/AniCraft)
=======
- **DesignCraft, PhotoCraft & VectorCraft**:
  - [DesignCraft Agents & Script Guide](https://github.com/storytold/designcraft/blob/main/docs/agents.md) and [MCP](https://github.com/storytold/designcraft/blob/main/docs/mcp.md)
  - [PhotoCraft Control Protocol](https://github.com/storytold/photocraft/blob/main/docs/control-protocol.md) and [MCP](https://github.com/storytold/photocraft/blob/main/docs/mcp.md)
  - [VectorCraft Control Protocol](https://github.com/storytold/vectorcraft/blob/main/docs/control-protocol.md) and [MCP](https://github.com/storytold/vectorcraft/blob/main/docs/mcp.md)
>>>>>>> 9055759 (Fix craft-libs d23f5f3 API migration and get Magic Studio compiling and running)
