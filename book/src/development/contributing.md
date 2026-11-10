# Contributing & House Rules

Magic Studio follows [`storytold/craft-contrib`](https://github.com/storytold/craft-contrib/blob/main/CONTRIBUTING.md) and [`House Rules`](https://github.com/storytold/craft-contrib/blob/main/docs/house-rules-core.md):

- **Single Responsibility & `< 900` Lines**: Keep every `.rs`, `.ts`, and `.tsx` file focused on one concern and under 900 lines.
- **Delegate to ArtCraft CLIs**: Never reimplement pixel matting, SVG tracing, puppet deformation, or NLE timeline rendering in Magic; invoke `photocraft-cli`, `vectorcraft-cli`, `designcraft-cli`, `effectcraft-cli`, and `filmcraft-cli` through `CraftHub`.
- **No Paraphrased Skill Dumps**: Do not add custom animation how-to markdowns or agent log files to the repo; link to upstream ArtCraft documentation and record missing host checks with `// TODO:` comments (never `todo!()`).
- **Zero AI Attribution Trailers**: Never add `Co-Authored-By` trailers or `Generated with ...` footers to commits, PRs, or issues.
