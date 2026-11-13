//! FilmCraft × EffectCraft bridge, timeline interchange, markers, and Studio finishing command specs.

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
        query_spec!(
            "video.list_ec_comps",
            "List Compositions & Alpha Formats in an EffectCraft .ecproj",
            "{\"project\":\"comp.ecproj\"}",
            crate::cmd::film::cmd_video_list_ec_comps
        ),
        cmd_spec!(
            "video.link_effectcraft",
            "Dynamic-Link EffectCraft Composition onto FilmCraft Timeline",
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
        query_spec!(
            "timeline.detect",
            "Detect Timeline Format (OTIO, FCPXML, FCP7 XML, EDL, AAF, OMF)",
            "{\"path\":\"optional edit.xml\"}",
            crate::cmd::film::cmd_timeline_detect
        ),
        query_spec!(
            "timeline.timecode",
            "Convert Exact Ticks, Seconds, Frames & SMPTE Timecode (craft-timeline)",
            "{\"seconds\":0,\"fps\":24}",
            crate::cmd::film::cmd_timeline_timecode
        ),
        query_spec!(
            "timeline.inspect",
            "Inspect Active .fcproj or External Timeline via craft-timeline Model",
            "{\"path\":\"optional edit.otio\"}",
            crate::cmd::film::cmd_timeline_inspect
        ),
        cmd_spec!(
            "timeline.export",
            "Export Sequence to OTIO, FCPXML, FCP7 XML, CMX 3600 EDL, or ALE (craft-timeline)",
            ["Studio"],
            None,
            "{\"format\":\"otio|fcpxml|xml|edl|ale\",\"path\":\"optional\"}",
            always,
            crate::cmd::film::cmd_timeline_export
        ),
        cmd_spec!(
            "timeline.import",
            "Import OTIO, FCPXML, FCP7 XML, CMX 3600 EDL, or ALE into Sequence (craft-timeline)",
            ["Studio"],
            None,
            "{\"path\":\"edit.otio\"}",
            always,
            crate::cmd::film::cmd_timeline_import
        ),
        cmd_spec!(
            "timeline.ale",
            "Parse or Export Avid Log Exchange .ale (craft-timeline)",
            ["Studio"],
            None,
            "{\"action\":\"parse|export\",\"path\":\"optional\"}",
            always,
            crate::cmd::film::cmd_timeline_ale
        ),
        cmd_spec!(
            "timeline.edl",
            "Parse or Export CMX 3600 .edl (craft-timeline)",
            ["Studio"],
            None,
            "{\"action\":\"parse|export\",\"path\":\"optional\"}",
            always,
            crate::cmd::film::cmd_timeline_edl
        ),
        query_spec!("video.film", "Film Timeline, Preview and Export Status", "{}", crate::cmd::film::cmd_video_film),
        cmd_spec!("video.preview", "Render Half-Scale Film Preview (FilmCraft)", ["Studio"], None, "{}", always, crate::cmd::film::cmd_video_preview),
        cmd_spec!("video.export", "Export Final Film (FilmCraft)", ["Studio"], None, "{}", always, crate::cmd::film::cmd_video_export),
        query_spec!("video.frame", "Render One Program Frame (FilmCraft)", "{\"seconds\":0}", crate::cmd::film::cmd_video_frame),
        query_spec!("video.open", "Open Timeline in FilmCraft", "{}", crate::cmd::film::cmd_video_open),
        query_spec!("video.timeline", "Film Tracks, Clips and Synced Markers", "{}", crate::cmd::film::cmd_video_timeline),
        cmd_spec!(
            "video.markers",
            "Manage, Export CSV & Interchange Timeline Markers (FilmCraft <-> EffectCraft)",
            ["Studio"],
            Some("M"),
            "{\"action\":\"list|add|edit|palette|export_csv|seed|delete|clear|interchange\"}",
            always,
            crate::cmd::film::cmd_video_markers
        ),
        cmd_spec!(
            "video.place_clip",
            "Place Clip on FilmCraft Track",
            ["Studio"],
            None,
            "{\"clip\":\"rendered .mov\",\"from\":0,\"track\":\"V2\"}",
            always,
            crate::cmd::film::cmd_video_place_clip
        ),
    ]
}
