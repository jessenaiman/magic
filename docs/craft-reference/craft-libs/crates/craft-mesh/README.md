# `craft-mesh` Reference (`storytold/craft-libs`)

Upstream crate: [`storytold/craft-libs` `crates/mesh`](https://github.com/storytold/craft-libs) (`craft-mesh`).

Pure-Rust 3D scene and polygon/skeletal mesh I/O (`gltf`/`.glb`, `obj`/`.mtl`, `fbx`, `stl`, `ply`, `threemf`) with bounded `Limits`, topology/skin/joint/animation `Scene::validate()`, and lossless `.glb`/`.obj`/`.stl` writers.

## Where Magic Studio Uses `craft-mesh`

- **Mesh Inspection & Format Conversion ([`crates/engine/src/cmd/character/mesh_io.rs`](../../../../crates/engine/src/cmd/character/mesh_io.rs))**: Implements `read_craft_mesh_scene`, `summarize_mesh_scene`, `mesh.inspect`, and `mesh.convert` with `craft-docpath` confined external buffer resolution.
- **3D Character Rigging & `EffectCraft` `advanced3D` Comps ([`crates/engine/src/cmd/character/model3d.rs`](../../../../crates/engine/src/cmd/character/model3d.rs))**: Validates face counts, UV sets, normals, tangents, skeletal skins/joints, and animation clips before normalizing non-glTF meshes to `.glb` for `EffectCraft` `layer.newModel`.
- **CLI & UI ([`apps/magic-cli/src/main.rs`](../../../../apps/magic-cli/src/main.rs), [`crates/ui-egui/src/views/animation_inspector.rs`](../../../../crates/ui-egui/src/views/animation_inspector.rs))**: Exposes `magic-cli mesh inspect|convert` and live 3D topology cards in `02 Animations`.
