//! Creative Craft-producer command registrations:
//! - Kinetic Teaching Lyrics (`words.*` -> `lettering/lyrics.ecproj` -> FilmCraft `V3` `"Kinetic Lyrics"`).
//! - Song-Action Motion (`motion.*` -> `motion/actions.ecproj` -> FilmCraft `V4` `"Action Motion"`).
//! - Character Animation (`character.*` -> `.ecproj` -> FilmCraft `V2` `"Character Animation"`).
//! - Subject Isolation & Vector Tracing (`native_media::isolate_subject_rgba`, `native_media::trace_silhouette_svg`, `scribbles.*`).
//! - FilmCraft × EffectCraft Bridge & Studio Finishing (`bridge.*`, `effect.*`, `video.*`).

use super::{cmd_spec, query_spec};
use crate::commands::{CommandSpec, always};

pub fn specs() -> Vec<CommandSpec> {
    let mut out = Vec::with_capacity(98);
    out.extend(specs_words());
    out.extend(specs_motion());
    out.extend(specs_photo_vector_and_scribbles());
    out.extend(specs_character());
    out.extend(specs_film());
    out
}

fn specs_words() -> Vec<CommandSpec> {
    vec![
        query_spec!(
            "words.workspace",
            "Open or Inspect a Standalone Lyrics Folder",
            "{\"path\":\"optional directory\",\"audio\":\"optional audio\"}",
            crate::cmd::words::cmd_words_workspace
        ),
        query_spec!("words.align", "Artist Words vs Timing Reference", "{}", crate::cmd::words::cmd_words_align),
        query_spec!("words.read", "Kinetic Teaching Lyrics Animations", "{\"animation_id\":\"optional\"}", crate::cmd::words::cmd_words_read),
        query_spec!("words.preview_frame", "Render Kinetic Lyrics Frame (EffectCraft)", "{\"time\":24.3}", crate::cmd::words::preview::frame),
        query_spec!(
            "words.motion.properties",
            "Native EffectCraft Word Transform & Animator Properties",
            "{\"line_id\":\"optional\",\"word_ids\":[]}",
            crate::cmd::words::native_motion::properties
        ),
        query_spec!(
            "words.motion.curve",
            "Preview Native Word Transform Curve",
            "{\"property\":\"transform/rotation\",\"to\":15,\"duration\":2}",
            crate::cmd::words::native_motion::curve
        ),
        cmd_spec!(
            "words.motion.set",
            "Set Native Word Transform in lyrics.ecproj",
            ["Lyrics"],
            None,
            "{\"animation_id\":\"id\",\"line_id\":\"id\",\"word_ids\":[\"id\"],\"expected_source_revision\":0,\"property\":\"transform/position\",\"to\":[0,-30],\"duration\":0.6}",
            always,
            crate::cmd::words::native_motion::set
        ),
        query_spec!("words.studio_track", "Kinetic Lyric Clips on FilmCraft V3", "{}", crate::cmd::words::cmd_words_studio_track),
        cmd_spec!(
            "words.lyrics_view",
            "Stage, Lyrics-Only or Scene-Only View",
            ["Lyrics"],
            None,
            "{\"show\":\"stage|lyrics|scene|still\"}",
            always,
            crate::cmd::words::cmd_words_lyrics_view
        ),
        query_spec!(
            "words.effects",
            "CLI-Mapped Craft Typography Effects (EffectCraft, VectorCraft & PhotoCraft)",
            "{\"app\":\"optional effectcraft|vectorcraft|photocraft\",\"category\":\"optional\",\"filter\":\"optional\"}",
            crate::cmd::words::cmd_words_effects
        ),
        cmd_spec!(
            "words.effect_install",
            "Install a Registry Effect",
            ["Lyrics"],
            None,
            "{\"name\":\"per-word-rise\"}",
            always,
            crate::cmd::words::cmd_words_effect_install
        ),
        cmd_spec!(
            "words.captions",
            "Sync SRT Captions onto FilmCraft Timeline",
            ["Lyrics"],
            None,
            "{\"by\":\"section|line\",\"to\":\"video\"}",
            always,
            crate::cmd::words::cmd_words_captions
        ),
        cmd_spec!(
            "words.captions_sync",
            "Bidirectional V3 Lyrics <-> FilmCraft C1 Caption Track & Graphic Upgrade",
            ["Lyrics"],
            None,
            "{\"track_format\":\"Subtitle|CEA-608|CEA-708|Teletext\",\"format\":\"srt|vtt|scc|ttml\",\"upgrade_to_graphic\":false,\"export_sidecar\":false}",
            always,
            crate::cmd::film::cmd_words_captions_sync
        ),
        cmd_spec!("words.track", "Lyric Line in the Video", ["Lyrics"], None, "{}", always, crate::cmd::words::cmd_words_track),
        cmd_spec!(
            "words.design",
            "Design Kinetic Teaching Lyric Scenes",
            ["Lyrics"],
            None,
            "{\"recipe\":\"optional lyrics-design.json\",\"apply\":true}",
            always,
            crate::cmd::words::cmd_words_design
        ),
        cmd_spec!(
            "words.letter",
            "Build lettering/lyrics.ecproj & Export V3 Track to FilmCraft",
            ["Lyrics"],
            None,
            "{\"recipe\":\"optional\",\"render\":true,\"studio\":true}",
            always,
            crate::cmd::words::lettering::cmd_words_letter
        ),
        cmd_spec!(
            "words.apply",
            "Use Designed Lyrics in the Video",
            ["Lyrics"],
            None,
            "{\"animation_id\":\"words-chorus-1\",\"use\":true}",
            always,
            crate::cmd::words::cmd_words_apply
        ),
        cmd_spec!(
            "words.publish",
            "Publish Rendered Kinetic Lyrics (.mov) onto FilmCraft V3",
            ["Lyrics"],
            None,
            "{\"path\":\"optional lettering/lyrics.mov\"}",
            always,
            crate::cmd::words::lettering::cmd_words_publish
        ),
        cmd_spec!(
            "words.save",
            "Save Kinetic Lyrics Scene",
            ["Lyrics"],
            None,
            "{\"animation_id\":\"words-chorus-1\",\"expected_source_revision\":0}",
            always,
            crate::cmd::words::cmd_words_save
        ),
        cmd_spec!(
            "words.link",
            "Link Accent Element to a Lyric Word",
            ["Lyrics"],
            None,
            "{\"animation_id\":\"section\",\"index\":0,\"word_id\":\"target\"}",
            always,
            crate::cmd::words::cmd_words_link
        ),
        cmd_spec!(
            "words.compose",
            "Compose Words Track",
            ["Lyrics"],
            None,
            "{\"animation_id\":\"words-chorus-1\"}",
            always,
            crate::cmd::words::cmd_words_compose
        ),
    ]
}

