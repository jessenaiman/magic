# `font-preview-service` Reference (`storytold/font-preview-service`)

Upstream repository: [`storytold/font-preview-service`](https://github.com/storytold/font-preview-service).

Lightweight HTTP specimen and codepoint-subset preview service used across the ArtCraft suite alongside `craft-font-catalog`, `craft-font-store` (`aiongg/craft-font-index`), and `craft_font_manager::FontsPanel` so users can preview thousands of remote font families on custom lyric or title sample strings before downloading full font binaries.

## Where Magic Studio Uses `font-preview-service`

- **Fonts Dock Panel & Specimen Previews ([`crates/ui-egui/src/fonts_ui.rs`](../../../crates/ui-egui/src/fonts_ui.rs))**: Configures the preview endpoint on `FontsPanel` (`PanelId::Fonts`) and renders live sample strings for `01 Storyboard`, `03 Lyrics`, `04 Motion`, and `05 Studio` titles.
- **Options Modal ([`crates/ui-egui/src/options_modal.rs`](../../../crates/ui-egui/src/options_modal.rs))**: Exposes `Font Sources` configuration (preview service URL, index cache status, and offline fallback toggle).
