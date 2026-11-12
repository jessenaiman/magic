# Security Policy

## Loopback Control & Automation Confinement

- `magic` opens no TCP listener unless started explicitly with `--control <port>` or `MAGIC_CONTROL_PORT`.
- The control socket binds strictly to loopback (`127.0.0.1` / `::1`) and validates a 64-hex secret token (`~/.config/magic/control-token`, mode `0600`, or `MAGIC_CONTROL_TOKEN`) in constant time (`subtle::ConstantTimeEq`).
- Automated MCP and `--control` sessions support repeatable `--automation-read-root` and `--automation-write-root` flags that reject symlink escapes and confine reads/writes to explicit directories (see [`docs/mcp.md`](docs/mcp.md)).
- Sibling Craft processes launched via `craft-libs` receive only explicit target tokens (`FILMCRAFT_CONTROL_TOKEN`, `TEXTURECRAFT_CONTROL_TOKEN`, or `--control-token-file`) and never inherit `MAGIC_CONTROL_TOKEN` CLI arguments.

## Reporting a Vulnerability

Please do **not** open public issues for security vulnerabilities. Report vulnerabilities privately through [GitHub Security Advisories](https://github.com/jessenaiman/magic/security/advisories/new).