fn specs_motion() -> Vec<CommandSpec> {
    vec![
        query_spec!("motion.read", "Song-Action Motion Cues & Timed Words (motion/actions.ecproj)", "{}", crate::cmd::motion::cmd_motion_read),
        query_spec!(
            "motion.effects",
            "CLI-Mapped Craft Vector & Motion Effects (VectorCraft, EffectCraft & PhotoCraft)",
            "{\"app\":\"optional vectorcraft|effectcraft|photocraft\",\"category\":\"optional\",\"filter\":\"optional\"}",
            crate::cmd::motion::cmd_motion_effects
        ),
        cmd_spec!(
            "motion.save",
            "Save Song-Action Drawing Cue (VectorCraft Trace / SVG + EffectCraft Trim Paths)",
            ["Motion"],
            None,
            "{\"word_id\":\"w-0001\",\"preset\":\"optional whale|wave|star|boat|moon|jump\",\"d\":\"optional SVG path\",\"image\":\"optional raster image to trace via VectorCraft\",\"color\":\"#64CBFF\",\"stroke_width\":8,\"draw_duration\":0.85,\"hold_duration\":1.2,\"render\":false,\"studio\":false}",
            always,
            crate::cmd::motion::cmd_motion_save
        ),
        cmd_spec!(
            "motion.publish",
            "Render motion/actions.ecproj (ProRes 4444 RGBA) & Publish V4 Action Motion Track to FilmCraft",
            ["Motion"],
            None,
            "{\"render\":true,\"path\":\"optional motion/actions.mov\"}",
            always,
            crate::cmd::motion::cmd_motion_publish
        ),
        cmd_spec!(
            "motion.open",
            "Open Song-Action Cue in VectorCraft or EffectCraft",
            ["Motion"],
            None,
            "{\"app\":\"vectorcraft|effectcraft\",\"id\":\"optional cue id\"}",
            always,
            crate::cmd::motion::cmd_motion_open
        ),
        query_spec!(
            "motion.preview_frame",
            "Render Song-Action Motion Frame (motion/actions.ecproj)",
            "{\"time\":12.5}",
            crate::cmd::motion::cmd_motion_preview_frame
        ),
        query_spec!("motion.theatre.read", "Read Project Timeline State", "{\"expected_project\":\"path\"}", crate::project::theatre_state),
        cmd_spec!(
            "motion.theatre.save",
            "Save Project Timeline State",
            [],
            None,
            "{\"expected_project\":\"path\",\"state\":{}}",
            always,
            crate::project::save_theatre_state
        ),
    ]
}

