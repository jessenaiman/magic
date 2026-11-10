import React, { useEffect, useRef, useState } from "react";
import { executeCommand, fileUrl, pickFile } from "../api";

interface MotionWordChip {
  word_id: string;
  line_id: string;
  text: string;
  start: number;
  end: number;
  clock: string;
  has_action: boolean;
}

interface MotionLine {
  line_id: string;
  section: string;
  start: number;
  clock: string;
  words: MotionWordChip[];
}

interface MotionCue {
  id: string;
  word_id: string;
  word: string;
  title: string;
  d: string;
  svg_path?: string | null;
  color: string;
  stroke_width: number;
  start: number;
  end: number;
  clock: string;
  draw_duration: number;
  hold_duration: number;
}

interface MotionPreset {
  id: string;
  name: string;
  d: string;
}

interface MotionReadPayload {
  ok: boolean;
  ecproj?: string | null;
  render?: string | null;
  anicraft_available?: boolean;
  duration: number;
  lines: MotionLine[];
  cues: MotionCue[];
  presets: MotionPreset[];
  backdrop?: { path?: string } | null;
}

interface MotionViewProps {
  projectPath: string;
  onNavigateStudio: () => void;
  onToast: (msg: string) => void;
}

/**
 * Dedicated Song-Action Drawing view (`motion/actions.ecproj` -> FilmCraft V4 `"Action Motion"`).
 *
 * Separated from `LyricsView` (Kinetic Teaching Lyrics on V3): binds drawn or `VectorCraft`-traced
 * figurative illustrations (e.g., drawing a Whale while `"Whale"` is sung) to exact timestamps from
 * `timing-reference.json`, animates them with `EffectCraft` Trim Paths, and publishes onto FilmCraft V4.
 */
