//! `03 Lyrics` dockable `Navigator` (`Lyric Moments` 35mm reel) and `Timeline` (`EffectCraft`
//! `V3` + `WORDS` sequence dock) panels rendered inside [`craft_ui::docking`].

use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, ScrollArea, Sense, Ui, Vec2};
use serde_json::{Value, json};

use crate::controls::{FilmbarOrientation, FilmbarStyle, fill_card, paint_craft_doc_card, paint_filmbar_frame, paint_local_image};
use crate::state::{UiAction, ViewCtx};
use crate::timeline::{CraftTimelineLane, CraftTimelineSpan, StandardTimelineSpec, TimelineFlavor, draw_standard_timeline};
use crate::views::lyrics::{StageWordToken, collect_stage_word_tokens};
use crate::widgets::{BtnSpec, action_button, status_pill, text_input};

struct LyricReelEntry {
    id: String,
    anchor_sec: f64,
    line_text: String,
    frame_path: Option<String>,
}

fn collect_lyric_reel_entries(ctx: &ViewCtx<'_>, anims: &[Value]) -> Vec<LyricReelEntry> {
    let film_frames = ctx.state.active_film_frames();
    if !anims.is_empty() {
        return anims
            .iter()
            .enumerate()
            .map(|(idx, anim)| {
                let id = anim["animation_id"].as_str().unwrap_or("sec").to_owned();
                let sec_name = anim["scope"]["section"].as_str().unwrap_or(&id);
                let anchor_sec = anim["song_anchor"].as_f64().unwrap_or((idx as f64) * 4.0);
                let words_str: String = anim["words"].as_array().into_iter().flatten().filter_map(|w| w["text"].as_str()).take(8).collect::<Vec<_>>().join(" ");
                let line_text = if words_str.is_empty() { sec_name.to_owned() } else { words_str };
                let frame_path = anim["background_path"]
                    .as_str()
                    .or_else(|| anim["preview_frame"].as_str())
                    .or_else(|| anim["lyrics_frame"].as_str())
                    .filter(|p| !p.is_empty())
                    .map(str::to_owned)
                    .or_else(|| if film_frames.is_empty() { None } else { film_frames.get(idx % film_frames.len()).cloned() });
                LyricReelEntry { id, anchor_sec, line_text, frame_path }
            })
            .collect();
    }

    let raw_lines: Vec<String> = ctx
        .state
        .cache
        .song_lyrics
        .get("lyrics")
        .and_then(Value::as_str)
        .map(|txt| txt.lines().map(str::trim).filter(|l| !l.is_empty()).take(10).map(str::to_owned).collect())
        .unwrap_or_default();

    let count = raw_lines.len().max(film_frames.len()).clamp(4, 10);
    let total = ctx.state.audio_duration_sec.max(24.0);
    (0..count)
        .map(|idx| {
            let anchor_sec = (idx as f64) * (total / (count as f64));
            let line_text = raw_lines.get(idx).cloned().unwrap_or_else(|| format!("Reel Beat {} · Align Audio", idx + 1));
            let frame_path = if film_frames.is_empty() { None } else { film_frames.get(idx % film_frames.len()).cloned() };
            LyricReelEntry { id: format!("reel-{}", idx + 1), anchor_sec, line_text, frame_path }
        })
        .collect()
}

