# Automation Overview

> **Last Reviewed & Updated:** 2026-11-12T22:48:11-05:00 · **Reviewed by:** Jesse A. Naiman

Magic Studio supports three coordinated automation surfaces over the same `magic_engine::Session` command registry:

1. **[`magic-cli`](cli.md)** — Direct headless CLI (`--project <file.magic> [--save]`) or live window client (`--connect 127.0.0.1:9875`).
2. **[`--control` Protocol](control-protocol.md)** — Loopback JSON-lines socket (`magic --control 9875`) for live UI watching, inspection, synthetic pointer/key input, screenshots, and sibling `EffectCraft`/`FilmCraft` bridges.
3. **[`magic-cli mcp`](mcp.md)** — Stdio JSON-RPC 2.0 MCP server supporting both isolated headless execution (`magic-cli mcp`) and live desktop watching (`magic-cli mcp --connect 9875`).
