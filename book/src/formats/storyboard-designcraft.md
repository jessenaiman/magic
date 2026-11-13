# `storyboard.designcraft` Package

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

`crates/engine/templates/storyboard.designcraft` (seeded on first launch into `~/.local/share/magic/templates/storyboard.designcraft`) is a native [`DesignCraft`](https://github.com/storytold/designcraft/blob/main/README.md) v2 ZIP package (`mimetype`, `meta.json`, `preview.png`, `document.json`, and `assets/`). Consult [`designcraft/docs/architecture.md`](https://github.com/storytold/designcraft/blob/main/docs/architecture.md) and [`designcraft/docs/mcp.md`](https://github.com/storytold/designcraft/blob/main/docs/mcp.md) in `storytold/designcraft` for the canonical format and MCP `--automation-write-root` rules.

- `storyboard.new` seeds `<project>/storyboard.designcraft` from `project.storyboard_template` or `~/.local/share/magic/templates/storyboard.designcraft`.
- `storyboard.set` and `storyboard.wizard` update unlocked story `text` (`utf8` byte run length + `fontFamily` + `rev`) and bundle placed slot artwork under `assets/*` directly inside `storyboard.designcraft` via `craft-zip` + `craft-atomic`.
- `storyboard.add`, `storyboard.remove`, and `storyboard.thumbnail` delegate spread insertion, deletion, and rendering to host `DesignCraft` (`designcraft --control 7979` or `designcraft-cli`).
