# ArtCraft Suite Delegation & Track Isolation

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

Each creative domain writes its own native ArtCraft project file and publishes onto a dedicated [`FilmCraft`](https://github.com/storytold/filmcraft) track so editing one domain never breaks or overwrites another:

| Domain | Native Project / Tool | Live Studio UI | `FilmCraft` Track |
|---|---|---|---|
| **Song Audio & Captions** | `craft-audio` + `film/<song>.fcproj` | [`FilmCraft`](https://github.com/storytold/filmcraft) | `A1` (Song) + Caption Track |
| **Storyboard & Plates** | `storyboard.designcraft` | [`DesignCraft`](https://github.com/storytold/designcraft) | `V1` (`Storyboard Scenes`) |
| **Subject Cutout & SVG** | `.pcraft` / `.svg` | [`PhotoCraft`](https://github.com/storytold/photocraft), [`VectorCraft`](https://github.com/storytold/vectorcraft) | Feeds `V1`, `V2`, `V4` |
| **Character Puppet & Sprite** | `assets/characters/<id>/<view>/rig.ecproj` | [`PhotoCraft`](https://github.com/storytold/photocraft) + [`EffectCraft`](https://github.com/storytold/effectcraft) | `V2` (`Character Animation`, ProRes 4444 RGBA) |
| **Kinetic Teaching Lyrics** | `effects/lyrics.ecproj` | [`EffectCraft`](https://github.com/storytold/effectcraft) | `V3` (`Kinetic Lyrics`, ProRes 4444 RGBA) |
| **Song-Action Motion** | `effects/motion.ecproj` | [`EffectCraft`](https://github.com/storytold/effectcraft) + [`VectorCraft`](https://github.com/storytold/vectorcraft) | `V4` (`Action Motion`, ProRes 4444 RGBA) |
