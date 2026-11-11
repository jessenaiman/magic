//! Project lifecycle (`project.*` and ArtCraft `file.*` suite commands), assets,
//! workshop/transcription (`timing-reference.json`), and `DesignCraft` Storyboard (`storyboard.*`).

use super::{cmd_spec, query_spec};
use crate::commands::{CommandSpec, always};
use crate::project::{
    cmd_file_save_as, cmd_project_add_asset, cmd_project_autosave, cmd_project_close, cmd_project_import_file, cmd_project_list_assets, cmd_project_load_asset,
    cmd_project_new, cmd_project_open, cmd_project_remove_asset, cmd_project_save, cmd_project_trunk, cmd_project_update_asset,
};
use crate::workshop::{
    cmd_audio_analyze, cmd_project_search, cmd_workshop_add, cmd_workshop_approve_draft, cmd_workshop_files, cmd_workshop_import_transcript,
    cmd_workshop_remove, cmd_workshop_set_audio, cmd_workshop_set_target, cmd_workshop_split_panels, cmd_workshop_status,
};

pub fn specs() -> Vec<CommandSpec> {
    let mut out = Vec::with_capacity(52);
    out.extend(specs_project());
    out.extend(specs_file());
    out.extend(specs_workshop());
    out.extend(specs_storyboard());
    out
}

fn specs_project() -> Vec<CommandSpec> {
    vec![
        query_spec!(
            "project.attach",
            "Attach Project Metadata Without Switching Studio",
            "{\"path\":\"existing .magic path\"}",
            crate::project::cmd_project_attach
        ),
        query_spec!("project.identity", "Canonical Project Identity", "{\"path\":\"optional existing .magic path\"}", crate::project::cmd_project_identity),
        query_spec!("project.catalog", "Home Project Navigation History", "{}", crate::project_catalog::catalog),
        cmd_spec!(
            "project.root",
            "Inspect / Set Production Project Root",
            ["File"],
            None,
            "{\"root\":\"optional absolute production root\"}",
            always,
            crate::project_settings::command
        ),
        query_spec!("project.list_assets", "List Project Assets", "{}", cmd_project_list_assets),
        query_spec!("project.snapshot", "Everything a View Draws for the Open Project", "{}", crate::cmd::project::cmd_project_snapshot),
        cmd_spec!(
            "project.new",
            "New Project (Requires Audio Track + Transcribes to FilmCraft & Starts Storyboard)",
            ["File"],
            Some("Cmd+N"),
            "{\"path\":\"folder/name.magic, or root + name\",\"root\":\"folder\",\"name\":\"project title\",\"mode\":\"project (song, requires audio) | animation\",\"audio\":\"required song audio path for song projects\",\"transcript\":\"optional pre-timed transcript JSON\",\"wizard\":{\"style\":\"\",\"world\":\"\",\"character\":\"\",\"motion\":\"\"},\"overwrite\":false}",
            always,
            cmd_project_new
        ),
        cmd_spec!("project.open", "Open Project", ["File"], Some("Cmd+O"), "{\"path\":\"folder/name.magic\"}", always, cmd_project_open),
        cmd_spec!("project.save", "Save Project", ["File"], Some("Cmd+S"), "{\"path\":\"optional_save_as.magic\"}", always, cmd_project_save),
        cmd_spec!("project.autosave", "Write Crash-Recovery Journal (.history/*.autosave)", [], None, "{}", always, cmd_project_autosave),
        cmd_spec!("film.save", "Save Film (FilmCraft)", ["File"], None, "{}", always, crate::cmd::film::cmd_film_save),
        cmd_spec!("effect.save", "Save Lettering (EffectCraft)", ["File"], None, "{}", always, crate::cmd::words::native_motion::cmd_effect_save),
        cmd_spec!("shot.commit", "Save the Animation's Three Documents", ["Animation"], None, "{}", always, crate::cmd::commit::cmd_shot_commit),
        cmd_spec!("project.close", "Close Project", ["File"], None, "{}", always, cmd_project_close),
        cmd_spec!(
            "project.add_asset",
            "Connect Project Asset (Optional PhotoCraft / VectorCraft Isolation)",
            ["Project Assets"],
            None,
            "{\"path\":\"image_or_manifest\",\"name\":\"optional\",\"id\":\"optional\",\"kind\":\"auto|image|poses|turnaround|cutout_rig|video\",\"character\":\"optional\",\"background\":\"optional\",\"image\":\"optional\",\"remove_background\":false,\"vectorize\":false}",
            always,
            cmd_project_add_asset
        ),
        cmd_spec!("project.remove_asset", "Remove Project Asset", ["Project Assets"], None, "{\"id\":\"asset_id\"}", always, cmd_project_remove_asset),
        cmd_spec!(
            "project.update_asset",
            "Rename / Reorder Asset",
            ["Project Assets"],
            None,
            "{\"id\":\"asset_id\",\"name\":\"optional\",\"index\":0}",
            always,
            cmd_project_update_asset
        ),
        cmd_spec!(
            "project.load_asset",
            "Load Connected Asset",
            ["Project Assets"],
            None,
            "{\"id\":\"asset_id\",\"reload\":false}",
            always,
            cmd_project_load_asset
        ),
        cmd_spec!(
            "project.import_file",
            "Import Project or Media File",
            ["Project Assets"],
            None,
            "{\"paths\":[\"file\"],\"path\":\"optional single file\",\"overwrite\":false}",
            always,
            cmd_project_import_file
        ),
    ]
}

