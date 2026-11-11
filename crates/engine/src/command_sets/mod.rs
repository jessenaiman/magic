//! Modular command registry (`command_sets`) split by Single Responsibility domain:
//! - [`project`]: Project lifecycle (`project.*` & `file.*`), assets, workshop, audio transcription (`timing-reference.json`), and Storyboard CRUD/wizard/bulk.
//! - [`creative`]: Kinetic Lyrics (`words.*` -> V3), Song-Action Motion (`motion.*` -> V4), Character Animation (`character.*` -> V2), `PhotoCraft` (`photo.*`), `VectorCraft` (`vector.*`), Fidget/Scribbles (`fidget.*`, `scribbles.*`), and `FilmCraft` (`video.*`).
//! - [`system`]: UI inspection/controls, Craft CLI pass-throughs, `craft-ai-integrations` (`ai.*`), diagnostics, and settings.

pub mod creative;
pub mod project;
pub mod system;

use crate::commands::CommandSpec;

pub(crate) const fn is_long_running_id(id: &str) -> bool {
    matches!(
        id.as_bytes(),
        b"project.new"
            | b"file.new"
            | b"project.intake"
            | b"ui.eval"
            | b"words.design"
            | b"words.letter"
            | b"motion.publish"
            | b"video.assemble"
            | b"audio.analyze"
            | b"storyboard.bulk_generate"
            | b"workshop.split_panels"
            | b"fidget.stage"
            | b"image.generate"
            | b"photo.remove_background"
            | b"vector.trace"
            | b"anicraft.exec"
            | b"video.frame"
            | b"video.export"
            | b"video.preview"
            | b"scribbles.generate"
            | b"scribbles.new"
            | b"scribbles.image"
            | b"scribbles.sheet"
            | b"scribbles.poses"
            | b"scribbles.poses_2x2"
            | b"scribbles.parts_chroma"
            | b"scribbles.moppet"
            | b"scribbles.actions"
            | b"scribbles.layers"
            | b"scribbles.custom"
            | b"scribbles.svg"
            | b"scribbles.reassemble"
            | b"character.mark"
            | b"character.cut"
            | b"character.rig"
            | b"character.action"
            | b"character.frame"
            | b"character.render"
            | b"character.sprite"
            | b"character.shot"
            | b"character.set"
            | b"character.model3d.breakout"
            | b"character.model3d.rig"
            | b"character.model3d.animate"
            | b"animation.prepare_motion"
    )
}

macro_rules! cmd_spec {
    ($id:literal, $label:literal, [$($m:literal),*], $sc:expr, $params:literal, $en:expr, $run:expr) => {
        crate::commands::CommandSpec {
            id: $id,
            label: $label,
            menu: &[$($m),*],
            shortcut: $sc,
            params: $params,
            help: "",
            example: "",
            enabled: $en,
            run: $run,
            journal: true,
            long_running: crate::command_sets::is_long_running_id($id),
        }
    };
}

macro_rules! query_spec {
    ($id:literal, $label:literal, $params:literal, $run:expr) => {
        crate::commands::CommandSpec {
            id: $id,
            label: $label,
            menu: &[],
            shortcut: None,
            params: $params,
            help: "",
            example: "",
            enabled: crate::commands::always,
            run: $run,
            journal: false,
            long_running: false,
        }
    };
}

pub(crate) use {cmd_spec, query_spec};

pub fn build() -> Vec<CommandSpec> {
    let mut specs = Vec::with_capacity(155);
    specs.extend(system::specs());
    specs.extend(project::specs());
    specs.extend(creative::specs());
    specs
}
