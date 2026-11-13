---
title: Magic Studio Overview
description: Coordinate DeckCraft storyboards, open multi-tool animations, kinetic lyrics, motion graphics, and FilmCraft finishing from one song recording.
---

**Magic Studio** (`magic`, `magic-cli`, `magic-tui`) is a pure-Rust song-to-video coordinator for the [ArtCraft](https://github.com/storytold) creative suite. Instead of duplicating slide, raster, vector, compositing, or NLE engines, Magic Studio connects your recorded song to native project files so every stage stays open and editable.

## How Magic Studio Fits Into ArtCraft

| Stage | Tools & Formats | Output Track / File |
|---|---|---|
| **Song & Captions** | [FilmCraft](https://github.com/storytold/filmcraft) + `craft-audio` + `craft-captions` | `A1` Song + `C1` Captions (`film/<slug>.fcproj`, `timing-reference.json`) |
| **01 Storyboard & Scribbles** | [DeckCraft](https://github.com/storytold/deckcraft) (`.deckcraft` / `.pptx` + PDF) + `extensions/scribbles/` (`craft-ai-core`) | Saved Storyboard slides & scene plates -> `V1` |
| **02 Animations** | Blender, Three.js, After Effects, [AniCraft](https://github.com/storytold/anicraft), [EffectCraft](https://github.com/storytold/effectcraft), [PhotoCraft](https://github.com/storytold/photocraft), [VectorCraft](https://github.com/storytold/vectorcraft) | Any clip/sequence recognized by `FilmCraft` + 3D meshes (`craft-mesh`) -> `V2` |
| **03 Lyrics** | [EffectCraft](https://github.com/storytold/effectcraft) + `craft-font-manager` + `craft-captions` | Timed word/phrase typography (`effects/lyrics.ecproj`) -> `V3` + `C1` |
| **04 Motion** | [EffectCraft](https://github.com/storytold/effectcraft) + `craft-drawingml` + [VectorCraft](https://github.com/storytold/vectorcraft) | Vector shapes, text warps & action cues (`effects/motion.ecproj`) -> `V4` |
| **05 Studio** | [FilmCraft](https://github.com/storytold/filmcraft) (`Live Breakdown` & `Finishing` dock) | Master `MP4`/`MOV`/ProRes exports & `OTIO`/`FCPXML`/`EDL`/`ALE` interchange |

## Licensing

You can freely create, distribute, and monetize commercial videos and music videos made with Magic Studio, modify the source code, and share non-commercial forks. Reselling Magic Studio or offering it as a commercial product/service requires a separate written license from Jesse Naiman (`LICENSE`).
