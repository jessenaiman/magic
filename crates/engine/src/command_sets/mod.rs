//! Modular command registry (`command_sets`) split by Single Responsibility domain:
//! - [`project`]: Project lifecycle (`project.*`, `file.*`), assets, workshop, audio transcription, and `DesignCraft` Storyboard.
//! - [`creative`]: Kinetic Lyrics (`words.*` -> V3/C1), Song-Action Motion (`motion.*` -> V4), Character Animation (`character.*` -> V2), and `FilmCraft` (`video.*`).
//! - [`system`]: UI inspection/controls, Craft AI contract (`ai.*`), diagnostics, and preferences.

pub mod creative;
pub(crate) mod film;
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
            | b"workshop.split_panels"
            | b"video.frame"
            | b"video.export"
            | b"video.preview"
            | b"character.cut"
            | b"character.rig"
            | b"character.frame"
            | b"character.render"
            | b"character.sprite"
            | b"character.shot"
            | b"character.set"
            | b"animation.promote"
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
