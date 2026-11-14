//! `shot.commit`: persist an animation's three documents in one call — the lettering EffectCraft
//! project, the FilmCraft project and the `.magic` project — and report success only after all
//! three saved. The first failure is returned as an error, never as an `ok`.

use std::path::Path;

use serde_json::{Value, json};

use crate::{Result, Session};

/// Record external document presence and modification timestamp as `(exists, mtime_millis)`.
pub fn external_doc_revision(path: &Path) -> (bool, u64) {
    match std::fs::metadata(path) {
        Ok(m) => {
            let mtime = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as u64);
            (true, mtime)
        }
        Err(_) => (false, 0),
    }
}

/// Save `lettering/lyrics.ecproj` (`effect.save`), then the film `.fcproj` (`film.save`), then
/// the `.magic` (`project.save`). The reply's `effect`, `film` and `magic` are the three saved
/// paths; `ok` is reported only after every one of them saved, tracking external Craft document
/// revisions as `(exists, mtime)`.
pub fn cmd_shot_commit(s: &mut Session, _p: &Value) -> Result<Value> {
    let effect = crate::cmd::words::native_motion::cmd_effect_save(s, &json!({}))?;
    let film = crate::cmd::film::cmd_film_save(s, &json!({}))?;
    let magic = crate::project::cmd_project_save(s, &json!({}))?;

    let magic_path = magic["project"].as_str().map(Path::new);
    let effect_path = effect["project"].as_str().map(Path::new);
    let film_path = film["project"].as_str().map(Path::new);

    let magic_rev = magic_path.map(external_doc_revision).unwrap_or((false, 0));
    let effect_rev = effect_path.map(external_doc_revision).unwrap_or((false, 0));
    let film_rev = film_path.map(external_doc_revision).unwrap_or((false, 0));

    Ok(json!({
        "ok": true,
        "magic": magic["project"],
        "effect": effect["project"],
        "film": film["project"],
        "revisions": {
            "magic": {"exists": magic_rev.0, "mtime": magic_rev.1},
            "effect": {"exists": effect_rev.0, "mtime": effect_rev.1},
            "film": {"exists": film_rev.0, "mtime": film_rev.1},
        }
    }))
}
