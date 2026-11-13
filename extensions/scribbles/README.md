# Scribbles Extension (`io.magic.scribbles`)

User-editable prompt templates (`templates.json`) and multi-step character/asset workflows
(`steps.json`) for Magic Studio. Edit either JSON file on disk — no Rust recompile required.

## Override Search Order

Magic Studio resolves `scribbles/` in this exact priority order:
1. `<project>/extensions/scribbles/` (per-project workflow override)
2. `$CRAFT_EXTENSIONS_DIR/scribbles/` (shared ArtCraft extensions store)
3. `~/.config/magic/extensions/scribbles/` (or `$XDG_CONFIG_HOME/magic/extensions/scribbles/`)
4. `<repo>/extensions/scribbles/` (bundled fallback defaults)

## Customizing & Storyboard Slide Gate

- Copy `manifest.json`, `templates.json`, and `steps.json` into any higher-priority folder above to
  adjust canvas dimensions, layout rules, `provider_command`, or multi-step pack sequences.
- Random prompt-only requests are refused: compose your prompt and visual references on one
  `DeckCraft` Storyboard slide, export it (`storyboard/slides/slide-01.png`), and pass `"slide"`.