fn specs_file() -> Vec<CommandSpec> {
    vec![
        cmd_spec!(
            "file.new",
            "New Project (ArtCraft Suite File Command)",
            ["File"],
            None,
            "{\"path\":\"folder/name.magic, or root + name\",\"root\":\"folder\",\"name\":\"project title\",\"mode\":\"project|animation\",\"audio\":\"song audio path\",\"overwrite\":false}",
            always,
            cmd_project_new
        ),
        cmd_spec!("file.open", "Open Project (ArtCraft Suite File Command)", ["File"], None, "{\"path\":\"folder/name.magic\"}", always, cmd_project_open),
        cmd_spec!("file.save", "Save Project (ArtCraft Suite File Command)", ["File"], None, "{\"path\":\"optional_save_as.magic\"}", always, cmd_project_save),
        cmd_spec!(
            "file.saveAs",
            "Save Project As (ArtCraft Suite File Command)",
            ["File"],
            Some("Cmd+Shift+S"),
            "{\"path\":\"destination.magic\"}",
            always,
            cmd_file_save_as
        ),
        cmd_spec!("file.close", "Close Project (ArtCraft Suite File Command)", ["File"], None, "{}", always, cmd_project_close),
        query_spec!("file.recent", "Recent Projects History (config.toml)", "{\"forget\":\"optional path\",\"clear\":false}", crate::project_catalog::catalog),
        query_spec!("file.list", "List Directory Files (Confined to --automation-read-root)", "{\"path\":\"directory\"}", crate::confinement::cmd_file_list),
    ]
}

fn specs_workshop() -> Vec<CommandSpec> {
    vec![
        query_spec!("workshop.status", "Workshop Stages", "{}", cmd_workshop_status),
        query_spec!("workshop.files", "Workshop Files", "{\"filter\":\"optional substring\",\"group\":\"optional\"}", cmd_workshop_files),
        query_spec!("project.trunk", "ArtCraft Project File Trunk", "{}", cmd_project_trunk),
        query_spec!("project.search", "Search Project, Files, Storyboard & Commands", "{\"query\":\"search text\"}", cmd_project_search),
        query_spec!("song.lyrics", "Song Lyrics (Artist Sheet or Transcript)", "{}", crate::cmd::words::lyrics::cmd_song_lyrics),
        query_spec!("song.waveform", "Song Audio Waveform", "{\"start\":0,\"end\":\"optional seconds\",\"bins\":800}", crate::cmd::song::waveform::read),
        cmd_spec!(
            "song.save",
            "Save Song Details",
            ["Song"],
            None,
            "{\"title\":\"optional\",\"album\":\"optional\",\"track_number\":1,\"lyrics\":\"optional\",\"notes\":\"optional\"}",
            always,
            crate::cmd::song::song_metadata::save
        ),
        cmd_spec!("workshop.set_audio", "Set Song Audio", ["Workshop"], None, "{\"path\":\"song audio file\"}", always, cmd_workshop_set_audio),
        cmd_spec!("workshop.set_target", "Set Video Target", ["Workshop"], None, "{\"target\":\"filmcraft\"}", always, cmd_workshop_set_target),
        cmd_spec!(
            "workshop.add",
            "Add Liner Notes / Panels / Assets",
            ["Workshop"],
            None,
            "{\"kind\":\"liner_notes|panels|assets\",\"paths\":[\"file\"]}",
            always,
            cmd_workshop_add
        ),
        cmd_spec!(
            "workshop.split_panels",
            "Split Storyboard Grid (PhotoCraft)",
            ["Workshop"],
            None,
            "{\"image\":\"storyboard grid\",\"columns\":3,\"rows\":2,\"inset\":8}",
            always,
            cmd_workshop_split_panels
        ),
        cmd_spec!(
            "workshop.import_transcript",
            "Import Transcript & Build timing-reference.json + Synced FilmCraft Lyric Track",
            ["Workshop"],
            None,
            "{\"path\":\"transcript.json\",\"captions\":\"optional SRT\",\"replace\":false}",
            always,
            cmd_workshop_import_transcript
        ),
        cmd_spec!("workshop.approve_draft", "Approve Storyboard", ["Workshop"], None, "{\"approved\":true}", always, cmd_workshop_approve_draft),
        cmd_spec!("workshop.remove", "Remove Workshop File", ["Workshop"], None, "{\"path\":\"file in project folder\"}", always, cmd_workshop_remove),
        cmd_spec!(
            "audio.analyze",
            "Transcribe Song Audio via FilmCraft & Write timing-reference.json",
            ["File"],
            None,
            "{\"audio\":\"optional song path\",\"model\":\"whisper-base\"}",
            always,
            cmd_audio_analyze
        ),
    ]
}

