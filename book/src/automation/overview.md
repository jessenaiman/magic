# Automation Overview

Magic Studio supports three coordinated automation surfaces over the same `magic::Session` command registry:

1. **[`magic-cli`](cli.md)** — Direct headless CLI (`--project <file.magic> [--save]`) or live window client (`--connect 127.0.0.1:9876`).
2. **[`--control` Protocol](control-protocol.md)** — Loopback JSON-lines socket (`magic --control 9876`) for live UI watching, inspection, synthetic pointer/key input, screenshots, and sibling `EffectCraft`/`FilmCraft` bridges.
3. **[`magic-cli mcp` (`magic-mcp`)](mcp.md)** — Stdio JSON-RPC 2.0 MCP server supporting both live `--connect 127.0.0.1:9876` and confined `--headless` sessions.
