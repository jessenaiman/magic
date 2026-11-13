# Security Policy

> **Last Reviewed & Updated:** 2026-11-12T22:48:11-05:00 · **Reviewed by:** Jesse A. Naiman

## Loopback Control & Automation Confinement

Magic Studio follows the loopback control and confinement model documented in [`storytold/designcraft/docs/control-protocol.md`](https://github.com/storytold/designcraft/blob/main/docs/control-protocol.md) and [`storytold/designcraft/docs/mcp.md`](https://github.com/storytold/designcraft/blob/main/docs/mcp.md):

- `magic` opens no TCP listener unless started explicitly with `--control <port>` or `MAGIC_CONTROL_PORT=<port>`.
- The control listener binds strictly to `127.0.0.1` (`Ipv4Addr::LOCALHOST`), serves up to 16 concurrent clients with a `16 MiB` line cap, has no authentication on loopback (enable `--control` only while actively automating), and closes a connection immediately on malformed JSON lines.
- Headless `magic-cli` and `magic-cli mcp` sessions support repeatable `--automation-read-root <DIR>` and `--automation-write-root <DIR>` flags resolved via `std::fs::canonicalize` so file access stays confined to explicit directories (see [`docs/mcp.md`](docs/mcp.md)).
- Host Craft applications invoked over `--control` receive only their own documented environment variables (such as `FILMCRAFT_CONTROL_TOKEN` when talking to [`FilmCraft`](https://github.com/storytold/filmcraft/blob/main/docs/control-protocol.md)).

## Reporting a Vulnerability

Please do **not** open public issues for security vulnerabilities. Report vulnerabilities privately through [GitHub Security Advisories](https://github.com/jessenaiman/magic/security/advisories/new).
