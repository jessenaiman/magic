# `magic-scribbles` (`crates/scribbles`, Layer 4)

Deterministic prompt templates, anatomical cutout part catalogs, and sprite-sheet grid geometry used when planning storyboard frames and character turnaround sheets. Zero network, UI, or subprocess dependencies.

## Where & Why to Explore Code

| Module | Related Engine Consumer | Why You Explore This File |
| :--- | :--- | :--- |
| **[`src/lib.rs`](src/lib.rs)** | [`crates/engine/src/cmd/character/mark.rs`](../engine/src/cmd/character/mark.rs) | Crate root exposing deterministic character landmark definitions, part bounding boxes, and prompt helpers. |
| **[`src/sheet.rs`](src/sheet.rs)** | [`crates/engine/src/cmd/storyboard/workflow.rs`](../engine/src/cmd/storyboard/workflow.rs) · [`crates/engine/src/cmd/character/sprite.rs`](../engine/src/cmd/character/sprite.rs) | Explore when refining turnaround sheet layouts, structured storyboard frame prompts, anatomical part bounding boxes, and pixel-exact sprite-atlas grid slicing. |
