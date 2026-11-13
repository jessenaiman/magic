---
title: Production Workflows
description: DeckCraft storyboards, extensions/scribbles, multi-tool 02 Animations, kinetic lyrics, motion cues, and FilmCraft studio mapping.
---

## `01 Storyboard` (`DeckCraft`) & `extensions/scribbles/`

- Author your storyboard in [DeckCraft](https://github.com/storytold/deckcraft) (`.deckcraft` or `.pptx`) with themes, shapes, speaker notes, and PDF handout export (`deckcraft --control 7990` or `deckcraft-cli`).
- Customize project prompt templates in `extensions/scribbles/` (backed by `craft-ai-core`). Once your Storyboard slides are saved on disk, run scribble turnarounds and scene plates with consistent visual references.

## `02 Animations` (Open Multi-Tool `V2` Lane)

- Import character and scene renders created in **Blender**, **Three.js**, **After Effects**, **[AniCraft](https://github.com/storytold/anicraft)**, or **[EffectCraft](https://github.com/storytold/effectcraft)** in any video/alpha format supported by [FilmCraft](https://github.com/storytold/filmcraft).
- Isolate cutouts in [PhotoCraft](https://github.com/storytold/photocraft) (`photo.remove_background`), trace clean contours in [VectorCraft](https://github.com/storytold/vectorcraft) (`vector.trace`), inspect 3D rigs (`GLB`/`glTF`/`OBJ`/`FBX`/`STL`) via `craft-mesh`, or animate puppet pins and visemes in [EffectCraft](https://github.com/storytold/effectcraft).

## `03 Lyrics` (`V3` + `C1`), `04 Motion` (`V4`) & `05 Studio` (`FilmCraft`)

- Style word-timed typography and broadcast captions in **`03 Lyrics`** (`effects/lyrics.ecproj` -> `V3` + `C1`).
- Author `craft-drawingml` shapes, text warps, and vector motion overlays in **`04 Motion`** (`effects/motion.ecproj` -> `V4`).
- Reconcile every stage onto `FilmCraft` tracks in one command:
  ```sh
  magic-cli --project song.magic studio map-tracks --save
  ```
- Open **`05 Studio`** to switch between **`Live Breakdown`** (track mapping, beat alignment, `OTIO`/`FCPXML`/`EDL`/`ALE` interchange) and **`Finishing`** (linear-light color wheels, waveform/vectorscope scopes, `rmsDbfs`/`peakDbfs` meters, and master `FilmCraft` export).