fn truncate_words(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_owned()
    } else {
        let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

#[allow(clippy::cognitive_complexity)]
pub fn draw_moment_film_reel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) -> Option<Pos2> {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array).cloned().unwrap_or_default();
    let entries = collect_lyric_reel_entries(ctx, &anims);
    let mut selected_socket = None;

    let panel = fill_card(ui, ctx.pal, true, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Film Reel · Lyrics").size(12.2).strong());
            status_pill(
                ui,
                ctx.pal,
                if anims.is_empty() { "Scene Reel" } else { "Timed V3" },
                if anims.is_empty() { ctx.pal.accent_amber } else { ctx.pal.accent_emerald },
            );
        });
        text_input(
            ui,
            ctx.reg,
            "words-lyrics-search",
            "Search lyrics",
            &mut ctx.state.lyrics.search,
            "Filter lyric moments…",
            (ui.available_width() - 4.0).max(120.0),
        );
        ui.add_space(4.0);

        let query = ctx.state.lyrics.search.trim().to_ascii_lowercase();
        ScrollArea::vertical().id_salt("words_moments_scroll").auto_shrink([false, false]).show(ui, |ui| {
            for (idx, item) in entries.iter().enumerate() {
                if !query.is_empty() && !item.line_text.to_ascii_lowercase().contains(&query) && !item.id.to_ascii_lowercase().contains(&query) {
                    continue;
                }
                let selected = ctx.state.lyrics.selected_anim_id.as_deref().map_or(idx == 0, |sel| sel == item.id)
                    || (anims.is_empty() && (ctx.state.lyrics.scrub_seconds - item.anchor_sec).abs() < 2.2);

                let reel_w = (ui.available_width() - 4.0).max(132.0);
                let (crect, resp) = ui.allocate_exact_size(Vec2::new(reel_w, 96.0), Sense::click());
                let inner = paint_filmbar_frame(
                    ui,
                    crect,
                    ctx.pal,
                    FilmbarStyle { orientation: FilmbarOrientation::Vertical, selected, hovered: resp.hovered(), timecode: None },
                );
                if !paint_local_image(ui, inner, item.frame_path.as_deref(), ctx.pal.bg_stage, 3) {
                    paint_craft_doc_card(ui, inner, ctx.pal, "EC", "lyrics.ecproj", "");
                }
                let band = Rect::from_min_max(Pos2::new(inner.min.x, (inner.max.y - 30.0).max(inner.min.y)), inner.max);
                ui.painter().rect_filled(band, CornerRadius::same(2), Color32::from_rgba_unmultiplied(8, 12, 18, 224));
                ui.painter().text(
                    Pos2::new(band.min.x + 6.0, band.min.y + 8.0),
                    Align2::LEFT_CENTER,
                    format!("♪ 0:{:04.1}", item.anchor_sec),
                    FontId::monospace(8.6),
                    if selected { ctx.pal.accent_emerald } else { ctx.pal.accent_amber },
                );
                ui.painter().text(
                    Pos2::new(band.min.x + 6.0, band.max.y - 8.0),
                    Align2::LEFT_CENTER,
                    truncate_words(&item.line_text, 22),
                    FontId::proportional(10.2),
                    ctx.pal.text_primary,
                );
                if selected {
                    selected_socket = Some(Pos2::new(crect.max.x - 2.0, crect.center().y));
                }
                if resp.clicked() {
                    if !anims.is_empty() {
                        ctx.state.lyrics.selected_anim_id = Some(item.id.clone());
                    }
                    ctx.state.lyrics.selected_word_id = None;
                    ctx.state.lyrics.scrub_seconds = item.anchor_sec;
                    ctx.push(UiAction::SeekAudio(item.anchor_sec));
                }
                ctx.reg.record(format!("words-moment-{}-0", item.id), &item.line_text, &resp, None);
                ui.add_space(5.0);
            }
        });
    });
    ctx.reg.record("words-lyrics", "Lyric Film Reel", &panel.response, None);
    selected_socket
}

pub fn draw_lyrics_timeline_panel(ui: &mut Ui, ctx: &mut ViewCtx<'_>) {
    let anims = ctx.state.cache.words_doc.pointer("/document/animations").and_then(Value::as_array).cloned().unwrap_or_default();
    let active =
        ctx.state.lyrics.selected_anim_id.as_deref().and_then(|id| anims.iter().find(|a| a["animation_id"].as_str() == Some(id))).or_else(|| anims.first());
    let tokens = collect_stage_word_tokens(ctx, active);
    draw_effectcraft_lyrics_timeline_dock(ui, ctx, &anims, active, &tokens);
}

