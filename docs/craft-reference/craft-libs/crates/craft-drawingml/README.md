# `craft-drawingml` Reference (`storytold/craft-libs`)

Upstream crate: [`storytold/craft-libs` `crates/drawingml`](https://github.com/storytold/craft-libs) (`craft-drawingml`).

Shared DrawingML (ECMA-376 Part 1, Chapter 20) model, preset geometry builder, custom geometry evaluator, preset text warps, and theme color transformations used across presentations, documents, spreadsheets, and Magic Studio's **`04 Motion`** stage ([`crates/engine/src/cmd/motion/mod.rs`](../../../../crates/engine/src/cmd/motion/mod.rs), [`crates/ui-egui/src/views/motion.rs`](../../../../crates/ui-egui/src/views/motion.rs)).

## Key Modules Used in Magic Studio

- **Root (`Xfrm`, `emu_to_pt`, `pt_to_emu`, `group_child_affine`)**: Converts between EMU (`12_700` per pt) and `f64` points and builds 2D affine transforms.
- **`color` (`Rgba`, `SchemeSlot`, `ColorScheme`, `ColorTransform`)**: Resolves 12-slot theme schemes (`dk1`, `lt1`, `dk2`, `lt2`, `accent1`..`accent6`, `hlink`, `folHlink` + `bg1`/`tx1` aliases) and luminance/tint/alpha transforms.
- **`preset` (`build(name, w, h, adjusts)`, `CATALOG`)**: Evaluates ECMA-376 preset shapes (`roundRect`, `rightArrow`, `star5`, `callout1`, etc.) into fill/stroke sub-paths, text boxes, adjust handles, and connection sites.
- **`custgeom` (`read_cust_geom`, `write_cust_geom`, `CustomPath`, `Guides`)**: Evaluates custom `a:custGeom` paths and `a:gd` guide formulas.
- **`warp` (`warp(name, adjusts, rect)`)**: Maps laid-out kinetic text lines onto preset text warps (`textArchUp`, `textWave1`, `textInflate`, etc.).
