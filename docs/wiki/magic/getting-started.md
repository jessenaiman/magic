---
title: Getting Started with Magic Studio
description: Create or attach a song project, author DeckCraft storyboard slides, and map tracks into FilmCraft.
---

## 1. Check Installed ArtCraft Apps

Run the built-in suite check to confirm which ArtCraft apps and `<craft>-cli` binaries are available on your machine:

```sh
magic-cli exec app.doctor
```

## 2. Start From a Song Recording (or an Existing Project)

Every Magic Studio project begins with an audio recording so timestamps synchronize storyboard beats, animations, kinetic lyrics, captions, and scene cuts:

```sh
magic-cli new [existing_project] --audio ~/Music/babys-boat.wav --name "Babys Boat"
```

Or launch the desktop window (`magic`), click **+ New** on the Home view, pick your song audio file, and open **`01 Storyboard`**.

## 3. Build & Save Your Storyboard Deck First

In **`01 Storyboard`**, author your visual style, world, character, and scene slides as a native [DeckCraft](https://github.com/storytold/deckcraft) `.deckcraft` or `.pptx` deck (with PDF export for review handouts) and save your slides to disk. Saving Storyboard slides unlocks user-editable **`extensions/scribbles/`** generation before moving into **`02 Animations`**, **`03 Lyrics`**, **`04 Motion`**, and **`05 Studio`** (`magic-cli studio map-tracks`).