export const MotionView: React.FC<MotionViewProps> = ({
  projectPath,
  onNavigateStudio,
  onToast,
}) => {
  const [state, setState] = useState<MotionReadPayload | null>(null);
  const [selectedWord, setSelectedWord] = useState<MotionWordChip | null>(null);
  const [pathD, setPathD] = useState<string>("");
  const [color, setColor] = useState<string>("#64CBFF");
  const [strokeWidth, setStrokeWidth] = useState<number>(8);
  const [drawDuration, setDrawDuration] = useState<number>(0.85);
  const [holdDuration, setHoldDuration] = useState<number>(1.2);
  const [busy, setBusy] = useState<boolean>(false);
  const drawingRef = useRef<boolean>(false);
  const svgRef = useRef<SVGSVGElement | null>(null);

  const load = async () => {
    try {
      const res = await executeCommand<MotionReadPayload>("motion.read", {
        _expected_project: projectPath,
      });
      setState(res);
      if (!selectedWord && res.lines.length > 0 && res.lines[0].words.length > 0) {
        const firstCueWord =
          res.lines.flatMap((l) => l.words).find((w) => w.has_action) || res.lines[0].words[0];
        selectWord(firstCueWord, res.cues, res.presets);
      }
    } catch (err) {
      onToast(String(err));
    }
  };

  useEffect(() => {
    setSelectedWord(null);
    void load();
  }, [projectPath]);

  const selectWord = (word: MotionWordChip, cues = state?.cues || [], presets = state?.presets || []) => {
    setSelectedWord(word);
    const existing = cues.find((c) => c.word_id === word.word_id);
    if (existing) {
      setPathD(existing.d || "");
      setColor(existing.color || "#64CBFF");
      setStrokeWidth(existing.stroke_width || 8);
      setDrawDuration(existing.draw_duration || 0.85);
      setHoldDuration(existing.hold_duration || 1.2);
    } else {
      const lower = word.text.toLowerCase().replace(/[^a-z]/g, "");
      const matchingPreset = presets.find((p) => p.id === lower);
      setPathD(matchingPreset?.d || "");
    }
  };

  const toStageCoords = (e: React.PointerEvent<SVGSVGElement>): [number, number] => {
    const svg = svgRef.current;
    if (!svg) return [960, 540];
    const rect = svg.getBoundingClientRect();
    const x = Math.round(((e.clientX - rect.left) / Math.max(1, rect.width)) * 1920);
    const y = Math.round(((e.clientY - rect.top) / Math.max(1, rect.height)) * 1080);
    return [x, y];
  };

  const runAction = async (fn: () => Promise<void>) => {
    if (busy) return;
    setBusy(true);
    try {
      await fn();
      await load();
    } catch (err) {
      onToast(String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleSaveCue = () =>
    void runAction(async () => {
      if (!selectedWord) {
        onToast("Select a sung word on the left first");
        return;
      }
      if (!pathD.trim()) {
        onToast("Draw a stroke on the canvas or stamp a preset first");
        return;
      }
      await executeCommand("motion.save", {
        word_id: selectedWord.word_id,
        word: selectedWord.text,
        d: pathD,
        color,
        stroke_width: strokeWidth,
        draw_duration: drawDuration,
        hold_duration: holdDuration,
        _expected_project: projectPath,
      });
      onToast(`Saved "${selectedWord.text}" action cue into motion/actions.ecproj`);
    });

  const handleTraceImage = () =>
    void runAction(async () => {
      if (!selectedWord) {
        onToast("Select a sung word on the left first");
        return;
      }
      const chosen = await pickFile({
        title: `Trace illustration for "${selectedWord.text}" via VectorCraft`,
        extensions: ["png", "jpg", "jpeg", "webp", "svg"],
      });
      if (typeof chosen !== "string" || !chosen) return;
      await executeCommand("motion.save", {
        word_id: selectedWord.word_id,
        word: selectedWord.text,
        image: chosen,
        color,
        stroke_width: strokeWidth,
        draw_duration: drawDuration,
        hold_duration: holdDuration,
        _expected_project: projectPath,
      });
      onToast(`Traced "${selectedWord.text}" via VectorCraft into motion/actions.ecproj`);
    });

  const handlePublishV4 = () =>
    void runAction(async () => {
      await executeCommand("motion.publish", {
        render: true,
        _expected_project: projectPath,
      });
      onToast("Published Action Motion (V4) track to FilmCraft");
      onNavigateStudio();
    });

  const activeCue = state?.cues.find((c) => c.word_id === selectedWord?.word_id);
  const backdropSrc = state?.backdrop?.path ? fileUrl(state.backdrop.path) : "";

  return (
    <div id="view-motion" className="motion-view-layout">
      {/* Left Rail: Timed Song Words from timing-reference.json */}
      <aside className="sb-panel" style={{ display: "flex", flexDirection: "column", gap: 8, padding: 10, overflowY: "auto" }}>
        <header>
          <h2 style={{ margin: 0, fontSize: 14 }}>Action Words</h2>
          <p className="muted" style={{ margin: 0, fontSize: 11 }}>
            Click a sung word (e.g. &ldquo;Whale&rdquo;) to draw its action on V4.
          </p>
        </header>

        {!state?.lines.length ? (
          <p className="muted" style={{ fontSize: 12 }}>
            No timed transcript yet — run <code>audio.analyze</code> or <code>workshop.import_transcript</code>.
          </p>
        ) : (
          state.lines.map((line) => (
            <div key={line.line_id} style={{ paddingBottom: 6, borderBottom: "1px solid rgba(255,255,255,0.06)" }}>
              <div className="muted" style={{ fontSize: 10, marginBottom: 3 }}>
                {line.clock} · {line.section}
              </div>
              <div>
                {line.words.map((w) => (
                  <button
                    key={w.word_id}
                    id={`motion-word-${w.word_id}`}
                    type="button"
                    className={`motion-word-chip ${selectedWord?.word_id === w.word_id ? "selected" : ""} ${w.has_action ? "has-cue" : ""}`}
                    onClick={() => selectWord(w)}
                    title={`${w.text} @ ${w.clock}`}
                  >
                    <span>{w.text}</span>
                  </button>
                ))}
              </div>
            </div>
          ))
        )}
      </aside>

      {/* Center Stage: Interactive 1920x1080 Action Stroke Canvas */}
      <main className="sb-panel" style={{ display: "flex", flexDirection: "column", gap: 8, padding: 10, minWidth: 0 }}>
        <header style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 8 }}>
          <strong>
            {selectedWord
              ? `"${selectedWord.text}" @ ${selectedWord.clock}`
              : "Song-Action Motion Canvas (1920×1080)"}
          </strong>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            {(state?.presets || []).map((preset) => (
              <button
                key={preset.id}
                id={`motion-preset-${preset.id}`}
                type="button"
                className="ghost magic-btn-icon"
                onClick={() => setPathD(preset.d)}
                title={`Stamp ${preset.name} vector silhouette`}
              >
                <span>✦</span>
                <span>{preset.name}</span>
              </button>
            ))}
            <button
              id="motion-trace-image"
              type="button"
              className="ghost magic-btn-icon"
              disabled={busy || !selectedWord}
              onClick={handleTraceImage}
              title="Trace raster illustration via VectorCraft (ignoreWhite)"
            >
              <span>◈</span>
              <span>Trace</span>
            </button>
            <button
              id="motion-clear-canvas"
              type="button"
              className="ghost magic-btn-icon"
              disabled={busy || !pathD}
              onClick={() => setPathD("")}
            >
              <span>✕</span>
              <span>Clear</span>
            </button>
          </div>
        </header>

        <div className="motion-stage-wrap">
          {backdropSrc && (
            <img
              src={backdropSrc}
              alt="Scene plate"
              style={{ width: "100%", height: "100%", objectFit: "cover", opacity: 0.35 }}
            />
          )}
          <svg
            ref={svgRef}
            className="motion-stage-svg"
            viewBox="0 0 1920 1080"
            onPointerDown={(e) => {
              if (!selectedWord) return;
              drawingRef.current = true;
              e.currentTarget.setPointerCapture(e.pointerId);
              const [x, y] = toStageCoords(e);
              setPathD((prev) => `${prev ? `${prev} ` : ""}M ${x} ${y}`);
            }}
            onPointerMove={(e) => {
              if (!drawingRef.current) return;
              const [x, y] = toStageCoords(e);
              setPathD((prev) => `${prev} L ${x} ${y}`);
            }}
            onPointerUp={() => {
              drawingRef.current = false;
            }}
          >
            {pathD && (
              <path
                d={pathD}
                fill="none"
                stroke={color}
                strokeWidth={strokeWidth}
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            )}
          </svg>
        </div>
      </main>

      {/* Right Rail: EffectCraft Trim Timing + Craft App Handoff + V4 Track Export */}
      <aside className="sb-panel" style={{ display: "flex", flexDirection: "column", gap: 10, padding: 10 }}>
        <strong style={{ fontSize: 13 }}>Stroke &amp; Track</strong>

        <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: 12 }}>
          <span>Color</span>
          <input type="color" value={color} onChange={(e) => setColor(e.target.value)} />
        </label>

        <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: 12 }}>
          <span>Width ({strokeWidth}px)</span>
          <input
            type="range"
            min={2}
            max={32}
            value={strokeWidth}
            onChange={(e) => setStrokeWidth(Number(e.target.value))}
          />
        </label>

        <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: 12 }}>
          <span>Draw ({drawDuration.toFixed(2)}s)</span>
          <input
            type="range"
            min={0.2}
            max={4.0}
            step={0.05}
            value={drawDuration}
            onChange={(e) => setDrawDuration(Number(e.target.value))}
          />
        </label>

        <label style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: 12 }}>
          <span>Hold ({holdDuration.toFixed(2)}s)</span>
          <input
            type="range"
            min={0.2}
            max={6.0}
            step={0.1}
            value={holdDuration}
            onChange={(e) => setHoldDuration(Number(e.target.value))}
          />
        </label>

        <div style={{ display: "flex", gap: 6 }}>
          <button
            id="motion-save-cue"
            type="button"
            className="start magic-btn-icon"
            style={{ flex: 1 }}
            disabled={busy || !selectedWord}
            onClick={handleSaveCue}
          >
            <span>✓</span>
            <span>Save</span>
          </button>
          {activeCue && (
            <button
              id="motion-delete-cue"
              type="button"
              className="ghost magic-btn-icon"
              disabled={busy}
              onClick={() =>
                void runAction(async () => {
                  await executeCommand("motion.save", {
                    remove_id: activeCue.id,
                    _expected_project: projectPath,
                  });
                  setPathD("");
                  onToast("Removed action cue");
                })
              }
            >
              <span>✕</span>
              <span>Delete</span>
            </button>
          )}
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 6, paddingTop: 8, borderTop: "1px solid rgba(255,255,255,0.08)" }}>
          <span className="muted" style={{ fontSize: 11 }}>
            Edit in ArtCraft (--control)
          </span>
          <button
            id="motion-open-vectorcraft"
            type="button"
            className="ghost magic-btn-icon"
            disabled={busy}
            onClick={() =>
              void executeCommand("motion.open", {
                app: "vectorcraft",
                id: activeCue?.id,
                _expected_project: projectPath,
              }).catch((e) => onToast(String(e)))
            }
          >
            <span>◈</span>
            <span>VectorCraft</span>
          </button>
          <button
            id="motion-open-effectcraft"
            type="button"
            className="ghost magic-btn-icon"
            disabled={busy}
            onClick={() =>
              void executeCommand("motion.open", {
                app: "effectcraft",
                _expected_project: projectPath,
              }).catch((e) => onToast(String(e)))
            }
          >
            <span>✦</span>
            <span>EffectCraft</span>
          </button>
          {state?.anicraft_available && (
            <span
              id="motion-anicraft-cli-badge"
              className="muted"
              style={{ fontSize: "var(--text-xs)", alignSelf: "center" }}
              title="AniCraft is early alpha: Magic invokes anicraft-cli headlessly and previews inside EffectCraft & FilmCraft"
            >
              anicraft-cli ready (headless)
            </span>
          )}
        </div>

        <div style={{ marginTop: "auto" }}>
          <button
            id="motion-publish-v4"
            type="button"
            className="start magic-btn-icon"
            style={{ width: "100%" }}
            disabled={busy || !(state?.cues.length)}
            onClick={handlePublishV4}
            title="Render motion/actions.ecproj (ProRes 4444 RGBA) and publish onto FilmCraft V4 Action Motion"
          >
            <span>⬆</span>
            <span>Publish</span>
          </button>
        </div>
      </aside>
    </div>
  );
};
