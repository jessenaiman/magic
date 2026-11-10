# Confinement & Control Security

## File Access Confinement (`AutomationRoots`)

`crates/engine/src/confinement.rs` implements `--automation-read-root` and `--automation-write-root` matching [`DesignCraft` MCP](https://github.com/storytold/designcraft/blob/main/docs/mcp.md) and [`VectorCraft`](https://github.com/storytold/vectorcraft):

- Every path is normalized lexically and checked component-by-component with `std::fs::symlink_metadata` so symbolic links escaping a confined root are rejected before any read or write.
- Read roots govern `file.open`, `project.open`, `file.list`, `audio`, `transcript`, `image`, `source_project`, and `references`.
- Write roots govern `file.new`, `file.save`, `file.saveAs`, `storyboard.export`, `video.export`, and `ui.screenshot {path}`.

## Loopback `--control` Socket Security

`apps/magic/src/control.rs` enforces:
- Loopback-only binding (`127.0.0.1` or `::1`) on port `1..=65535` (opt-in via `--control` or `MAGIC_CONTROL_PORT`).
- Constant-time 64-hex token comparison (`subtle::ConstantTimeEq`) against `~/.config/magic/control-token` (`0600`) or `MAGIC_CONTROL_TOKEN`.
- Immediate connection termination (`"...; closing the connection"`) on the first invalid JSON, oversized (`> 4 MiB`), HTTP-smuggled, or unauthorized line (`filmcraft#119`).
