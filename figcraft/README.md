# FigCraft Design System (`magic-studio-master.figcraft`)

`figcraft/magic-studio-master.figcraft` is Magic Studio's **single canonical UI design document**.

* **Strict Single-Doc Policy (`cargo xtask layers`)**: Never create additional `.figcraft`, `.fig`, or `.html` UI mockup files anywhere in this repository, and never dump markdown logs. Update canonical pages inside `magic-studio-master.figcraft` only.
* **Canonical Page Index**: `00` Craft UI Tokens · `01` Home · `02` Project · `03` Storyboard (RO `.designcraft` Master + `storyboard.pdf`) · `04` Animations (`V2`) · `05` Lyrics (`V3`) · `06` Motion (`V4`) · `07` Studio (`FilmCraft`) · `08` Supplementary Controls (`T01`–`T42`).

## Driving `FigCraft` via Commands / MCP

1. **Local Archive & Pure-Rust Renderer**: `.figcraft` is an offline ZIP (`document.json`, `images/<sha256>`, `thumbnail.png`, `versions/`) rendered by `tiny-skia`, `kurbo`, `linesweeper`, and `HarfRust`.
2. **CLI & MCP Commands**:
   * Render PNG: `figcraft-cli render figcraft/magic-studio-master.figcraft -o out.png --scale 2`
   * Export SVG: `figcraft-cli export figcraft/magic-studio-master.figcraft -o figcraft/diagrams/ --format svg`
   * Batch script: `figcraft-cli run script.jsonl --out figcraft/magic-studio-master.figcraft`
   * Headless confined MCP: `figcraft-cli mcp --automation-read-root . --automation-write-root figcraft`
   * Live window MCP: start `figcraft --control 47800`, attach with `figcraft-cli mcp --attach 47800`.
