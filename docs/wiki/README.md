# Magic Studio — `storytold/craft-wiki` App Guides

The 5 Starlight Markdown pages in [`docs/wiki/magic/`](magic/) follow the 5-page app guide contract of [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki) (`src/content/docs/apps/<slug>/`):

- [`magic/overview.md`](magic/overview.md) — What Magic Studio is, how it coordinates the ArtCraft suite, and commercial video vs non-resale licensing.
- [`magic/getting-started.md`](magic/getting-started.md) — Creating or attaching a song project (`magic-cli new [existing_project] --audio`), DeckCraft storyboard slides, and Studio preview.
- [`magic/install-update.md`](magic/install-update.md) — Installing `magic`, `magic-cli`, and `magic-tui` (`./install.sh --from-source`), plus registering `magic-cli mcp` and `--control 9875`.
- [`magic/workflows.md`](magic/workflows.md) — DeckCraft storyboard & `extensions/scribbles/`, open multi-tool `02 Animations`, `03 Lyrics`, `04 Motion`, and `05 Studio` (`Live Breakdown` & `Finishing`).
- [`magic/troubleshooting.md`](magic/troubleshooting.md) — `app.doctor`, `--control` connection checks, `AutomationRoots` errors, and the saved-Storyboard-slide gate.

## Publishing Into `storytold/craft-wiki`

1. Copy `docs/wiki/magic/` to `src/content/docs/apps/magic/` in a checkout of [`storytold/craft-wiki`](https://github.com/storytold/craft-wiki).
2. Add the `magic` entry to `src/data/apps.mjs`, `astro.config.mjs` (sidebar), `public/admin/config.yml` (Decap CMS collection), and `src/content/docs/index.mdx`:

```js
// src/data/apps.mjs in storytold/craft-wiki
{
  slug: "magic",
  name: "Magic Studio",
  tagline: "Song-to-video storyboard, multi-tool animation, kinetic lyrics, and FilmCraft studio",
  repo: "https://github.com/jessenaiman/magic",
  cli: "magic-cli",
  controlPort: 9875,
}
```
