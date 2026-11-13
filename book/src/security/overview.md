# Confinement & Control Security

> **Last Reviewed & Updated:** 2026-11-12T22:48:11-05:00 · **Reviewed by:** Jesse A. Naiman

## File Access Confinement (`AutomationRoots`)

`crates/engine/src/confinement.rs` implements `--automation-read-root` and `--automation-write-root` matching [`storytold/designcraft/docs/mcp.md`](https://github.com/storytold/designcraft/blob/main/docs/mcp.md):

- Configured roots and target paths (or nearest existing ancestors for new output files) are resolved via `std::fs::canonicalize` so reads and writes stay confined inside the allowed directories.
- Read roots govern `file.open`, `project.open`, `file.list`, `audio`, `transcript`, `image`, `source_project`, and `references`.
- Write roots govern `file.new`, `file.save`, `file.saveAs`, `storyboard.export`, `video.export`, and `ui.screenshot {path}`.

## Loopback `--control` Socket Security

`apps/magic/src/control.rs` matches [`storytold/designcraft/docs/control-protocol.md`](https://github.com/storytold/designcraft/blob/main/docs/control-protocol.md):
- Opt-in loopback-only binding (`127.0.0.1` on port `1..=65535`, or `0` for an OS-assigned port) via `--control <port>` or `MAGIC_CONTROL_PORT`.
- Unauthenticated on `127.0.0.1` (enable `--control` only while actively automating), serving at most 16 concurrent clients with a `16 MiB` line limit.
- Immediate connection termination (`"...; closing the connection"`) on the first invalid JSON or oversized line.
