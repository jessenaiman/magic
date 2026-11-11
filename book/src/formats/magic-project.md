# `.magic` Project Format

A `.magic` file (`application/vnd.artcraft.magic+json`) is an atomic UTF-8 JSON manifest (`"format": "magic.project"`, `"version": 1`) written via `craft_atomic::atomic_write` at the root of a song workspace folder (with orphaned `.craft_tmp` siblings swept on open).

## Workspace Layout

```text
my-song/
├── My Song.magic              # Coordinator manifest (paths canonicalized via craft-docpath)
├── storyboard.designcraft     # DesignCraft production book + locked *_cli fineprint instructions
├── storyboard/
│   ├── references.json        # Maps placed flat PNG slots back to source .ecproj/.pcraft/.vectorcraft files
│   └── renders/               # Flattened Craft snapshots placed onto storyboard frames
├── timing-reference.json      # Word/line timestamps driving V2 lip-sync, V3 lyrics, and V4 motion
├── transcript.srt             # Synced caption sidecar imported into FilmCraft
├── assets/
│   ├── backgrounds/           # Scene plates for V1
│   └── characters/<id>/       # PhotoCraft cutouts (.pcraft) + EffectCraft puppet rigs (.ecproj)
├── lettering/lyrics.ecproj    # EffectCraft Kinetic Teaching Lyrics composition (exports V3)
├── motion/actions.ecproj      # EffectCraft Song-Action Trim-Path composition (exports V4)
└── film/my-song.fcproj        # Master FilmCraft NLE project (A1 + V1..V4)
```
