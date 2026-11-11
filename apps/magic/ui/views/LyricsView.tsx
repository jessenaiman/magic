import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { executeCommand } from "../api";
import { takeAuthoringTime } from "../authoring-navigation";
import { LyricsInspector } from "./lyrics/LyricsInspector";
import { LyricsMomentRail } from "./lyrics/LyricsMomentRail";
import { LyricsTetherOverlay } from "./lyrics/LyricsTetherOverlay";
import { LyricsStageCanvas } from "./lyrics/LyricsStageCanvas";
import { LyricsTimelineDock } from "./lyrics/LyricsTimelineDock";
import {
  BackdropImage, buildDefaultLineDesign, LetteringPreset, LineDesign, LyricLineGroup,
  MovePreset, SceneClip, STAGE_H, STAGE_W, StrokePreset, TextPresetItem, WordToken,
  WordsAnimation, WordsReadReply,
} from "./lyrics/types";

export interface LyricsViewProps {
  projectPath: string;
  mode: "design" | "motion";
  onModeChange: (nextMode: "design" | "motion") => void;
  onNavigateStudio: () => void;
  onToast: (message: string) => void;
}

const DEFAULT_MOVES: MovePreset[] = [
  { id: "quiet", name: "Quiet", does: "Fades in and holds" },
  { id: "pop", name: "Bounce / Pop", does: "Swells on the beat" },
  { id: "loop", name: "Wave / Loop", does: "Continuous wave" },
  { id: "fall", name: "Kinetic Tilt", does: "Drops down from above" },
  { id: "rise", name: "Fire / Impact", does: "Arcs up from below" },
  { id: "east", name: "Neon Drift", does: "Slides right" },
  { id: "west", name: "Slide West", does: "Slides left" },
  { id: "settle", name: "Sparkle Settle", does: "Drops and settles into place" },
  { id: "sleep", name: "Sleep", does: "Letters sink one by one" },
];

const DEFAULT_LETTERINGS: LetteringPreset[] = [
  { id: "serif-italic", name: "Serif Italic", does: "Expressive hero word", font: "serif", weight: 400, italic: true },
  { id: "serif-bold", name: "Serif Bold", does: "Strong hero word", font: "serif", weight: 700, italic: false },
  { id: "song-bold", name: "Song Bold", does: "Heavy display sans", font: "song", weight: 700, italic: false },
  { id: "song", name: "Song Regular", does: "Clean lyric lettering", font: "song", weight: 400, italic: false },
  { id: "sans", name: "Sans Quiet", does: "Calm context words", font: "sans", weight: 400, italic: false },
];

const DEFAULT_CHOREOGRAPHIES = [
  { id: "cascade", name: "Cascade", does: "Diagonal staircase" },
  { id: "scatter", name: "Spiral / Scatter", does: "Tilted dynamic spread" },
  { id: "poster", name: "Zoom Blast", does: "Hero word giant & tilted" },
  { id: "stack", name: "Stack", does: "Vertical stepped column" },
  { id: "plain", name: "Plain", does: "Clean horizontal row" },
];

const formatTime = (sec: number): string => {
  if (!Number.isFinite(sec) || sec < 0) return "0:00.0";
  const m = Math.floor(sec / 60);
  const s = (sec % 60).toFixed(1).padStart(4, "0");
  return `${m}:${s}`;
};

