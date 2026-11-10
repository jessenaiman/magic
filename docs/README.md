# Magic Studio Developer & Suite Documentation

Magic Studio uses **two official ArtCraft documentation surfaces** — no ad-hoc markdown logs or paraphrased animation skills are kept in this repository:

1. **Developer & Architecture Book (`book/`)** — Built with [mdBook](https://rust-lang.github.io/mdBook/) following [`storytold/photocraft`](https://github.com/storytold/photocraft) (`mdbook build book`, deployed to GitHub Pages by `.github/workflows/mdbook.yml`).
2. **User & Community Wiki (`docs/wiki/magic/`)** — Starlight-formatted user guides ready for [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki) (`overview.md`, `getting-started.md`, `install-update.md`, `workflows.md`, `troubleshooting.md`).

## Reference Specifications in `docs/`

- **[Control Protocol (`--control`)](control-protocol.md)** — Complete loopback JSON-lines protocol for `magic --control 9876`, `magic-cli --connect`, `magic-cli mcp`, and `EffectCraft`/`FilmCraft` bridges, including the review checklist of enhancement controls.
- **[MCP Server (`magic-cli mcp`)](mcp.md)** — Remote (`--connect 127.0.0.1:9876`) vs Headless (`--headless`) setup, `.mcp.json` configuration, and `--automation-read-root` / `--automation-write-root` confinement.
- **[Architecture & Layer Boundaries](architecture.md)** — Single-responsibility crate hierarchy (`craft-util`, `craft-scribbles`, `magic-engine`, `magic-mcp`, `magic-cli`, `magic`) and `FilmCraft` `V1`–`V4` track isolation.
- **[Feature Gaps & Unimplemented Requests](gaps.md)** — Actionable `- [ ]` checkmark lists grouped by subsystem covering missing features, `// TODO:` host checks, and control enhancements.
- **[Craft-Wiki App Guides](wiki/README.md)** — Drop-in user documentation pages for [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).
- **[House Rules](house-rules-core.md)** — Engineering standards from [`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/docs/house-rules-core.md).
- **[Brand Asset Notice](brand/LICENSE-brand.txt)** — Trademark and visual identity reservation.

## Upstream ArtCraft Suite Documentation

- **[FilmCraft](https://github.com/storytold/filmcraft)** — [Control Protocol](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md) · [CLI & Automation](https://github.com/storytold/filmcraft/blob/main/docs/cli-and-automation.md) · [MCP Server](https://github.com/storytold/filmcraft/blob/main/docs/mcp-server.md) · [Captions & Speech](https://github.com/storytold/filmcraft/blob/main/docs/captions-and-speech.md)
- **[EffectCraft](https://github.com/storytold/effectcraft)** — [Control Protocol](https://github.com/storytold/effectcraft/blob/main/docs/control-protocol.md) · [CLI](https://github.com/storytold/effectcraft/blob/main/docs/cli.md) · [MCP Server](https://github.com/storytold/effectcraft/blob/main/docs/mcp-server.md) · [Animation, Puppet Pins & Expressions](https://github.com/storytold/effectcraft/blob/main/docs/animation-and-expressions.md) · [Text & Presets](https://github.com/storytold/effectcraft/blob/main/docs/text-and-presets.md)
- **[DesignCraft](https://github.com/storytold/designcraft)** — [MCP & CLI](https://github.com/storytold/designcraft/blob/main/docs/mcp.md)
- **[PhotoCraft](https://github.com/storytold/photocraft)** — [README & mdBook](https://github.com/storytold/photocraft/blob/main/README.md) · [Roadmap](https://github.com/storytold/photocraft/blob/main/ROADMAP.md)
- **[VectorCraft](https://github.com/storytold/vectorcraft)** — [README & MCP](https://github.com/storytold/vectorcraft/blob/main/README.md)
- **[AniCraft](https://github.com/storytold/AniCraft)** — Headless `anicraft-cli` frame/sprite export only (early-alpha desktop UI disabled).