#[allow(clippy::cognitive_complexity)]
fn draw_effectcraft_lyrics_timeline_dock(ui: &mut Ui, ctx: &mut ViewCtx<'_>, anims: &[Value], active: Option<&Value>, tokens: &[StageWordToken]) {
    fill_card(ui, ctx.pal, true, |ui| {
        let total_dur = ctx.state.audio_duration_sec.max(30.0);
        let has_audio = ctx.state.song_audio_path().is_some();
        let film_frames = ctx.state.active_film_frames();
        let active_words = active.and_then(|a| a["words"].as_array()).cloned().unwrap_or_default();

        let word_spans: Vec<CraftTimelineSpan> = if !active_words.is_empty() {
            active_words
                .iter()
                .take(36)
                .map(|w| {
                    let st = w["start"].as_f64().unwrap_or(0.0);
                    CraftTimelineSpan::new(st, w["end"].as_f64().unwrap_or(st + 0.28), w["text"].as_str().unwrap_or(""))
                        .with_id(w["word_id"].as_str().unwrap_or(""))
                })
                .collect()
        } else {
            tokens.iter().map(|t| CraftTimelineSpan::new(t.start_sec, t.end_sec, t.text.clone()).with_id(t.id.clone())).collect()
        };

        let section_spans: Vec<CraftTimelineSpan> = anims
            .iter()
            .enumerate()
            .map(|(idx, a)| {
                let st = a["song_anchor"].as_f64().unwrap_or((idx as f64) * 5.0);
                let next_st = anims.get(idx + 1).and_then(|n| n["song_anchor"].as_f64()).unwrap_or((st + 4.5).min(total_dur));
                let lbl = a["scope"]["section"].as_str().or_else(|| a["animation_id"].as_str()).unwrap_or("Lyrics").to_owned();
                CraftTimelineSpan::new(st, next_st.max(st + 1.2), lbl).with_id(a["animation_id"].as_str().unwrap_or(""))
            })
            .collect();

        let cue_keys: Vec<f64> =
            if !anims.is_empty() { anims.iter().filter_map(|a| a["song_anchor"].as_f64()).collect() } else { tokens.iter().map(|t| t.start_sec).collect() };

        let audio_spans = if has_audio {
            vec![CraftTimelineSpan::new(0.0, total_dur, if ctx.state.lyrics.audio_source == "filmcraft" { "FilmCraft Audio Mix" } else { "Song Audio" })]
        } else {
            Vec::new()
        };

        let dirty_v3 = ctx.state.dirty_tracks.contains("V3") || ctx.state.dirty_drafts.contains("lyrics");
        if dirty_v3 {
            ui.horizontal(|ui| {
                status_pill(ui, ctx.pal, "Out of sync (lyrics.ecproj modified)", ctx.pal.accent_amber);
                if action_button(
                    ui,
                    ctx.reg,
                    ctx.pal,
                    BtnSpec::new("words-sync-linked", "refresh", "Sync Comps", "Sync lyrics.ecproj into FilmCraft").primary(),
                )
                .clicked()
                {
                    ctx.exec("video.sync_links", json!({}), Some("Synced lyrics.ecproj with FilmCraft"));
                    ctx.state.dirty_tracks.remove("V3");
                }
            });
        }

        let lanes = [
            CraftTimelineLane {
                code: "V1",
                title: "FC V1 · FILM",
                color: ctx.pal.accent_sky,
                filmbar: true,
                waveform: false,
                dirty: false,
                spans: Vec::new(),
                keyframes: &[],
                frames: &film_frames,
                empty_hint: "Scene reference",
            },
            CraftTimelineLane {
                code: "AUDIO",
                title: "AUDIO",
                color: ctx.pal.accent_emerald,
                filmbar: false,
                waveform: has_audio,
                dirty: false,
                spans: audio_spans,
                keyframes: &cue_keys,
                frames: &[],
                empty_hint: "No audio linked",
            },
            CraftTimelineLane {
                code: "V3",
                title: "EC V3 LYRICS",
                color: ctx.pal.accent_amber,
                filmbar: false,
                waveform: false,
                dirty: dirty_v3,
                spans: section_spans,
                keyframes: &cue_keys,
                frames: &[],
                empty_hint: "effects/lyrics.ecproj sections",
            },
            CraftTimelineLane {
                code: "WORDS",
                title: "WORDS",
                color: ctx.pal.accent_sky,
                filmbar: false,
                waveform: false,
                dirty: false,
                spans: word_spans,
                keyframes: &[],
                frames: &[],
                empty_hint: "Interactive word beats",
            },
        ];
        let mut scrub = ctx.state.lyrics.scrub_seconds;
        let tl_out = draw_standard_timeline(
            ui,
            ctx,
            StandardTimelineSpec { id_prefix: "words", flavor: TimelineFlavor::EffectCraft, total_dur_sec: total_dur, fps: 24.0, in_out: None },
            &mut scrub,
            &lanes,
        );
        if tl_out.toggled_play {
            ctx.push(UiAction::ToggleAudio);
        }
        if let Some((lane_idx, span_id, _)) = tl_out.clicked_span
            && !span_id.is_empty()
        {
            if lane_idx == 2 {
                ctx.state.lyrics.selected_anim_id = Some(span_id);
            } else if lane_idx == 3 {
                ctx.state.lyrics.selected_word_id = Some(span_id);
            }
        }
        if tl_out.scrubbed {
            ctx.state.lyrics.scrub_seconds = scrub;
            ctx.push(UiAction::SeekAudio(scrub));
        }
    });
}