export const LyricsView: React.FC<LyricsViewProps> = ({
  projectPath,
  mode,
  onModeChange,
  onNavigateStudio,
  onToast,
}) => {
  const [readData, setReadData] = useState<WordsReadReply | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [selectedLineKey, setSelectedLineKey] = useState<string | null>(null);
  const [activeItemIndex, setActiveItemIndex] = useState<number>(0);
  const [stageBgMode, setStageBgMode] = useState<"scene" | "paper">("scene");
  const [ecFrameUrl, setEcFrameUrl] = useState<string | null>(null);
  const [renderingEcFrame, setRenderingEcFrame] = useState<boolean>(false);
  const [publishingStudio, setPublishingStudio] = useState<boolean>(false);
  const [drawMode, setDrawMode] = useState<boolean>(false);
  const [brushColor, setBrushColor] = useState<string>("#FF9E3D");
  const [brushWidth, setBrushWidth] = useState<number>(8);
  const [playheadSec, setPlayheadSec] = useState<number>(0.8);
  const [waveformPeaks, setWaveformPeaks] = useState<number[]>([]);
  const [timelineMarkers, setTimelineMarkers] = useState<
    {
      id?: number | null;
      ec_index?: number;
      index: number;
      name: string;
      comment: string;
      kind: string;
      color: string;
      time_sec: number;
      duration_sec: number;
      source: string;
    }[]
  >([]);
  const [sceneTrackClips, setSceneTrackClips] = useState<
    {
      clip: number;
      name: string;
      start_sec: number;
      end_sec: number;
      label?: string;
    }[]
  >([]);

  // Local draft map keyed by `${animId}::${lineId}`
  const [draftDesigns, setDraftDesigns] = useState<Record<string, LineDesign>>({});

  // Refs for tether computation & interactive stage dragging
  const workspaceRef = useRef<HTMLDivElement | null>(null);
  const stageRef = useRef<HTMLDivElement | null>(null);
  const anchorRefs = useRef<Map<string, HTMLButtonElement>>(new Map());
  const boxRefs = useRef<Map<number, HTMLDivElement>>(new Map());
  const [tetherCoords, setTetherCoords] = useState<{
    x1: number;
    y1: number;
    x2: number;
    y2: number;
    cx1: number;
    cy1: number;
    cx2: number;
    cy2: number;
  } | null>(null);

  const loadWords = useCallback(async () => {
    setLoading(true);
    try {
      const res = (await executeCommand("words.read", {})) as WordsReadReply;
      setReadData(res);
      const nextDrafts: Record<string, LineDesign> = {};
      let firstKey: string | null = null;
      let firstTime = 0.8;
      for (const anim of res.document?.animations ?? []) {
        const byLine = new Map<string, WordToken[]>();
        for (const w of anim.words ?? []) {
          const lid = w.line_id || "line-1";
          const arr = byLine.get(lid) ?? [];
          arr.push(w);
          byLine.set(lid, arr);
        }
        for (const [lid, lwords] of byLine.entries()) {
          const key = `${anim.animation_id}::${lid}`;
          if (!firstKey) {
            firstKey = key;
            firstTime = lwords[0]?.start ?? 0.8;
          }
          const saved = anim.design?.[lid];
          if (saved && Array.isArray(saved.items) && saved.items.length > 0) {
            nextDrafts[key] = structuredClone(saved);
          } else {
            nextDrafts[key] = buildDefaultLineDesign(lwords, "cascade");
          }
        }
      }
      setDraftDesigns(nextDrafts);
      const jumpSec = takeAuthoringTime(projectPath);
      if (typeof jumpSec === "number" && Number.isFinite(jumpSec)) {
        setPlayheadSec(jumpSec);
      } else {
        setPlayheadSec(firstTime);
      }
      setSelectedLineKey((prev) => prev ?? firstKey);
    } catch (err) {
      onToast(`Could not load lyrics: ${String(err)}`);
    } finally {
      setLoading(false);
    }
  }, [onToast, projectPath]);

  const refreshSyncedTimeline = useCallback(() => {
    executeCommand("video.timeline", {})
      .then((res) => {
        const r = res as {
          markers?: typeof timelineMarkers;
          tracks?: {
            role?: string;
            label?: string;
            clips?: {
              clip: number;
              name?: string;
              start_tick?: number;
              end_tick?: number;
              label?: string;
            }[];
          }[];
        };
        if (Array.isArray(r.markers)) {
          setTimelineMarkers(r.markers);
        }
        const bgTrack = (r.tracks ?? []).find(
          (t) => t.role === "background" || t.label === "V1",
        );
        if (bgTrack && Array.isArray(bgTrack.clips)) {
          setSceneTrackClips(
            bgTrack.clips.map((c, idx) => ({
              clip: c.clip ?? idx,
              name: c.name || `Scene ${idx + 1}`,
              start_sec: (c.start_tick ?? 0) / 254016000000,
              end_sec: (c.end_tick ?? 0) / 254016000000,
              label: c.label,
            })),
          );
        }
      })
      .catch(() => {
        executeCommand("video.markers", { action: "list" })
          .then((res) => {
            const r = res as { markers?: typeof timelineMarkers };
            if (Array.isArray(r.markers)) setTimelineMarkers(r.markers);
          })
          .catch(() => {});
      });
  }, []);

  useEffect(() => {
    void loadWords();
    refreshSyncedTimeline();
    executeCommand("song.waveform", { bins: 180 })
      .then((res) => {
        const peaks = (res as { peaks?: number[] })?.peaks;
        if (Array.isArray(peaks) && peaks.length > 0) {
          setWaveformPeaks(peaks);
        }
      })
      .catch(() => {});
  }, [loadWords, refreshSyncedTimeline]);

  // Flatten all lyric lines across sections
  const allLineGroups = useMemo<LyricLineGroup[]>(() => {
    const out: LyricLineGroup[] = [];
    for (const anim of readData?.document?.animations ?? []) {
      const sectionName =
        anim.scope?.section ||
        anim.animation_id.replace(/-/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
      const byLine = new Map<string, WordToken[]>();
      for (const w of anim.words ?? []) {
        const lid = w.line_id || "line-1";
        const list = byLine.get(lid) ?? [];
        list.push(w);
        byLine.set(lid, list);
      }
      for (const [lineId, words] of byLine.entries()) {
        const start = words[0]?.start ?? anim.song_anchor ?? 0;
        const end = words[words.length - 1]?.end ?? start + 3.5;
        const text = words.map((w) => w.text).join(" ");
        out.push({
          lineId,
          animId: anim.animation_id,
          sectionName,
          words,
          start,
          end,
          text,
        });
      }
    }
    return out;
  }, [readData]);

  const filteredLines = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return allLineGroups;
    return allLineGroups.filter(
      (g) =>
        g.text.toLowerCase().includes(q) ||
        g.sectionName.toLowerCase().includes(q),
    );
  }, [allLineGroups, searchQuery]);

  const activeLine = useMemo(() => {
    if (!allLineGroups.length) return null;
    if (selectedLineKey) {
      const found = allLineGroups.find(
        (g) => `${g.animId}::${g.lineId}` === selectedLineKey,
      );
      if (found) return found;
    }
    return allLineGroups[0];
  }, [allLineGroups, selectedLineKey]);

  const activeAnim = useMemo<WordsAnimation | undefined>(() => {
    if (!activeLine) return undefined;
    return readData?.document?.animations?.find(
      (a) => a.animation_id === activeLine.animId,
    );
  }, [activeLine, readData]);

  const activeKey = activeLine
    ? `${activeLine.animId}::${activeLine.lineId}`
    : "";

  const activeDesign: LineDesign = useMemo(() => {
    if (!activeLine) return { choreography: "cascade", items: [], draws: [] };
    return (
      draftDesigns[activeKey] ?? buildDefaultLineDesign(activeLine.words, "cascade")
    );
  }, [activeKey, activeLine, draftDesigns]);

  const updateActiveDesign = useCallback(
    (updater: (draft: LineDesign) => void) => {
      if (!activeLine) return;
      const key = `${activeLine.animId}::${activeLine.lineId}`;
      setDraftDesigns((prev) => {
        const base = prev[key]
          ? structuredClone(prev[key])
          : buildDefaultLineDesign(activeLine.words, "cascade");
        updater(base);
        return { ...prev, [key]: base };
      });
      setEcFrameUrl(null);
    },
    [activeLine],
  );

  // Save current animation's line designs to words.json and sync lettering/lyrics.ecproj
  const saveAndSyncEcproj = useCallback(async () => {
    if (!activeAnim || !activeLine) return;
    const designPayload: Record<string, LineDesign> = {};
    for (const [k, d] of Object.entries(draftDesigns)) {
      const [aid, lid] = k.split("::");
      if (aid === activeAnim.animation_id && lid) {
        designPayload[lid] = d;
      }
    }
    await executeCommand("words.save", {
      animation_id: activeAnim.animation_id,
      background_asset: activeAnim.background_asset ?? null,
      design: designPayload,
    });
    const letterRes = (await executeCommand("words.letter", {
      render: false,
    })) as { saved_by?: string; schema?: number };
    const refreshed = (await executeCommand("words.read", {})) as WordsReadReply;
    setReadData(refreshed);
    onToast(
      `Saved design/words.json & lettering/lyrics.ecproj (schema ${letterRes.schema ?? 2})`,
    );
  }, [activeAnim, activeLine, draftDesigns, onToast]);

  const previewEffectCraftStage = useCallback(async () => {
    if (!activeAnim || !activeLine) return;
    setRenderingEcFrame(true);
    try {
      await saveAndSyncEcproj();
      const res = (await executeCommand("words.preview_frame", {
        kind: "stage",
        animation_id: activeAnim.animation_id,
        line_id: activeLine.lineId,
        seconds: playheadSec,
      })) as { path?: string };
      if (res?.path) {
        setEcFrameUrl(`${convertFileSrc(res.path)}?t=${Date.now()}`);
        onToast("Rendered live frame from lettering/lyrics.ecproj via EffectCraft");
      }
    } catch (err) {
      onToast(`EffectCraft frame preview error: ${String(err)}`);
    } finally {
      setRenderingEcFrame(false);
    }
  }, [activeAnim, activeLine, onToast, playheadSec, saveAndSyncEcproj]);

  const renderLyricsToStudio = useCallback(async () => {
    setPublishingStudio(true);
    try {
      await saveAndSyncEcproj();
      await executeCommand("words.letter", { render: true, studio: true });
      onToast("Rendered transparent lettering & published to Studio");
      onNavigateStudio();
    } catch (err) {
      onToast(`Render to Studio failed: ${String(err)}`);
    } finally {
      setPublishingStudio(false);
    }
  }, [onNavigateStudio, onToast, saveAndSyncEcproj]);

  // Backdrop image resolution
  const images: BackdropImage[] = readData?.images ?? [];
  const sceneClips: SceneClip[] = readData?.scene_clips ?? [];
  const activeBackdrop = useMemo(() => {
    const wanted = activeAnim?.background_asset;
    if (wanted) {
      const found = images.find((img) => img.id === wanted || img.asset_id === wanted);
      if (found) return found;
    }
    return images[0] ?? null;
  }, [activeAnim?.background_asset, images]);

  const selectBackdrop = async (imgId: string) => {
    if (!activeAnim) return;
    try {
      await executeCommand("words.save", {
        animation_id: activeAnim.animation_id,
        background_asset: imgId,
      });
      const refreshed = (await executeCommand("words.read", {})) as WordsReadReply;
      setReadData(refreshed);
      onToast(`Scene backdrop set to ${imgId}`);
    } catch (err) {
      onToast(String(err));
    }
  };

  // Recompute musical-note tether coordinates whenever selection or layout changes
  const updateTether = useCallback(() => {
    const ws = workspaceRef.current;
    const anchor = selectedLineKey ? anchorRefs.current.get(selectedLineKey) : null;
    const box = boxRefs.current.get(activeItemIndex) ?? boxRefs.current.get(0);
    if (!ws || !anchor || !box) {
      setTetherCoords(null);
      return;
    }
    const wsRect = ws.getBoundingClientRect();
    const aRect = anchor.getBoundingClientRect();
    const bRect = box.getBoundingClientRect();
    const x1 = aRect.left + aRect.width * 0.5 - wsRect.left;
    const y1 = aRect.top + aRect.height * 0.5 - wsRect.top;
    const x2 = bRect.left - wsRect.left;
    const y2 = bRect.top + bRect.height * 0.5 - wsRect.top;
    const dx = Math.max(80, Math.abs(x2 - x1) * 0.45);
    setTetherCoords({
      x1,
      y1,
      x2,
      y2,
      cx1: x1 + dx,
      cy1: y1 - 24,
      cx2: x2 - dx,
      cy2: y2 + 18,
    });
  }, [activeItemIndex, selectedLineKey]);

  useEffect(() => {
    const id = window.requestAnimationFrame(updateTether);
    window.addEventListener("resize", updateTether);
    return () => {
      window.cancelAnimationFrame(id);
      window.removeEventListener("resize", updateTether);
    };
  }, [updateTether, activeDesign, mode]);

  // Pointer drag state for moving, resizing, rotating, or setting motion trajectory on stage
  const startBoxPointerDrag = (
    e: React.PointerEvent,
    idx: number,
    action: "move" | "resize" | "rotate" | "motion-target",
  ) => {
    e.stopPropagation();
    setActiveItemIndex(idx);
    const stageEl = stageRef.current;
    const item = activeDesign.items[idx];
    if (!stageEl || !item) return;
    const rect = stageEl.getBoundingClientRect();
    const scaleX = STAGE_W / Math.max(1, rect.width);
    const scaleY = STAGE_H / Math.max(1, rect.height);
    const startClientX = e.clientX;
    const startClientY = e.clientY;
    const origX = item.x;
    const origY = item.y;
    const origW = item.w ?? 640;
    const origH = item.h ?? 150;
    const origRot = item.rotate ?? 0;
    const origDx = item.dx ?? 0;
    const origDy = item.dy ?? -32;

    const onMove = (mv: PointerEvent) => {
      const dxStage = (mv.clientX - startClientX) * scaleX;
      const dyStage = (mv.clientY - startClientY) * scaleY;
      updateActiveDesign((draft) => {
        const target = draft.items[idx];
        if (!target) return;
        if (action === "move") {
          target.x = Math.round(Math.max(20, Math.min(STAGE_W - 120, origX + dxStage)));
          target.y = Math.round(Math.max(20, Math.min(STAGE_H - 80, origY + dyStage)));
        } else if (action === "resize") {
          target.w = Math.round(Math.max(140, Math.min(STAGE_W - target.x, origW + dxStage)));
          target.h = Math.round(Math.max(60, Math.min(STAGE_H - target.y, origH + dyStage)));
        } else if (action === "rotate") {
          const nextRot = Math.round(origRot + dxStage * 0.45);
          target.rotate = Math.max(-360, Math.min(360, nextRot));
        } else if (action === "motion-target") {
          target.dx = Math.round(origDx + dxStage);
          target.dy = Math.round(origDy + dyStage);
        }
      });
    };

    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  // Freehand drawing on the 16:9 stage
  const [drawingPoints, setDrawingPoints] = useState<[number, number][] | null>(null);
  const onStageDrawDown = (e: React.PointerEvent<SVGSVGElement>) => {
    if (!drawMode) return;
    const svg = e.currentTarget;
    const rect = svg.getBoundingClientRect();
    const x = Math.round(((e.clientX - rect.left) / Math.max(1, rect.width)) * STAGE_W);
    const y = Math.round(((e.clientY - rect.top) / Math.max(1, rect.height)) * STAGE_H);
    setDrawingPoints([[x, y]]);
  };
  const onStageDrawMove = (e: React.PointerEvent<SVGSVGElement>) => {
    if (!drawMode || !drawingPoints) return;
    const svg = e.currentTarget;
    const rect = svg.getBoundingClientRect();
    const x = Math.round(((e.clientX - rect.left) / Math.max(1, rect.width)) * STAGE_W);
    const y = Math.round(((e.clientY - rect.top) / Math.max(1, rect.height)) * STAGE_H);
    setDrawingPoints((prev) => (prev ? [...prev, [x, y]] : [[x, y]]));
  };
  const onStageDrawUp = () => {
    if (!drawMode || !drawingPoints || drawingPoints.length < 2) {
      setDrawingPoints(null);
      return;
    }
    const d = drawingPoints
      .map((pt, i) => `${i === 0 ? "M" : "L"} ${pt[0]} ${pt[1]}`)
      .join(" ");
    updateActiveDesign((draft) => {
      draft.draws.push({
        name: "Freehand Brush",
        view_box: `0 0 ${STAGE_W} ${STAGE_H}`,
        x: 0,
        y: 0,
        w: STAGE_W,
        h: STAGE_H,
        paths: [
          {
            d,
            color: brushColor,
            width: brushWidth,
            from: 0,
            to: 0.9,
          },
        ],
      });
    });
    setDrawingPoints(null);
  };

  const totalSongDuration = useMemo(() => {
    const last = allLineGroups[allLineGroups.length - 1];
    return Math.max(30, (last?.end ?? 118) + 4);
  }, [allLineGroups]);

  const moves = readData?.tools?.moves?.length ? readData.tools.moves : DEFAULT_MOVES;
  const letterings = readData?.tools?.letterings?.length
    ? readData.tools.letterings
    : DEFAULT_LETTERINGS;
  const strokes: StrokePreset[] = readData?.tools?.strokes ?? [];
  const choreographies = readData?.tools?.choreographies?.length
    ? readData.tools.choreographies
    : DEFAULT_CHOREOGRAPHIES;
  const easings = readData?.tools?.easings?.length
    ? readData.tools.easings
    : [
        { id: "linear", name: "Linear", does: "Linear" },
        { id: "quad_in", name: "Ease in (quad)", does: "Ease in (quad)" },
        { id: "quad_out", name: "Ease out (quad)", does: "Ease out (quad)" },
        { id: "quad_in_out", name: "Ease in-out (quad)", does: "Ease in-out (quad)" },
        { id: "cubic_in", name: "Ease in (cubic)", does: "Ease in (cubic)" },
        { id: "cubic_out", name: "Ease out (cubic)", does: "Ease out (cubic)" },
        { id: "cubic_in_out", name: "Ease in-out (cubic)", does: "Ease in-out (cubic)" },
        { id: "smooth", name: "Smooth (bezier)", does: "Smooth (bezier)" },
      ];
  const textPresets: (string | TextPresetItem)[] = readData?.tools?.effectcraft
    ?.text_presets?.length
    ? readData.tools.effectcraft.text_presets
    : [
        "typewriter",
        "fade_up_characters",
        "bounce_in_words",
        "tracking_in",
        "scramble",
        "blur_in",
      ];

  const ecprojMeta = readData?.ecproj;

  return (
    <div
      ref={workspaceRef}
      id="lyrics-workspace-root"
      className="lyrics-native-workspace"
      data-authoring-mode={mode}
    >
      <LyricsTetherOverlay tetherCoords={tetherCoords} />
      <div className="lyrics-main-columns">
        <LyricsMomentRail
          loading={loading}
          allLineGroups={allLineGroups}
          activeBackdrop={activeBackdrop}
          updateTether={updateTether}
          searchQuery={searchQuery}
          setSearchQuery={setSearchQuery}
          filteredLines={filteredLines}
          activeKey={activeKey}
          draftDesigns={draftDesigns}
          setSelectedLineKey={setSelectedLineKey}
          setActiveItemIndex={setActiveItemIndex}
          setPlayheadSec={setPlayheadSec}
          anchorRefs={anchorRefs}
          formatTime={formatTime}
        />
        {/* CENTER STAGE & TIMELINE */}
        <main id="words-stage-panel" className="lyrics-center-column">
          {/* Header Bar */}
          <header className="lyrics-stage-topbar">
            <div className="lyrics-stage-title">
              <strong>
                {activeLine
                  ? `${activeLine.sectionName} · ${activeLine.lineId}`
                  : "Lyrics & Motion"}
              </strong>
              {ecprojMeta?.path && (
                <span className="lyrics-ecproj-pill" title={ecprojMeta.path}>
                  lettering/lyrics.ecproj · schema {ecprojMeta.schema ?? 2}
                  {ecprojMeta.layer_count
                    ? ` · ${ecprojMeta.layer_count} layers`
                    : ""}
                </span>
              )}
            </div>

            <div className="lyrics-mode-tabs">
              <button
                id="words-mode-design"
                type="button"
                className={`lyrics-mode-btn ${mode === "design" ? "active" : ""}`}
                onClick={() => onModeChange("design")}
              >
                T Lyrics
              </button>
              <button
                id="words-mode-motion"
                type="button"
                className={`lyrics-mode-btn ${mode === "motion" ? "active" : ""}`}
                onClick={() => onModeChange("motion")}
              >
                ↗ Motion
              </button>
              <button
                id="words-draw-toggle"
                type="button"
                className={`lyrics-mode-btn ${drawMode ? "active" : ""}`}
                title="Draw freehand vector overlay strokes on stage"
                onClick={() => setDrawMode((d) => !d)}
              >
                ✎ Draw
              </button>
              <button
                id="words-save"
                type="button"
                className="lyrics-mode-btn primary"
                title="Save words.json & rebuild lettering/lyrics.ecproj"
                onClick={() => void saveAndSyncEcproj()}
              >
                💾 Save
              </button>
            </div>
          </header>

          {/* Scene & Video Backdrop Strip */}
          <div id="words-scene-strip" className="lyrics-scene-strip">
            <span className="strip-label">Scene</span>
            {sceneClips.slice(0, 3).map((clip) => (
              <button
                key={clip.id}
                type="button"
                className="scene-video-pill"
                title={clip.label}
                onClick={() => setPlayheadSec(clip.start)}
              >
                ▶ {clip.id}
              </button>
            ))}
            <div className="scene-thumbs-scroll">
              {images.map((img) => {
                const selected = activeBackdrop?.id === img.id;
                return (
                  <button
                    key={img.id}
                    id={`words-bg-thumb-${img.id}`}
                    type="button"
                    className={`scene-thumb-btn ${selected ? "active" : ""}`}
                    title={img.name}
                    onClick={() => void selectBackdrop(img.id)}
                  >
                    <img src={convertFileSrc(img.path)} alt={img.name} />
                  </button>
                );
              })}
            </div>
            <button
              id="words-open-effectcraft"
              type="button"
              className="ghost"
              title="Open lettering/lyrics.ecproj in EffectCraft"
              onClick={() =>
                void saveAndSyncEcproj().then(() =>
                  executeCommand("workspace.open", {
                    app: "effectcraft",
                    path: "lettering/lyrics.ecproj",
                  }).catch((e) => onToast(String(e))),
                )
              }
            >
              EffectCraft ↗
            </button>
          </div>

          <LyricsStageCanvas
            stageRef={stageRef}
            boxRefs={boxRefs}
            mode={mode}
            onModeChange={onModeChange}
            stageBgMode={stageBgMode}
            activeBackdrop={activeBackdrop}
            ecFrameUrl={ecFrameUrl}
            renderingEcFrame={renderingEcFrame}
            previewEffectCraftStage={previewEffectCraftStage}
            drawMode={drawMode}
            setDrawMode={setDrawMode}
            brushColor={brushColor}
            brushWidth={brushWidth}
            activeDesign={activeDesign}
            activeItemIndex={activeItemIndex}
            updateActiveDesign={updateActiveDesign}
            activeLine={activeLine}
            drawingPoints={drawingPoints}
            onStageDrawDown={onStageDrawDown}
            onStageDrawMove={onStageDrawMove}
            onStageDrawUp={onStageDrawUp}
            startBoxPointerDrag={startBoxPointerDrag}
          />
          {/* Transport & Export Bar */}
          <div className="lyrics-transport-bar">
            <input
              id="words-scrub"
              type="range"
              min={0}
              max={totalSongDuration}
              step={0.05}
              value={playheadSec}
              onChange={(e) => setPlayheadSec(Number(e.target.value))}
            />
            <span id="words-time" className="lyrics-time-readout">
              {formatTime(playheadSec)} / {formatTime(totalSongDuration)}
            </span>
            <button
              id="words-add-marker"
              type="button"
              className="ghost"
              title="Add synced Cue/Range Marker at playhead in FilmCraft (.fcproj) and EffectCraft (lyrics.ecproj)"
              onClick={() => {
                const cueName = activeLine
                  ? `${activeLine.sectionName} · ${activeLine.lineId}`
                  : `cue_${formatTime(playheadSec)}`;
                const cueComment = activeLine?.text || cueName;
                void executeCommand("video.markers", {
                  action: "add",
                  seconds: playheadSec,
                  name: cueName,
                  comment: cueComment,
                  color: "Iris",
                  kind: "flashCue",
                })
                  .then((res) => {
                    const r = res as { markers?: typeof timelineMarkers };
                    if (Array.isArray(r.markers)) setTimelineMarkers(r.markers);
                    onToast(
                      `Added marker "${cueName}" at ${formatTime(playheadSec)} in FilmCraft & EffectCraft`,
                    );
                  })
                  .catch((err) => onToast(String(err)));
              }}
            >
              ◆ + Cue
            </button>
            <button
              id="words-seed-markers"
              type="button"
              className="ghost"
              title="Seed section Range Markers & line Flash Cues across FilmCraft (.fcproj) and EffectCraft (lyrics.ecproj)"
              onClick={() => {
                void executeCommand("video.markers", {
                  action: "seed",
                })
                  .then((res) => {
                    const r = res as {
                      seeded_sections?: number;
                      seeded_lines?: number;
                      markers?: typeof timelineMarkers;
                    };
                    if (Array.isArray(r.markers)) setTimelineMarkers(r.markers);
                    onToast(
                      `Synced ${r.seeded_sections ?? 0} section range markers & ${r.seeded_lines ?? 0} line cues (FilmCraft ↔ EffectCraft)`,
                    );
                  })
                  .catch((err) => onToast(String(err)));
              }}
            >
              ⇄ Sync ({timelineMarkers.length})
            </button>
            <div className="lyrics-bg-mode-toggle">
              <button
                id="words-bg-scene"
                type="button"
                className={stageBgMode === "scene" ? "active" : ""}
                onClick={() => setStageBgMode("scene")}
              >
                Scene
              </button>
              <button
                id="words-bg-paper"
                type="button"
                className={stageBgMode === "paper" ? "active" : ""}
                onClick={() => setStageBgMode("paper")}
              >
                Paper
              </button>
            </div>
            <button
              id="words-render-studio"
              type="button"
              className="ghost"
              disabled={publishingStudio}
              onClick={() => void renderLyricsToStudio()}
            >
              {publishingStudio ? "Rendering…" : "Publish"}
            </button>
            <button
              id="words-export-film"
              type="button"
              className="primary-emerald"
              onClick={onNavigateStudio}
            >
              Studio
            </button>
          </div>

          <LyricsTimelineDock
            totalSongDuration={totalSongDuration}
            playheadSec={playheadSec}
            setPlayheadSec={setPlayheadSec}
            timelineMarkers={timelineMarkers}
            sceneTrackClips={sceneTrackClips}
            waveformPeaks={waveformPeaks}
            allLineGroups={allLineGroups}
            activeKey={activeKey}
            setSelectedLineKey={setSelectedLineKey}
            setActiveItemIndex={setActiveItemIndex}
            activeLine={activeLine}
            formatTime={formatTime}
          />
        </main>

        {/* RIGHT INSPECTOR RAIL: Unified .panel.motion-section + Dial/Slider/Toggle */}
        <LyricsInspector
          mode={mode}
          anim={activeAnim}
          lineId={activeLine?.lineId ?? null}
          lineWords={activeLine?.words ?? []}
          design={activeDesign}
          activeItemIndex={activeItemIndex}
          onSelectItemIndex={setActiveItemIndex}
          onUpdateDesign={updateActiveDesign}
          onSaveAndSync={saveAndSyncEcproj}
          moves={moves}
          letterings={letterings}
          strokes={strokes}
          choreographies={choreographies}
          easings={easings}
          textPresets={textPresets}
          drawMode={drawMode}
          onToggleDrawMode={() => setDrawMode((d) => !d)}
          brushColor={brushColor}
          onBrushColorChange={setBrushColor}
          brushWidth={brushWidth}
          onBrushWidthChange={setBrushWidth}
          onToast={onToast}
        />
      </div>
    </div>
  );
};
