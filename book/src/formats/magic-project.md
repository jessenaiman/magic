# `.magic` Self-Contained Project Format

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

A `.magic` file (`application/vnd.artcraft.magic+json`) is an atomic UTF-8 JSON manifest written via `craft_atomic::atomic_write` at the root of `~/Magic/<slug>/`, with all paths validated through `craft_docpath::resolve_absolute`.

## Self-Contained 5-Folder Layout (`~/Magic/<slug>/`)

```text
~/Magic/<slug>/
├── <slug>.magic               # Project manifest (all relative paths confined inside ~/Magic/<slug>/)
├── storyboard.designcraft     # Modifiable DesignCraft ZIP package (document.json, meta.json, preview.png, assets/)
├── timing-reference.json      # Word/line timestamps in craft-timeline Ticks (254,016,000,000/s)
├── audio/                     # Copied master song audio + stems
├── assets/                    # PhotoCraft (.pcraft), VectorCraft (.vectorcraft/.svg), and 2D/3D character assets
├── effects/                   # EffectCraft compositions (lyrics.ecproj, motion.ecproj, animation.ecproj)
├── film/                      # FilmCraft NLE timeline (<slug>.fcproj, schema_version: 17)
└── exports/                   # Final rendered MP4/MOV/ProRes masters, captions, and OTIO/FCPXML/EDL files
```

When migrating an external legacy folder via `project.migrate`, the original directory is archived once into `~/Magic/<slug>/original-project.zip` (`craft-zip`). Active projects are never stored as `.zip` archives.
