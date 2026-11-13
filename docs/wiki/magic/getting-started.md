---
title: Getting Started with Magic Studio
description: Create a song project, complete the 4 Storyboard Main Title Pages, and assemble your first FilmCraft preview.
---

> **Last Reviewed & Updated:** 2026-11-12T16:59:00-05:00 · **Reviewed by:** Jesse A. Naiman

## 1. Check Installed ArtCraft Apps

Run the built-in suite check to confirm which ArtCraft apps and `<craft>-cli` binaries are available on your machine:

```sh
magic-cli exec app.doctor
```

## 2. Start From a Song Recording

Every full Magic Studio project begins with an audio recording so timestamps synchronize lyrics, facial visemes, motion cues, and scene cuts:

```sh
magic-cli new   --root ~/Videos/Magic   --name "Babys Boat"   --audio ~/Music/babys-boat.wav   --wizard
```

Or launch the desktop window (`magic`), click **+ New** on the Home view, pick your song audio file, and open the **Storyboard** tab.

## 3. Complete the Four Main Title Pages First

In **Storyboard**, complete the four master reference spreads until their traffic lights turn green:
1. **A-Cover** — Overall visual style and key art.
2. **B-World** — Master stage/location design.
3. **C-Character** — Lead character sheet for cutout and rigging.
4. **D-Motion** — Song-action graphic cues.

Then click **Bulk** (or run `magic-cli storyboard bulk_generate`) to generate consistent scene artwork across your storyboard spreads before moving into **Lyrics**, **Motion**, **Animation**, and **Studio**.
