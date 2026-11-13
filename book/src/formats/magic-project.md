# `.magic` Project Format

A `.magic` file (`application/vnd.artcraft.magic+json`) is an atomic UTF-8 JSON manifest written via `craft_atomic::atomic_write` inside the project directory, with relative paths validated through `craft_docpath`.

## Project Layout (`magic-cli new [existing_project] --audio song.wav --name "..."`)

```text
<project>/
├── <slug>.magic               # Project manifest
├── <slug>.deckcraft           # Native DeckCraft storyboard slide deck (.deckcraft / .pptx + PDF export)
├── timing-reference.json      # Word/line timestamps in craft-timeline Ticks (254,016,000,000/s)
├── extensions/
│   └── scribbles/             # User-editable turnaround & scene recipes (craft-ai-core; gated on saved Storyboard slides)
├── audio/                     # Master song audio + stems
├── assets/                    # Multi-tool 2D/3D assets, PhotoCraft (.pcraft), VectorCraft (.svg), Blender/AniCraft renders
├── effects/                   # EffectCraft compositions (lyrics.ecproj, motion.ecproj, animation.ecproj)
├── film/                      # FilmCraft NLE timeline (<slug>.fcproj)
└── exports/                   # Final rendered MP4/MOV/ProRes masters, PDFs, captions, and OTIO/FCPXML/EDL files
```