fn specs_photo_vector_and_scribbles() -> Vec<CommandSpec> {
    vec![
        cmd_spec!(
            "image.generate",
            "Generate Image via craft-ai-integrations (With Optional PhotoCraft Cutout & VectorCraft Trace)",
            ["Project Assets"],
            None,
            "{\"prompt\":\"description\",\"references\":[],\"remove_background\":false,\"vectorize\":false,\"name\":\"optional\"}",
            always,
            crate::craft_ai::cmd_image_generate
        ),
        cmd_spec!(
            "scribbles.connect",
            "Bind Scribbles Connection & Model in SharedStore (craft-ai-integrations)",
            ["Scribbles"],
            None,
            "{\"connection\":\"connection id\",\"model\":\"model id\"}",
            always,
            crate::craft_ai::cmd_ai_configure
        ),
        cmd_spec!(
            "scribbles.consent",
            "Grant or Revoke Outbound Craft AI Consent",
            ["Scribbles"],
            None,
            "{\"agree\":true,\"term\":\"one_hour|one_day|until_revoked\"}",
            always,
            crate::craft_ai::cmd_ai_consent
        ),
        cmd_spec!(
            "scribbles.ingest",
            "Ingest Staged Character Bundle Folder into PhotoCraft & EffectCraft",
            ["Scribbles"],
            None,
            "{\"dir\":\"bundle directory\",\"character\":\"optional\"}",
            always,
            crate::scribbles::cmd_scribbles_ingest
        ),
        query_spec!(
            "fidget.prepare",
            "Preview Deterministic Template Brief",
            "{\"kind\":\"new|image|sheet|poses|actions|custom\",\"prompt\":\"description\"}",
            crate::scribbles::cmd_fidget_prepare
        ),
        cmd_spec!(
            "fidget.stage",
            "Rephrase & Stage Structured Prompt via Fidget (With Optional Seamless Scribbles Handoff)",
            ["Scribbles"],
            None,
            "{\"kind\":\"new|image|sheet|poses|actions|custom\",\"prompt\":\"description\",\"seamless\":false}",
            always,
            crate::scribbles::cmd_fidget_stage
        ),
        cmd_spec!(
            "scribbles.generate",
            "Generate Drawing From Template via Scribbles",
            ["Scribbles"],
            None,
            "{\"prompt\":\"instructions\"}",
            always,
            crate::scribbles::cmd_scribbles_generate
        ),
        query_spec!("scribbles.templates", "Scribbles Templates & Craft AI Status", "{}", crate::scribbles::cmd_scribbles_templates),
        cmd_spec!("scribbles.new", "New Character Drawing", ["Scribbles"], None, "{\"prompt\":\"character\"}", always, crate::scribbles::cmd_scribbles_new),
        cmd_spec!(
            "scribbles.image",
            "Redraw Character for Animation",
            ["Scribbles"],
            None,
            "{\"reference\":\"image\",\"prompt\":\"character\"}",
            always,
            crate::scribbles::cmd_scribbles_image
        ),
        cmd_spec!(
            "scribbles.sheet",
            "Turnaround Sheet",
            ["Scribbles"],
            None,
            "{\"reference\":\"image\",\"prompt\":\"character\"}",
            always,
            crate::scribbles::cmd_scribbles_sheet
        ),
        cmd_spec!(
            "scribbles.poses",
            "Pose Sheet (3x2)",
            ["Scribbles"],
            None,
            "{\"reference\":\"image\",\"prompt\":\"character\"}",
            always,
            crate::scribbles::cmd_scribbles_poses
        ),
        cmd_spec!(
            "scribbles.poses_2x2",
            "Pose Sheet (2x2)",
            ["Scribbles"],
            None,
            "{\"reference\":\"image\",\"prompt\":\"character\"}",
            always,
            crate::scribbles::cmd_scribbles_poses_2x2
        ),
        cmd_spec!(
            "scribbles.parts_chroma",
            "Cutout Parts Sheet",
            ["Scribbles"],
            None,
            "{\"reference\":\"image\",\"prompt\":\"character\"}",
            always,
            crate::scribbles::cmd_scribbles_parts_chroma
        ),
        cmd_spec!(
            "scribbles.moppet",
            "3-Part Character Bundle + PhotoCraft Split & EffectCraft Rig",
            ["Scribbles"],
            None,
            "{\"character\":\"id\",\"prompt\":\"description\",\"rig\":true}",
            always,
            crate::scribbles::cmd_scribbles_moppet
        ),
        cmd_spec!(
            "scribbles.actions",
            "8-Frame Action Sheet",
            ["Scribbles"],
            None,
            "{\"reference\":\"image\",\"action\":\"walk\"}",
            always,
            crate::scribbles::cmd_scribbles_actions
        ),
        cmd_spec!(
            "scribbles.layers",
            "Layered Character Sheet",
            ["Scribbles"],
            None,
            "{\"reference\":\"image\"}",
            always,
            crate::scribbles::cmd_scribbles_layers
        ),
        cmd_spec!("scribbles.custom", "Custom Drawing", ["Scribbles"], None, "{\"prompt\":\"drawing\"}", always, crate::scribbles::cmd_scribbles_custom),
        cmd_spec!(
            "scribbles.svg",
            "Trace Vector Pose Sheet via VectorCraft",
            ["Scribbles"],
            None,
            "{\"sheet\":\"pose-sheet.png\"}",
            always,
            crate::scribbles::cmd_scribbles_svg
        ),
        cmd_spec!(
            "scribbles.reassemble",
            "Split Parts Sheet via PhotoCraft (layer.maskAllObjects) & Rig in EffectCraft",
            ["Scribbles"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"sheet\":\"parts-sheet.png\",\"rig\":true}",
            always,
            crate::scribbles::cmd_scribbles_reassemble
        ),
    ]
}

fn specs_character() -> Vec<CommandSpec> {
    vec![
        query_spec!("character.list", "Characters and Their Craft Files (.pcraft, .ecproj)", "{}", crate::cmd::character::cmd_character_list),
        query_spec!("character.meta", "Character Views, Parts and Actions", "{\"character\":\"id\"}", crate::cmd::character::cmd_character_meta),
        query_spec!("character.actions", "Actions in Shared Pose Table", "{}", crate::cmd::character::cmd_character_actions),
        query_spec!(
            "character.keys",
            "Rig Layers, Keyframes and Curves",
            "{\"character\":\"id\",\"view\":\"front\"}",
            crate::cmd::character::keys::cmd_character_keys
        ),
        query_spec!("character.path", "Character Motion Path", "{\"character\":\"id\",\"view\":\"front\"}", crate::cmd::character::path::cmd_character_path),
        cmd_spec!(
            "character.save_path",
            "Save Character Motion Path",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"points\":[[0,0],[10,10]]}",
            always,
            crate::cmd::character::path::cmd_character_save_path
        ),
        cmd_spec!(
            "character.import",
            "Import Character From Image, Craft File, or 3D Model",
            ["Animation"],
            None,
            "{\"path\":\"file\"}",
            always,
            crate::cmd::character::cmd_character_import
        ),
        cmd_spec!(
            "character.mark",
            "Mark Parts and Joints",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"image\":\"drawing.png\"}",
            always,
            crate::cmd::character::cmd_character_mark
        ),
        cmd_spec!(
            "character.save_marks",
            "Save Adjusted Part Outlines & Joints",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"parts\":[]}",
            always,
            crate::cmd::character::cmd_character_save_marks
        ),
        cmd_spec!(
            "character.cut",
            "Isolate & Cut Parts Into Layers (PhotoCraft + VectorCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"remove_background\":true,\"rig\":false}",
            always,
            crate::cmd::character::cmd_character_cut
        ),
        cmd_spec!(
            "character.rig",
            "Build Layered Puppet Rig in EffectCraft (.ecproj)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\"}",
            always,
            crate::cmd::character::cmd_character_rig
        ),
        cmd_spec!(
            "character.action",
            "Key & Expression-Drive Action in EffectCraft",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"action\":\"idle\",\"times\":1}",
            always,
            crate::cmd::character::cmd_character_action
        ),
        cmd_spec!(
            "character.sprite",
            "Build Sprite Cycle in PhotoCraft / Headless AniCraft CLI + EffectCraft",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"action\":\"walk\",\"sheet\":\"sheet.png or .anicraft\",\"columns\":4,\"rows\":2}",
            always,
            crate::cmd::character::cmd_character_sprite
        ),
        cmd_spec!(
            "character.shot",
            "Build Scene Shot (.ecproj -> ProRes 4444) & Optional V2 Publish",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"sprite\":\"walk\",\"duration\":4,\"studio\":false}",
            always,
            crate::cmd::character::cmd_character_shot
        ),
        cmd_spec!(
            "character.publish",
            "Publish Character ProRes 4444 Render onto FilmCraft V2",
            ["Animation"],
            None,
            "{\"path\":\"rendered .mov\"}",
            always,
            crate::cmd::character::cmd_character_publish
        ),
        cmd_spec!(
            "character.render",
            "Render Frames or Clip (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"start\":0,\"end\":1,\"format\":\"frames|clip\"}",
            always,
            crate::cmd::character::cmd_character_render
        ),
        query_spec!(
            "character.frame",
            "Render Single Character Frame (EffectCraft)",
            "{\"character\":\"id\",\"view\":\"front\",\"time\":0}",
            crate::cmd::character::cmd_character_frame
        ),
        cmd_spec!(
            "character.set",
            "Key One Transform Property (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"part\":\"head\",\"prop\":\"rotation\",\"value\":8,\"time\":0.5}",
            always,
            crate::cmd::character::cmd_character_set
        ),
        query_spec!(
            "character.open",
            "Open Character in EffectCraft or PhotoCraft",
            "{\"character\":\"id\",\"view\":\"front\",\"app\":\"effectcraft|photocraft\"}",
            crate::cmd::character::cmd_character_open
        ),
        cmd_spec!(
            "animation.open",
            "Open .ecproj in EffectCraft",
            ["Animation"],
            None,
            "{\"path\":\".ecproj\"}",
            always,
            crate::cmd::character::cmd_animation_open
        ),
        query_spec!(
            "animation.view",
            "Animation View Model",
            "{\"character\":\"optional\",\"view\":\"optional\",\"time\":0}",
            crate::cmd::character::cmd_animation_view
        ),
        query_spec!("animation.motion_options", "Single-Subject EffectCraft Motion Options", "{}", crate::cmd::character::review::cmd_motion_options),
        cmd_spec!(
            "animation.prepare_motion",
            "Prepare Character Motion in EffectCraft",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"clip\":\"clip\",\"option\":\"baseline\"}",
            always,
            crate::cmd::character::review::cmd_prepare_motion
        ),
        query_spec!("animation.review", "Animation Review Models and Revisions", "{}", crate::cmd::character::review::cmd_review),
        cmd_spec!(
            "animation.save_revision",
            "Save Animation Revision",
            ["Animation"],
            None,
            "{\"expected_project\":\"path\",\"expected_revision\":0,\"character\":\"id\"}",
            always,
            crate::cmd::character::review::cmd_save_revision
        ),
        cmd_spec!(
            "character.puppet.rig_face",
            "Rig 11-Pin Facial & Mouth Puppet Mesh (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"optional head\"}",
            always,
            crate::cmd::character::cmd_puppet_rig_face
        ),
        cmd_spec!(
            "character.puppet.expression",
            "Keyframe Facial Expression via Puppet Pins (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"expression\":\"happy|joy|surprised|oh|sad|focused|blink|wink_L|wink_R|neutral\",\"at\":0,\"duration\":0.9}",
            always,
            crate::cmd::character::cmd_puppet_expression
        ),
        cmd_spec!(
            "character.puppet.lipsync",
            "Animate Mouth Viseme Puppet Pins From timing-reference.json",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"start\":0,\"intensity\":1.0}",
            always,
            crate::cmd::character::cmd_puppet_lipsync
        ),
        cmd_spec!(
            "character.puppet.follow",
            "Cascade Secondary Follow-Through on Puppet Pins",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"tail\",\"leader\":\"tail_base\",\"pins\":[\"tail_mid\",\"tail_tip\"],\"delay\":0.07,\"cascade\":true}",
            always,
            crate::cmd::character::cmd_puppet_follow
        ),
        cmd_spec!(
            "character.puppet.record",
            "Record Real-Time Puppet Pin Path",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"head\",\"pin\":\"jaw_lower\",\"samples\":[[0,128,200]]}",
            always,
            crate::cmd::character::cmd_puppet_record
        ),
        cmd_spec!(
            "character.puppet.addPin",
            "Add Puppet Pin (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"torso\",\"kind\":\"position|advanced|bend|starch|overlap\",\"position\":[0,0]}",
            always,
            crate::cmd::character::cmd_puppet_add_pin
        ),
        cmd_spec!(
            "character.puppet.setPin",
            "Set Advanced/Bend/Starch/Overlap Pin Properties",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"torso\",\"pin\":\"Puppet Pin 1\",\"rotation\":0,\"scale\":100,\"stiffness\":50,\"inFront\":50}",
            always,
            crate::cmd::character::cmd_puppet_set_pin
        ),
        cmd_spec!(
            "character.puppet.selectPins",
            "Select Puppet Pins on Layer",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"torso\",\"pins\":[\"Puppet Pin 1\"]}",
            always,
            crate::cmd::character::cmd_puppet_select_pins
        ),
        cmd_spec!(
            "character.puppet.nullRig",
            "Link Puppet Pins <-> Null Rig (paths.pointsFollowNulls / nullsFollowPoints)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"torso\",\"mode\":\"points_follow_nulls|nulls_follow_points\"}",
            always,
            crate::cmd::character::cmd_puppet_null_rig
        ),
        cmd_spec!(
            "character.puppet.movePin",
            "Move Puppet Pin (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"torso\",\"pin\":\"Puppet Pin 1\",\"position\":[0,0]}",
            always,
            crate::cmd::character::cmd_puppet_move_pin
        ),
        cmd_spec!(
            "character.puppet.removePin",
            "Delete Puppet Pin (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"torso\",\"pin\":\"Puppet Pin 1\"}",
            always,
            crate::cmd::character::cmd_puppet_remove_pin
        ),
        cmd_spec!(
            "character.expression.set",
            "Set Expression (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"torso\",\"path\":\"transform/rotation\",\"expression\":\"wiggle(2,4)\"}",
            always,
            crate::cmd::character::cmd_expression_set
        ),
        cmd_spec!(
            "character.keyframe.easyEase",
            "Easy Ease Keyframes (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"torso\",\"prop\":\"rotation\"}",
            always,
            crate::cmd::character::cmd_keyframe_easy_ease
        ),
        cmd_spec!(
            "character.keyframe.velocity",
            "Set Keyframe Velocity (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"torso\",\"prop\":\"rotation\"}",
            always,
            crate::cmd::character::cmd_keyframe_velocity
        ),
        cmd_spec!(
            "character.time.remap",
            "Enable Time Remapping (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"torso\"}",
            always,
            crate::cmd::character::cmd_time_remap
        ),
        cmd_spec!(
            "character.mask.add",
            "Add Mask to Layer (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"torso\",\"points\":[]}",
            always,
            crate::cmd::character::cmd_mask_add
        ),
        cmd_spec!(
            "character.layer.new3dModel",
            "Add 3D Model Layer (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"path\":\"model.glb\"}",
            always,
            crate::cmd::character::cmd_layer_new_3d_model
        ),
        cmd_spec!(
            "character.layer.setBlendMode",
            "Set Layer Blend Mode (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"torso\",\"mode\":\"normal\"}",
            always,
            crate::cmd::character::cmd_layer_set_blend_mode
        ),
        cmd_spec!(
            "character.layer.setParent",
            "Parent Layer (EffectCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\",\"view\":\"front\",\"layer\":\"head\",\"parent\":\"torso\"}",
            always,
            crate::cmd::character::cmd_layer_set_parent
        ),
        query_spec!("character.model3d.catalog", "3D Character Breakout & Motion Preset Catalog", "{}", crate::cmd::character::cmd_model3d_catalog),
        cmd_spec!(
            "character.model3d.breakout",
            "3D Character Breakout (PhotoCraft Split / T-Pose Spec)",
            ["Animation"],
            None,
            "{\"character\":\"id\"}",
            always,
            crate::cmd::character::cmd_model3d_breakout
        ),
        cmd_spec!(
            "character.model3d.rig",
            "3D Mesh Pre-Check & Skeleton Validation (TextureCraft)",
            ["Animation"],
            None,
            "{\"character\":\"id\"}",
            always,
            crate::cmd::character::cmd_model3d_rig
        ),
        cmd_spec!(
            "character.model3d.animate",
            "3D Motion Clip & EffectCraft advanced3D Comp",
            ["Animation"],
            None,
            "{\"character\":\"id\"}",
            always,
            crate::cmd::character::cmd_model3d_animate
        ),
    ]
}

fn specs_film() -> Vec<CommandSpec> {
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
