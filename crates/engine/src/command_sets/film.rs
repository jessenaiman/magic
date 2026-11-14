//! FilmCraft × EffectCraft bridge and Studio finishing command specs.

use super::{cmd_spec, query_spec};
use crate::commands::{CommandSpec, always};

pub(super) fn specs_film() -> Vec<CommandSpec> {
    vec![
        cmd_spec!(
            "video.assemble",
            "Assemble FilmCraft Timeline (A1 Song, V1 Scenes, V2 Character, V3 Lyrics, V4 Motion)",
            ["Studio"],
            None,
            "{\"backgrounds\":[],\"characters\":[],\"lettering\":\"optional\",\"motion\":\"optional\",\"fps\":30,\"export\":true}",
            always,
            crate::cmd::film::assembly::cmd_video_assemble
        ),
        cmd_spec!(
            "video.map_tracks",
            "Batch-Map FilmCraft Studio Tracks into Reusable Lyrics & Motion Groups",
            ["Studio"],
            None,
            "{\"mappings\":[{\"tracks\":[\"V3\",\"V5\"],\"target\":\"lyrics\",\"group\":\"chorus\"},{\"tracks\":[\"V4\"],\"target\":\"motion\",\"group\":\"beats\"}]}",
            always,
            crate::cmd::film::cmd_video_map_tracks
        ),
        cmd_spec!(
            "studio.mapTracks",
            "Batch-Map FilmCraft Studio Tracks into Reusable Lyrics & Motion Groups",
            ["Studio"],
            None,
            "{\"mappings\":[{\"tracks\":[\"V3\",\"V5\"],\"target\":\"lyrics\",\"group\":\"chorus\"},{\"tracks\":[\"V4\"],\"target\":\"motion\",\"group\":\"beats\"}]}",
            always,
            crate::cmd::film::cmd_video_map_tracks
        ),
        query_spec!(
            "video.list_ec_comps",
            "List Compositions & Alpha Formats in an EffectCraft .ecproj",
            "{\"project\":\"comp.ecproj\"}",
            crate::cmd::film::cmd_video_list_ec_comps
        ),
        cmd_spec!(
            "video.link_effectcraft",
            "Dynamic-Link EffectCraft Composition or Saved Animation onto FilmCraft Timeline",
            ["Studio"],
            None,
            "{\"project\":\"comp.ecproj\",\"comp\":\"optional Main\",\"track\":\"V2\",\"format\":\"prores|webm|exr\",\"fps_preset\":\"24\",\"from\":0}",
            always,
            crate::cmd::film::cmd_video_link_effectcraft
        ),
        cmd_spec!(
            "video.sync_links",
            "Check or Re-Render Dirty Dynamic-Linked EffectCraft Compositions",
            ["Studio"],
            None,
            "{\"render\":true}",
            always,
            crate::cmd::film::cmd_video_sync_links
        ),
        cmd_spec!(
            "video.bake_link",
            "Bake Dynamic-Linked EffectCraft Comp to Standalone Media (or Unbake)",
            ["Studio"],
            None,
            "{\"link_id\":\"comp-id\",\"unbake\":false}",
            always,
            crate::cmd::film::cmd_video_bake_link
        ),
        cmd_spec!(
            "video.finish",
            "Execute FilmCraft Studio Finishing Operation (Edit, Trim, Color, FX, Audio, Titles)",
            ["Studio"],
            None,
            "{\"op\":\"mark_in|mark_out|insert|overwrite|lift|extract|add_edit|trim_mode|sequence_rate|color_space|lumetri_look|fx_apply|audio_gain|audio_ducking|mixer_strip|graphic_text|scopes\"}",
            always,
            crate::cmd::film::cmd_video_finish
        ),
        query_spec!("video.film", "Film Timeline, Preview and Export Status", "{}", crate::cmd::film::cmd_video_film),
        cmd_spec!("video.preview", "Render Half-Scale Film Preview (FilmCraft)", ["Studio"], None, "{}", always, crate::cmd::film::cmd_video_preview),
        cmd_spec!("video.export", "Export Final Film (FilmCraft)", ["Studio"], None, "{}", always, crate::cmd::film::cmd_video_export),
        query_spec!("video.frame", "Render One Program Frame (FilmCraft)", "{\"seconds\":0}", crate::cmd::film::cmd_video_frame),
        query_spec!("video.open", "Open Timeline in FilmCraft", "{}", crate::cmd::film::cmd_video_open),
        query_spec!("video.timeline", "Film Tracks, Clips and Synced Markers", "{}", crate::cmd::film::cmd_video_timeline),
        cmd_spec!(
            "video.place_clip",
            "Place Clip on FilmCraft Track",
            ["Studio"],
            None,
            "{\"clip\":\"rendered .mov/.mp4/.webm/sequence\",\"from\":0,\"track\":\"V2\"}",
            always,
            crate::cmd::film::cmd_video_place_clip
        ),
    ]
}