fn specs_storyboard() -> Vec<CommandSpec> {
    vec![
        cmd_spec!(
            "storyboard.open",
            "Open Storyboard PDF in PDFCraft (--control)",
            ["Storyboard"],
            None,
            "{\"path\":\"optional storyboard.pdf\",\"page\":0}",
            always,
            crate::cmd::storyboard::cmd_storyboard_open
        ),
        cmd_spec!(
            "storyboard.new",
            "Initialize storyboard.pdf from Read-Only Master Template",
            ["Storyboard"],
            None,
            "{\"path\":\"optional storyboard.pdf\",\"overwrite\":false}",
            always,
            crate::cmd::storyboard::cmd_storyboard_new
        ),
        query_spec!(
            "storyboard.pages",
            "Inspect 17-Page Storyboard PDF Sections, Fields & Previews",
            "{\"path\":\"optional storyboard.pdf\",\"page\":\"optional index\",\"label\":\"optional field\",\"full\":false}",
            crate::cmd::storyboard::cmd_storyboard_pages
        ),
        query_spec!(
            "storyboard.check",
            "Red/Green Required-Field Readiness Across Storyboard Sections",
            "{\"path\":\"optional storyboard.pdf\"}",
            crate::cmd::storyboard::cmd_storyboard_check
        ),
        cmd_spec!(
            "storyboard.wizard",
            "Populate Core Storyboard PDF Fields (Title, Style, World, Character, Motion)",
            ["Storyboard"],
            None,
            "{\"title\":\"optional\",\"style\":\"optional\",\"world\":\"optional\",\"character\":\"optional\",\"motion\":\"optional\"}",
            always,
            crate::cmd::storyboard::cmd_storyboard_wizard
        ),
        cmd_spec!(
            "storyboard.bulk_generate",
            "Plan Bulk Scene Still Generation From Storyboard References",
            ["Storyboard"],
            None,
            "{\"overwrite\":false}",
            always,
            crate::cmd::storyboard::cmd_storyboard_bulk_generate
        ),
        cmd_spec!(
            "storyboard.set",
            "Edit Storyboard PDF Text Field or Place PhotoCraft / VectorCraft / EffectCraft Artwork",
            ["Storyboard"],
            None,
            "{\"page\":0,\"label\":\"optional field\",\"text\":\"optional text\",\"image\":\"optional .png/.pcraft/.psd/.vectorcraft/.ecproj\",\"character\":\"optional id\",\"time\":0.0,\"remove_background\":false,\"lut\":\"optional .cube\",\"vectorize\":false,\"fields\":{},\"images\":{}}",
            always,
            crate::cmd::storyboard::cmd_storyboard_set
        ),
        cmd_spec!(
            "storyboard.grade",
            "Batch Grade Placed Storyboard Images via PhotoCraft (photocraft-cli batch --actions)",
            ["Storyboard"],
            None,
            "{\"section\":\"optional Scenes|Backgrounds\",\"lut\":\"optional .cube/.3dl/.look\",\"vibrance\":15,\"curves\":[[0,0],[255,255]],\"sharpen\":40,\"auto_tone\":false,\"actions\":[]}",
            always,
            crate::cmd::storyboard::cmd_storyboard_grade
        ),
        cmd_spec!(
            "storyboard.add",
            "Read-Only Guard (Master Storyboard Layout Is Immutable)",
            ["Storyboard"],
            None,
            "{}",
            always,
            crate::cmd::storyboard::cmd_storyboard_add
        ),
        cmd_spec!(
            "storyboard.remove",
            "Read-Only Guard (Master Storyboard Layout Is Immutable)",
            ["Storyboard"],
            None,
            "{}",
            always,
            crate::cmd::storyboard::cmd_storyboard_remove
        ),
        cmd_spec!(
            "storyboard.section",
            "Read-Only Guard (Master Storyboard Sections Are Immutable)",
            ["Storyboard"],
            None,
            "{}",
            always,
            crate::cmd::storyboard::cmd_storyboard_section
        ),
        cmd_spec!(
            "storyboard.locations",
            "Inspect or Update Storyboard Backgrounds Fields",
            ["Storyboard"],
            None,
            "{\"page\":\"optional page\",\"fields\":{},\"images\":{}}",
            always,
            crate::cmd::storyboard::cmd_storyboard_locations
        ),
        query_spec!(
            "storyboard.thumbnail",
            "Render storyboard.pdf Pages to PNG via PDFCraft",
            "{\"page\":0,\"rail\":true,\"scale\":1.5}",
            crate::cmd::storyboard::cmd_storyboard_thumbnail
        ),
        query_spec!(
            "storyboard.export",
            "Export storyboard.pdf Copy or PNG Page Directory",
            "{\"pdf\":\"optional path\",\"png_dir\":\"optional dir\",\"scale\":1.0}",
            crate::cmd::storyboard::cmd_storyboard_export
        ),
    ]
}
