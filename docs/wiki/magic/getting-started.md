---
title: Getting started
description: Create a song project, complete the 4 Storyboard Main Title Pages, and assemble your first FilmCraft preview.
---

## 1. Check installed ArtCraft apps

Run the built-in suite check to confirm which ArtCraft apps and `<craft>-cli` binaries are available on your machine:

```sh
magic-cli exec app.doctor
```

## 2. Start from a song recording

Every full Magic Studio project begins with an audio recording so timestamps synchronize lyrics, facial visemes, motion cues, and scene cuts:

```sh
magic-cli new \
  --root ~/Videos/Magic \
  --name "Babys Boat" \
  --audio ~/Music/babys-boat.wav \
  --wizard
```

Or launch the desktop window (`magic`), click **New** on the Home view (or choose **File ▸ New Song Project…**), pick your song audio file, and open the **Storyboard** tab.

## 3. Complete the four main title pages first

In **Storyboard**, complete the four master reference spreads until their traffic lights turn green:
1. **A-Cover** — Overall visual style and key art.
2. **B-World** — Master stage/location design.
3. **C-Character** — Lead character sheet for cutout and rigging.
4. **D-Motion** — Song-action graphic cues.

Then click **Bulk** (or run `magic-cli storyboard bulk_generate`) to generate consistent scene artwork across your storyboard spreads before moving into **Lyrics**, **Motion**, **Animation**, and **Studio**.
