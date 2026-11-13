# Song-to-Video Quickstart

Every Magic Studio project starts from an audio recording (in a fresh folder or an existing project directory) so word timing drives the DeckCraft storyboard, `02 Animations`, `03 Lyrics`, `04 Motion`, and the `05 Studio` timeline.

## 1. Create or Attach a Song Project

```sh
magic-cli new [existing_project]   --audio ~/Music/rendered-yesterday.wav   --name "Rendered Yesterday"
```

This initializes the `.magic` manifest, `timing-reference.json`, `extensions/scribbles/`, the DeckCraft storyboard deck, and `film/rendered-yesterday.fcproj`.

## 2. Save Storyboard Slides in DeckCraft & Run Scribbles

Edit and save your storyboard slides in [`DeckCraft`](https://github.com/storytold/deckcraft) (`.deckcraft` or `.pptx`, with optional PDF export), unlocking `extensions/scribbles/` turnaround and scene generation.

## 3. Add `02 Animations`, `03 Lyrics` & `04 Motion`

Drop renders from Blender, Three.js, After Effects, `AniCraft`, or `EffectCraft` into `02 Animations`, style kinetic lyrics (`03 Lyrics`) and `craft-drawingml` motion cues (`04 Motion`), then map all tracks onto `FilmCraft`:

```sh
PROJ=~/Magic/rendered-yesterday/rendered-yesterday.magic
magic-cli --project "$PROJ" studio map-tracks --save
```

## 4. Review in `05 Studio` (`Live Breakdown` & `Finishing`) & Export

Open **`05 Studio`** to inspect track alignment in **`Live Breakdown`** and grade/export your master video in **`Finishing`** via [`FilmCraft`](https://github.com/storytold/filmcraft).
