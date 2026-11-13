# ArtCraft Suite Delegation & Track Isolation

Each creative stage writes native project files and maps cleanly onto dedicated [`FilmCraft`](https://github.com/storytold/filmcraft) tracks (`magic-cli studio map-tracks`) so iterating on one stage never overwrites another:

| Stage | Native Files & Compatible Tools | Live Studio / Host Apps | `FilmCraft` Track |
|---|---|---|---|
| **Song Audio & Captions** | `audio/*` + `film/<slug>.fcproj` (`craft-audio`, `craft-captions`) | [`FilmCraft`](https://github.com/storytold/filmcraft) (`:9876`) | `A1` (Song) + `C1` (Captions) |
| **01 Storyboard & Scribbles** | `<slug>.deckcraft` / `.pptx` + PDF export + `extensions/scribbles/` (`craft-ai-core`) | [`DeckCraft`](https://github.com/storytold/deckcraft) (`:7990`) | `V1` (`Storyboard Scenes`) |
| **02 Animations** | ProRes / H.264 / HEVC / WebM / alpha sequences, `.ecproj`, `.pcraft`, `.svg`, `GLB`/`OBJ`/`FBX` (`craft-mesh`) | Blender, Three.js, After Effects, [`AniCraft`](https://github.com/storytold/anicraft), [`EffectCraft`](https://github.com/storytold/effectcraft), [`PhotoCraft`](https://github.com/storytold/photocraft), [`VectorCraft`](https://github.com/storytold/vectorcraft) | `V2` (`Animations`) |
| **03 Lyrics** | `effects/lyrics.ecproj` + `SRT`/`WebVTT`/`TTML` | Live `03 Lyrics` view + [`EffectCraft`](https://github.com/storytold/effectcraft) (`:9877`) | `V3` (`Kinetic Lyrics`) + `C1` |
| **04 Motion** | `effects/motion.ecproj` (`craft-drawingml` shapes & text warps) | Live `04 Motion` view + [`EffectCraft`](https://github.com/storytold/effectcraft) / [`VectorCraft`](https://github.com/storytold/vectorcraft) | `V4` (`Action Motion`) |
| **05 Studio** | `film/<slug>.fcproj` + `exports/*` (`OTIO`/`FCPXML`/`EDL`/`ALE`) | 2-tab `Live Breakdown` & `Finishing` dock + [`FilmCraft`](https://github.com/storytold/filmcraft) | Master `V1`–`V4` + `C1` + `A1` |
