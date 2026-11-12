---
title: Production Workflows
description: Storyboard overlays, PhotoCraft subject isolation, EffectCraft puppet lip-sync, kinetic lyrics, and FilmCraft assembly.
---

## Storyboard (`DesignCraft`)

- Inspect spreads and read-only fine-print CLI instructions: `magic-cli --project song.magic storyboard pages page=0`
- Place a flat image or Craft project snapshot onto a slot: `magic-cli --project song.magic storyboard set page=4 label=scene_hero image=assets/hero.png`
- Open live in DesignCraft: click **DesignCraft** in the Storyboard toolbar (`storyboard.open`).

## Subject Cutout & Vector Trace (`PhotoCraft` & `VectorCraft`)

- Remove background with feathered alpha: `magic-cli --project song.magic --save exec photo.remove_background path=assets/fox.png`
- Trace clean vector strokes/silhouettes (`ignoreWhite: true`): `magic-cli --project song.magic --save exec vector.trace path=assets/star.png`

## Facial Expressions & Mouth Lip-Sync (`EffectCraft` Puppet Controls)

In the **Animation** view (or via `magic-cli`), use [EffectCraft's Puppet Controls](https://github.com/storytold/effectcraft/blob/main/docs/agents.md) for expressive faces and mouth movements:

1. Click **Mark**, **Cut**, and **Rig** (`character.mark`, `character.cut`, `character.rig`).
2. Open the **Puppet** drawer and click **Face** (`character.puppet.rig_face`) to install the 11-pin facial mesh.
3. Click **Lipsync** (`character.puppet.lipsync`) to keyframe mouth visemes from `timing-reference.json`, and layer emotion cues (`happy`, `surprised`, `blink`, `wink_L`) with `character.puppet.expression`.
4. Add secondary motion on ears, hair, or tails with `character.puppet.follow`.

## Kinetic Lyrics (`V3`), Action Motion (`V4`) & Studio (`FilmCraft`)

- Author word styling and curves in **Lyrics** (`words.letter` -> `lettering/lyrics.ecproj` -> `V3`).
- Attach SVG trim-path drawings to sung action verbs in **Motion** (`motion.save` + `motion.publish` -> `motion/actions.ecproj` -> `V4`).
- Open **Studio** and click **Assemble**, **Preview**, or **Export** (`video.assemble`, `video.preview`, `video.export`) — or click **FilmCraft** to fine-tune directly in `FilmCraft`.
