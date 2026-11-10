# Tauri Base Control & Styling System

Magic Studio's Tauri frontend (`apps/magic/ui/`) provides a unified **Base Control & Styling System** (`apps/magic/ui/components/controls/` + `apps/magic/ui/base-controls.css`) so every toolbar, asset drawer, rail, and inspector shares deterministic alignment and spacing.

## Core Rules

1. **Shared Control Primitives**: Interactive toolbars and panels compose `<Toolbar>`, `<ControlGroup>`, `<Button>`, `<IconButton>`, `<Segmented>`, `<SelectControl>`, `<InputControl>`, `<Slider>`, `<Dial>`, and `<Toggle>` from `apps/magic/ui/components/controls`.
2. **Identical Baseline & Responsive Height Tokens**: Every control size (`--ctrl-h-xs`, `--ctrl-h-sm`, `--ctrl-h-md`) enforces `margin: 0`, `box-sizing: border-box`, `line-height: 1`, and `inline-flex` vertical centering so primary buttons, ghost buttons, icon buttons, and `<select>` dropdowns never shift vertically relative to neighbors.
3. **Icon + Single-Word Labels**: Interactive buttons and tabs pair a 14px stroke-aligned `<Icon>` with **at most one word** (plus an optional numeric `badge` chip), keeping full context in `title` / `<Tooltip>` and `aria-label`.
4. **Stable Automation IDs**: Every interactive control carries a deterministic `id` discoverable via `magic-cli --connect 127.0.0.1:9876 ui elements`.
