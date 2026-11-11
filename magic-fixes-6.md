# magic-fixes-6 — UI comparison checklist

Reviewed 2026-11-11. Build: `2fb993b`, branch `review/magic-complete-project-2026-11-11`, repository `jessenaiman/magic`.

Compared the live Rust/egui app with all nine images in Drive's Magic/ui-design-reviews folder. Captured 40 screen states at 1600 × 950, 100% UI scale, dark Aurora theme. Initial narrow-window observations were also checked; final evidence uses the wider window. A copied Storybook Moves project supplied real media, character rigs, and a 12-page storyboard. The original project was not edited. Opening the audit copy added another recent-project entry; duplicate catalog cards from that copy are not counted as a product bug.

**Legend:** `[x]` visibly present; `[ ]` correction or missing visual detail; **Unverified** means the current data or disconnected engine prevents a fair verdict. A checked box verifies UI presence, not that an export, AI operation, or Craft integration works end to end.

[Download all 40 screenshots and UI-state records](https://drive.google.com/file/d/179MKmCRrovKfGUJk2jnT10Sf2xWShzWY/view?usp=drivesdk). Filenames below refer to `screens/` within this ZIP. No source project, control token, or credentials are included.

## 00 — Shared shell and component system

Reference: `00-figma-component-system-sheet.jpg`. Evidence: all main screens; `04-animations.png`, `05-lyrics.png`, `07-studio.png`.

- [x] Project pill, five authoring tabs, Craft launcher buttons, active teal states, filmstrip borders, readiness indicators, file badges, rotary dials, sliders, and pill toggles are present.
- [ ] Fix horizontal overflow: inspector buttons, roster controls, XY controls, and long tool rows are cut off even at 1600 px. At the original approximately 941 px window, navigation and the center workspace are substantially cramped. Use wrapping, bounded sizing, or accessible scrolling.
- [ ] Match the component sheet's precision-control presentation: consistent units, visible numeric values and slider handles, and a fully visible two-axis XY pad. Some controls extend beyond the panel, including Point Y and grading slider handles/values.
- [ ] Restore visual hierarchy and readability: denser, smaller labels and repeated toolbar rows compete with the canvas. References give the main content and controls more balanced space.
- [ ] Replace abbreviated letter launchers with the intended recognizable Craft icon treatment, retaining clear accessible labels.

## 01 — Home / song catalog

Reference: `01-home-song-catalog.jpg`. Evidence: `01-home.png`, `01a-home-in-production.png`, `01b-home-ready-to-render.png`, `01c-home-character-rigs.png`.

- [x] Start Here/New Song Project card, search, production/render/character filters, project cards, file badges, Animate/Studio/Open Project actions, and character-rig cards are present.
- [ ] Preserve image aspect ratios in project and rig cards. Characters are visibly compressed vertically in the current thumbnails compared with the reference's cinematic previews.
- [ ] Match the reference's card proportions and spacing; the current project images are shallow banners with substantial unused space below the catalog.
- **Unverified:** The reference's six projects and multiple rendered-film previews are sample content. The available catalog has one original project plus its audit copy; absent sample projects are not missing UI.

## 02 — Project / Craft files

Reference: `02-project-craft-files.jpg`. Evidence: `02-project.png`.

- [x] Song waveform, lyrics/notes, transcript import, grouped Craft files, type/group controls, file search, asset import, Open in Craft actions, stage navigation, and Close apps are present. Additional bridge/interchange controls also exist.
- [ ] Fix crowding around Transcript and the extra “Open Kinetic Lyrics workspace” control so labels and counts stay clearly separated.
- [ ] Match the reference's compact file rows and clearer domain grouping. Current row padding and header spacing reduce how many assets can be reviewed at once.
- [ ] Distinguish connection state from file availability. Studio shows an `.fcproj Live` badge while live inspection reports both FilmCraft and EffectCraft disconnected; a file on disk is not proof of a live session.
- **Unverified:** PhotoCraft/VectorCraft rows below the initial scroll position are not classified as absent merely because EffectCraft files occupy the first screen.

## 03 — Storyboard

Reference: `03-storyboard-3pane-designcraft.jpg`. Evidence: `03-storyboard.png`.

- [x] Three-pane layout, two-up page thumbnails, active spread, readiness lights, file badges, Render Spread, Sync References, Open in DesignCraft, and Export PDF are present.
- [ ] Fit the two-up thumbnails inside their rail. The second card in each pair is horizontally cut off at the current sidebar width.
- [ ] Preserve active-spread aspect ratio and sharpness. The shown page is stretched into a wide, shallow preview and its text is blurry.
- [ ] Improve the active-spread presentation: raw template slot names such as `cover_page_no_left` and `cover_kicker` dominate beneath the preview, rather than the reference's composed artwork/lyric treatment.
- **Unverified:** Storybook Moves uses different storyboard templates from the cave/pterodactyl reference. Its exact artwork, lyrics, and requirement names should not be copied as missing features. Current readiness is 11/12; one scene requirement is red.

## 04 — Animations / character rig

Reference: `04-animations-craft-rig.jpg`. Evidence: `04-animations.png`, `04a-animation-3d.png`, `04b-animation-roto.png`, `04c-animation-keys.png`, `04d-animation-curves.png`, `04e-animation-path-lower.png`.

- [x] Character roster, sprite/2.5D/3D modes, action choices, duration and loop controls, path/depth controls, stage markers, Puppet/3D/Roto inspectors, and Theatre/Keys/Graph Editor tabs are present.
- [ ] Make roster names useful and unambiguous: several different characters appear as `parts (cutout)` rather than their character names.
- [ ] Fix clipped action rows, XY pad/Point Y controls, pin-kind buttons, and right-inspector rows. Lower path controls can be reached by scrolling, so they are not missing.
- [ ] Match the reference's readable animation timeline: the Theatre view currently offers a scrub line and action pills, without the reference's clearly drawn time ruler, channels, and populated keyframe diamonds.
- [ ] Expand the Graph Editor to the designed usable plotting area. The current graph is a small corner widget beside a long influence slider; the reference shows substantial value/speed curves, navigation, and influence handles.
- **Unverified:** The selected character shows rig marks but reports zero puppet pins. Connected bone chains, a populated five-pin mesh, and the illustrated travel path require an appropriately authored rig/path before judging their rendering or manipulation.

## 05 — Lyrics / V3

Reference: `05-lyrics-v3-effectcraft.jpg`. Evidence: `05-lyrics.png`, `05a-lyrics-styles.png`, `05b-lyrics-fx.png`, `05c-lyrics-inspector-lower.png`.

- [x] Lyric Moments rail, stage, filmstrip, transform/skew dials, stretch controls, variable-font selectors, per-character 3D toggle, layer styles, typography effects, expressions, and caption controls are present.
- [ ] Give the cue/audio/lyrics/word timeline a bounded visible area. The large stage and permanently exposed caption/template controls push its lower lanes below the workspace at 1600 × 950.
- [ ] Keep supplementary caption/template controls compact or collapsible so the default workspace retains the reference's stage-plus-timeline balance.
- [ ] Fix horizontal clipping in font choices, FX/expressions actions, and inspector control rows.
- **Unverified:** This copy reports zero timed transcript words and no timed lyric sections. Its moment cards, filmstrip thumbnails, editable lyric selection box, individual word blocks, cue pins, and rendered text preview are consequently empty. Populate timing and render V3 before calling those features missing.

## 06 — Motion / V4

Reference: `06-motion-v4-vectorcraft-effectcraft.jpg`. Evidence: `06-motion.png`, `06a-motion-fx.png`, `06b-motion-track.png`.

- [x] Motion Cues rail, filmstrip stage, Beat/Cue/Motion/Trim lanes, shape/mask controls, FX browser, blend/matte settings, tracking/EXR/OCIO controls, Lottie controls, and Publish V4 are present.
- [ ] Restore the default rail's emphasis on cue cards. Blend modes, mattes, blur, and alpha-queue settings occupy the top of the cue rail rather than remaining compact supplementary inspector controls.
- [ ] Add the reference's legible timeline ruler, playhead/time ticks, cue blocks, and clear track separation to the current thin lane presentation; do not rely on a plain colored bar as the timeline's only orientation.
- [ ] Fix clipped mask operators, blend modes, tracking settings, and long inspector rows.
- **Unverified:** The project reports no authored motion cues/SVGs. The reference's Beat/Flourish/Swoop/Trace thumbnails and composited swirl artwork cannot be verified in this state; their absence is not proof of a missing renderer.

## 07 — Studio / FilmCraft assembly

Reference: `07-studio-filmcraft-assembly.jpg`. Evidence: `07-studio.png`, `070-studio-edit.png` through `075-studio-export.png`, `07a-studio-scopes.png`.

- [x] Scene rail, central preview, Assemble/Preview/Grab Frame/Open in FilmCraft controls, three output cards, Elements Matrix, and Film/V1-V2/V3/V4 lanes are present.
- [ ] Preserve 16:9 in the master preview and output cards. The master background becomes a shallow panorama, and the animation cards/matrix visibly squash the bear vertically.
- [ ] Restore the reference's informative review timeline: time ticks, playhead, scene-filmstrip thumbnails, and audio waveform detail. The current Film lane is an empty perforated strip and the other lanes provide only sparse blocks/labels.
- [ ] Keep all three matrix columns and inspector tabs accessible; the Motion column, Export tab, and other row controls are cut off at the right boundary.
- [ ] Consolidate the two competing tab rows (“Matrix/Edit/Color…” and “Workspace: Assembly/Editing/Color…”). They consume space and communicate two different current workspace states.
- **Unverified:** Only one scene card and two V2 clip labels are shown for this project. The reference's many scene cards and full V3/V4 output cards require populated scene/timing/motion data and rendered outputs. FilmCraft and EffectCraft were not live-connected during the audit.

## 08 — Supplementary controls, sections 6–12

Reference: `08-supplementary-filmcraft-effectcraft-controls-sheet.png`.

### 6 — Dynamic link / interchange / Lottie

- [x] Project has AEP, Lottie, Interchange, and Sync Comps controls; Studio has Sync Comps/Bake controls; Motion has Lottie export controls.
- [ ] Make link format/status and offline/baked state clearly readable near the relevant controls, without horizontal clipping. A “Live” badge must reflect the actual live connection.
- **Unverified:** Actual OTIO/FCPXML/AAF/OMF/EDL round trips, linked alpha renders, Lottie warnings, and template compatibility were not executed.

### 7 — Three-point editing / trim / markers

- [x] Mark In/Out, Insert/Overwrite/Lift/Extract, J/K/L, trim modes, rate stretch, frame-rate choices, and marker CSV controls are visible.
- [ ] Replace the cryptic compact button labels (`I`, `O`, `, Ins`, `. Ovr`, and similar) with the reference's clear labels/tooltips and visible shortcut treatment; fix the clipped trailing controls.
- **Unverified:** Dynamic trim behavior and exact edit results require a connected FilmCraft sequence and an editing test.

### 8 — Color / curves / scopes

- [x] Three grading wheels, RGB and hue/luma curve modes, exposure/temperature controls, look controls, HDR choices, and a scopes panel are present. Lower grading controls are available by scrolling.
- [ ] Match the designed large wheel/curve treatment with visible numeric readouts and editable-point affordances; the current small wheels and simple line give less usable feedback.
- [ ] Keep slider knobs/values inside the inspector; several extend beyond its right edge.
- **Unverified:** Scope traces and color changes were not validated against live rendered pixels.

### 9 — Graph / navigation / expressions

- [x] Graph canvas, interpolation choices, Easy Ease, influence control, expression fields, and Pick-Whip buttons are present.
- [ ] Match the reference's full-size differentiated value/speed graph and visible key navigation; the current small plot does not show equivalent readable detail.
- **Unverified:** Key-specific curve editing and expression application were not exercised.

### 10 — Puppet / 3D / roto / tracking

- [x] Position/Advanced/Bend/Starch/Overlap pin types, live-record/follow/null controls, 3D camera/light controls, roto choices, and tracker controls are present.
- [ ] Wrap or scroll pin types and inspector rows so every option is reachable without being cut off.
- **Unverified:** No puppet mesh, 3D camera solve, or roto result was generated; selected character reports zero puppet pins.

### 11 — Audio / loudness / mixer

- [x] Loudness panel, stream/EBU/ATSC targets, DSP choices, ducking, and Read/Latch/Touch/Write automation choices are present.
- [ ] Match the reference's separate labeled M/S/I/TP meter bars and target markings. Current UI shows one bar with a compact text readout, rather than the four-meter visual hierarchy.
- [ ] Expose the wider console/channel-strip presentation needed for mixer review; current inspector is a compact set of controls.
- **Unverified:** Displayed loudness values, live metering, DSP results, and automation behavior were not measured.

### 12 — Titles / captions / export queue

- [x] Title text/layout controls, graphic templates, caption standards, codec/container choices, burn-in, offline override, background export queue, Render Master Film, and Poll Jobs ETA controls are present.
- [ ] Make all title/export controls fit in the inspector; codec, HDR, caption, and layout rows currently run off its right edge.
- [ ] Replace hard-coded-looking example title text in the current UI (“Pterodactyl Song” while Storybook Moves is open) with project-appropriate initialization or an obvious example placeholder.
- **Unverified:** Render queue progress/results and codec compatibility were not exercised; no export was started.

## Additional accessible screens

Evidence: `08-options.png`, `08a-options-ai.png`, `08b-options-autosave.png`, `08c-options-bridge.png`, `08d-options-general.png`, `09-history.png`, `10-assets-drawer.png`, `11-fidget.png`, `12-scribbles.png`, `13-about-suite-doctor.png`, `14-report-problem.png`, `15-command-search.png`.

- [x] General/Style, AI, AutoSave, and Bridge preferences; history/recovery; asset drawer; Fidget/Scribbles prompt stager; Suite Doctor; Report a Problem; and command-search popup are accessible and captured.
- [x] Fidget and Scribbles currently expose the same shared prompt-staging window; both entry points were captured.
- **Unverified:** These screens have no dedicated reference image in the nine-image set, so they receive a coverage record rather than an invented design-parity verdict. AI was disabled. No generation, external report submission, recovery, or export was triggered. File pickers, every menu-open permutation, alternative themes, all scroll offsets, and external Craft applications are outside this visual inventory.

## Recommended order

1. Fix overflow and aspect-ratio preservation across panels.
2. Restore readable, consistently positioned timelines and graph-editor space.
3. Make engine connectivity and missing-project-data states explicit.
4. Populate a timed-lyrics/motion test project and connect FilmCraft/EffectCraft for a second visual/functional pass.
5. Refine icons, typography, spacing, and precision-control details.

## Reference images

- [00-figma-component-system-sheet.jpg](https://drive.google.com/file/d/1yQ-GZlHS7upfHX5WOFa3LcEzDaD6CccJ/view)
- [01-home-song-catalog.jpg](https://drive.google.com/file/d/1eabCQYmBX_pPJTk409WooMJnNXS4iUxl/view)
- [02-project-craft-files.jpg](https://drive.google.com/file/d/1fiPJa99WEQfqTLlMnqakUSzOM5K0JOQN/view)
- [03-storyboard-3pane-designcraft.jpg](https://drive.google.com/file/d/1cidqHdpzMNBWAVbNV000JWZi6fwTWeF6/view)
- [04-animations-craft-rig.jpg](https://drive.google.com/file/d/1-aPwxLCYYTgVWckTZKc4E98-tVO8aGkt/view)
- [05-lyrics-v3-effectcraft.jpg](https://drive.google.com/file/d/1OkMIQdIerZn-uoUEtD0kFgW0RZyL3wNC/view)
- [06-motion-v4-vectorcraft-effectcraft.jpg](https://drive.google.com/file/d/1XM0VC3sGL2ngdR8xFMfKM-Gpd0Zm9GS2/view)
- [07-studio-filmcraft-assembly.jpg](https://drive.google.com/file/d/1_HbNuJmCK6Yb8coHv9RIziZVabs1Z5CD/view)
- [08-supplementary-filmcraft-effectcraft-controls-sheet.png](https://drive.google.com/file/d/12VB-ssuqkPWdvpjjCHZFJY9SluDI5OKH/view)

## Screenshot inventory

- `01-home.png`
- `01a-home-in-production.png`
- `01b-home-ready-to-render.png`
- `01c-home-character-rigs.png`
- `02-project.png`
- `03-storyboard.png`
- `04-animations.png`
- `04a-animation-3d.png`
- `04b-animation-roto.png`
- `04c-animation-keys.png`
- `04d-animation-curves.png`
- `04e-animation-path-lower.png`
- `05-lyrics.png`
- `05a-lyrics-styles.png`
- `05b-lyrics-fx.png`
- `05c-lyrics-inspector-lower.png`
- `06-motion.png`
- `06a-motion-fx.png`
- `06b-motion-track.png`
- `07-studio.png`
- `070-studio-edit.png`
- `071-studio-color.png`
- `071a-studio-color-lower.png`
- `072-studio-fx.png`
- `073-studio-audio.png`
- `074-studio-titles.png`
- `075-studio-export.png`
- `07a-studio-scopes.png`
- `08-options.png`
- `08a-options-ai.png`
- `08b-options-autosave.png`
- `08c-options-bridge.png`
- `08d-options-general.png`
- `09-history.png`
- `10-assets-drawer.png`
- `11-fidget.png`
- `12-scribbles.png`
- `13-about-suite-doctor.png`
- `14-report-problem.png`
- `15-command-search.png`
