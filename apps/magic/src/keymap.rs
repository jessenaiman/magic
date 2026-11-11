//! Keyboard shortcuts through [`craft_keymap`] and [`craft_keymap::egui::Dispatcher`].

use std::sync::OnceLock;

use craft_keymap::egui::Dispatcher;
use craft_keymap::{Active, Bindable, Binding, Chord, CommandRef, Keymap, Lookup, Platform, Scope, Scopes};

pub fn catalog_keymap() -> &'static Keymap {
    static KEYMAP: OnceLock<Keymap> = OnceLock::new();
    KEYMAP.get_or_init(|| {
        let catalog = magic::command_specs()
            .iter()
            .filter_map(|spec| {
                let binding = Binding::parse(spec.shortcut?).ok()?;
                Some(Bindable::new(spec.id, spec.label, Scope::app()).with_default(binding))
            })
            .collect();
        Keymap::new(catalog, Scopes::new(), Platform::current())
    })
}

pub fn dispatch_frame_keys(ctx: &egui::Context, dispatcher: &mut Dispatcher, modal_open: bool) -> Vec<CommandRef> {
    if modal_open || craft_menu::bar::is_open(ctx) {
        return Vec::new();
    }
    craft_menu::bar::leave_for_shortcut(ctx);
    let active = Active {
        typing: ctx.egui_wants_keyboard_input(),
        ..Active::NONE
    };
    let hits = ctx.input_mut(|input| dispatcher.dispatch(input, catalog_keymap(), &active));
    hits.into_iter().map(|hit| hit.command).collect()
}

pub fn lookup_chord(chord_str: &str, typing: bool) -> Result<Option<CommandRef>, String> {
    let chord = Chord::parse(chord_str).map_err(|e| format!("invalid chord: {e:?}"))?;
    match catalog_keymap().lookup(&chord, false, &Active { typing, ..Active::NONE }) {
        Lookup::Run(cmd) => Ok(Some(cmd)),
        _ => Ok(None),
    }
}
