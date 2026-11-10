# `storyboard.designcraft` & Fine-Print CLI

`crates/engine/templates/storyboard.designcraft` is a native [`DesignCraft`](https://github.com/storytold/designcraft) archive containing 7 starter spreads across 5 archetypes:

1. `A-Cover` (`page=0`) — Project title, D01 visual style, key art, and song map.
2. `B-World` (`page=1`) — Master location reference (`storyboard.locations`).
3. `C-Character` (`page=2`) — Lead character turnaround, cutout, and puppet rig sheet.
4. `D-Motion` (`page=3`) — Song-action vector cue sheet.
5. `Scene` (`page=4..N`) — Individual lyric/scene spreads populated via `storyboard.bulk_generate` or drag-and-drop.

## Read-Only Fine-Print `magic-cli` Instructions (`*_cli`)

Every spread and editable slot has a companion `*_cli` text frame on Layer 2 (`"CLI Fineprint (Read-Only)"`), styled in `Geist Mono` `5.75pt` (`"CLI Fineprint"` paragraph style) with `locked: true` and `readonly: true`:

- `magic-cli storyboard pages page=0` lists both editable `slots` and the read-only `cli_instructions` array.
- `magic-cli storyboard set` refuses to modify any `*_cli` frame.
- `magic-cli storyboard add archetype=scene` updates the duplicated spread's `*_cli` labels to the new `page=<index>` and re-locks them.
