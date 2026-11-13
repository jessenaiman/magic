---
title: Magic Studio Overview
description: Coordinate DesignCraft storyboards, PhotoCraft cutouts, EffectCraft puppet animation & kinetic lyrics, and FilmCraft timelines from one song recording.
---

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

**Magic Studio** (`magic`, `magic-cli`, `magic-mcp`) is a lightweight song-to-video coordinator for the [ArtCraft](https://github.com/storytold) creative suite. Instead of duplicating raster, vector, compositing, or NLE engines, Magic Studio connects your recorded song to dedicated ArtCraft project files so every stage stays editable in its native application.

## How Magic Studio Fits Into ArtCraft

| Stage | ArtCraft App | Output Track / File |
|---|---|---|
| **Song & Captions** | [FilmCraft](https://github.com/storytold/filmcraft) | `A1` Song + Caption Track (`film/<song>.fcproj`, `timing-reference.json`) |
| **Storyboard** | [DesignCraft](https://github.com/storytold/designcraft) | `storyboard.designcraft` (`A-Cover`, `B-World`, `C-Character`, `D-Motion`, Scenes -> `V1`) |
| **Cutouts & SVG** | [PhotoCraft](https://github.com/storytold/photocraft) & [VectorCraft](https://github.com/storytold/vectorcraft) | Isolated character layers (`.pcraft`) and clean SVG silhouettes (`.svg`) |
| **Puppet Animation** | [EffectCraft](https://github.com/storytold/effectcraft) | 11-pin facial & mouth viseme rigs (`rig.ecproj` -> `V2` ProRes 4444 RGBA) |
| **Kinetic Lyrics** | [EffectCraft](https://github.com/storytold/effectcraft) | Timed word/phrase typography (`effects/lyrics.ecproj` -> `V3` ProRes 4444 RGBA) |
| **Action Motion** | [EffectCraft](https://github.com/storytold/effectcraft) | Trim-path vector action cues (`effects/motion.ecproj` -> `V4` ProRes 4444 RGBA) |

## Licensing

You can freely create, distribute, and monetize commercial videos and music videos made with Magic Studio, modify the source code, and share non-commercial forks. Reselling Magic Studio or offering it as a commercial product/service requires a separate written license from Jesse Naiman (`LICENSE`).
