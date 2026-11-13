# DeckCraft Storyboards & `extensions/scribbles/`

Magic Studio authors creative storyboards in **`01 Storyboard`** as native [`DeckCraft`](https://github.com/storytold/deckcraft/blob/main/README.md) slide decks (`.deckcraft` and PowerPoint `.pptx`), with full PDF export (slides, notes pages, and handouts). Consult [`storytold/deckcraft/README.md`](https://github.com/storytold/deckcraft/blob/main/README.md) and [`storytold/deckcraft/docs/mcp.md`](https://github.com/storytold/deckcraft/blob/main/docs/mcp.md) for DeckCraft's slide commands and MCP server.

- **DeckCraft Authoring (`01 Storyboard`)**: Build title, style, world, character, and scene slides using DeckCraft themes, layouts, 150+ preset shapes, speaker notes, and transitions (`deckcraft --control 7990` or `deckcraft-cli`), and export review PDFs or PNG slide plates into the project.
- **User-Editable `extensions/scribbles/` (`craft-ai-core`)**: Each project includes plain, user-editable scribble templates and prompt specs under `extensions/scribbles/` backed by `craft-ai-core` ([`storytold/craft-libs/docs/ai-shared-layer.md`](https://github.com/storytold/craft-libs/blob/main/docs/ai-shared-layer.md)).
- **Saved-Storyboard-Slide Gate**: Scribble turnaround and scene generation require saved Storyboard slides on disk first, ensuring downstream artwork always anchors to saved slide structure and speaker notes.
