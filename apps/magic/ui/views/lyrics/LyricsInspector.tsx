import React, { useCallback, useEffect, useState } from "react";
import { executeCommand } from "../../api";
import { StyleClipboard } from "../../components/StyleClipboard";
import { Dial, Slider, Toggle } from "../../components/controls";
import { LyricsMotionMarks } from "./LyricsMotionMarks";
import {
  buildDefaultLineDesign,
  EasingPreset,
  effectiveOutlineColor,
  isDarkHex,
  LetteringPreset,
  LineDesign,
  MotionPair,
  MotionProperty,
  MotionSpec,
  MovePreset,
  StrokePreset,
  TextPresetItem,
  WordItem,
  WordsAnimation,
  WordToken,
} from "./types";

const ROTATION = "transform/rotation";
const OPACITY = "transform/opacity";
const POSITION = "transform/position";
const SCALE = "transform/scale";

const FALLBACK_PROPERTIES: MotionProperty[] = [
  { path: POSITION, name: "Position" },
  { path: ROTATION, name: "Rotation" },
  { path: SCALE, name: "Scale" },
  { path: OPACITY, name: "Opacity" },
];

type Pair = MotionPair;

const finite = (value: unknown, fallback: number): number =>
  typeof value === "number" && Number.isFinite(value) ? value : fallback;

const pair = (value: unknown, fallback: Pair): Pair =>
  Array.isArray(value) && value.length >= 2
    ? [finite(value[0], fallback[0]), finite(value[1], fallback[1])]
    : fallback;

const scalar = (value: unknown, fallback: number): number =>
  typeof value === "number" && Number.isFinite(value) ? value : fallback;

function defaultMotionSpec(property: string): MotionSpec {
  if (property === POSITION) {
    return { property, delay: 0, duration: 1.2, from: [0, -40], to: [0, 0], speed: 0, easing: "cubic_out", loopMode: "play_once", firstFrame: 0 };
  }
  if (property === SCALE) {
    return { property, delay: 0, duration: 0.9, from: [130, 130], to: [100, 100], speed: 0, easing: "cubic_out", loopMode: "play_once", firstFrame: 0 };
  }
  if (property === OPACITY) {
    return { property, delay: 0, duration: 0.8, from: 0, to: 100, speed: 0, easing: "quad_out", loopMode: "play_once", firstFrame: 0 };
  }
  return { property, delay: 0, duration: 1.2, from: -12, to: 0, speed: 90, easing: "cubic_out", loopMode: "loop", firstFrame: 0 };
}

interface StoryboardSlot {
  id?: string;
  label?: string;
  kind?: string;
  text?: string;
}

interface StoryboardPage {
  page: number;
  title?: string;
  archetype?: string;
  layout?: string;
  slots?: StoryboardSlot[];
}

export interface LyricsInspectorProps {
  mode: "design" | "motion";
  anim: WordsAnimation | undefined;
  lineId: string | null;
  lineWords: WordToken[];
  design: LineDesign;
  activeItemIndex: number;
  onSelectItemIndex: (index: number) => void;
  onUpdateDesign: (updater: (draft: LineDesign) => void) => void;
  onSaveAndSync: () => Promise<void>;
  moves: MovePreset[];
  letterings: LetteringPreset[];
  strokes: StrokePreset[];
  choreographies: { id: string; name: string; does: string }[];
  easings: EasingPreset[];
  textPresets: (string | TextPresetItem)[];
  drawMode: boolean;
  onToggleDrawMode: () => void;
  brushColor: string;
  onBrushColorChange: (color: string) => void;
  brushWidth: number;
  onBrushWidthChange: (width: number) => void;
  onToast: (msg: string) => void;
}

export const LyricsInspector: React.FC<LyricsInspectorProps> = ({
  mode,
  anim,
  lineId,
  lineWords,
  design,
  activeItemIndex,
  onSelectItemIndex,
  onUpdateDesign,
  onSaveAndSync,
  moves,
  letterings,
  strokes,
  choreographies,
  easings,
  textPresets,
  drawMode,
  onToggleDrawMode,
  brushColor,
  onBrushColorChange,
  brushWidth,
  onBrushWidthChange,
  onToast,
}) => {
  const item: WordItem | undefined = design.items[activeItemIndex] ?? design.items[0];
  const itemText = (item?.word_ids ?? [])
    .map((wid) => lineWords.find((w) => w.word_id === wid)?.text)
    .filter(Boolean)
    .join(" ");

  const patchItem = useCallback(
    (patch: Partial<WordItem>) => {
      onUpdateDesign((draft) => {
        const target = draft.items[activeItemIndex] ?? draft.items[0];
        if (!target) return;
        Object.assign(target, patch);
      });
    },
    [activeItemIndex, onUpdateDesign],
  );

  // Native EffectCraft keyframe curve state
  const [motionProps, setMotionProps] = useState<MotionProperty[]>(FALLBACK_PROPERTIES);
  const [compBindingLabel, setCompBindingLabel] = useState<string>("Lyrics · EffectCraft");
  const [motionSpec, setMotionSpec] = useState<MotionSpec>(() =>
    defaultMotionSpec(ROTATION),
  );
  const [motionBusy, setMotionBusy] = useState(false);

  const wordIdsJoined = (item?.word_ids ?? []).join("|");
  useEffect(() => {
    let alive = true;
    if (!anim || !lineId || !item?.word_ids?.length) return;
    executeCommand("words.motion.properties", {
      line_id: lineId,
      word_ids: item.word_ids,
    })
      .then((res) => {
        if (!alive) return;
        const r = res as {
          properties?: MotionProperty[];
          comp?: { name?: string };
          binding?: { layer?: { name?: string } };
        };
        if (Array.isArray(r.properties) && r.properties.length > 0) {
          setMotionProps(r.properties);
        }
        if (r.binding?.layer?.name) {
          setCompBindingLabel(`${r.binding.layer.name} · ${r.comp?.name ?? "Lyrics"}`);
        } else {
          setCompBindingLabel("Lyrics · EffectCraft");
        }
      })
      .catch(() => {
        if (!alive) return;
        setCompBindingLabel("Lyrics · EffectCraft");
      });
    return () => {
      alive = false;
    };
  }, [anim, lineId, wordIdsJoined, item?.word_ids]);

  const applyNativeKeyframes = async () => {
    if (!anim || !lineId || !item?.word_ids?.length) return;
    setMotionBusy(true);
    try {
      await onSaveAndSync();
      const values =
        motionSpec.property === POSITION || motionSpec.property === SCALE
          ? {
              from: pair(motionSpec.from, [0, 0]),
              to: pair(motionSpec.to, [0, 0]),
            }
          : {
              from: scalar(motionSpec.from, 0),
              to: scalar(motionSpec.to, 0),
            };
      const req: Record<string, unknown> = {
        animation_id: anim.animation_id,
        line_id: lineId,
        word_ids: item.word_ids,
        expected_source_revision: finite(anim.source_revision, 0),
        property: motionSpec.property,
        delay: motionSpec.delay,
        duration: motionSpec.duration,
        from: values.from,
        easing: motionSpec.easing,
        loop_mode: motionSpec.loopMode,
        first_frame: motionSpec.firstFrame,
      };
      if (motionSpec.property === ROTATION) {
        req.speed = motionSpec.speed;
      } else {
        req.to = values.to;
      }
      const reply = (await executeCommand("words.motion.set", req)) as {
        ok?: boolean;
        motion_keys?: [number, unknown][];
        reason?: string;
      };
      if (!reply.ok) {
        throw new Error(reply.reason ?? "Failed to write keyframes to lyrics.ecproj");
      }
      patchItem({
        motion_keys: reply.motion_keys,
      });
      onToast(
        `Wrote ${reply.motion_keys?.length ?? 2} keyframes to lettering/lyrics.ecproj`,
      );
    } catch (err) {
      onToast(`Motion keyframe error: ${String(err)}`);
    } finally {
      setMotionBusy(false);
    }
  };

  // Storyboard pages for Scene Context
  const [sbPages, setSbPages] = useState<StoryboardPage[]>([]);
  const [selectedSbPage, setSelectedSbPage] = useState<number>(0);

  useEffect(() => {
    let alive = true;
    executeCommand("storyboard.pages", { full: true })
      .then((res) => {
        if (!alive) return;
        const pages = (res as { pages?: StoryboardPage[] })?.pages ?? [];
        setSbPages(pages);
        const firstScene = pages.find((p) => p.archetype === "scene") ?? pages[0];
        if (firstScene) setSelectedSbPage(firstScene.page);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, []);

  const currentSbPage = sbPages.find((p) => p.page === selectedSbPage) ?? sbPages[0];
  const textSlots = (currentSbPage?.slots ?? []).filter(
    (sl) => sl.kind === "text" || typeof sl.text === "string",
  );

  const rotateVal = finite(item?.rotate, 0);
  const skewVal = finite(item?.skew, 0);
  const skewAxisVal = finite(item?.skew_axis, 0);
  const scaleXVal = finite(item?.scale_x, 100);
  const scaleYVal = finite(item?.scale_y, 100);
  const sizeVal = finite(item?.size, 84);
  const trackingVal = finite(item?.tracking, 0);
  const strokeWidthVal = finite(item?.stroke_width, 6);
  const fillHex = item?.color || "#D95B38";
  const outlineHex = effectiveOutlineColor(item ?? { word_ids: [], x: 0, y: 0 }) || "#FFF8E9";
  const warpVal = item?.warp_path ?? "none";

  return (
    <aside
      id="words-effects-rail"
      className="animation-rail-page lyrics-inspector-rail"
      aria-label="Lyrics and Motion Inspector"
    >
      {/* 1. Word Group & Transform / Skew (Cutout Look) */}
      <details
        id="lyrics-transform-section"
        className="panel motion-section"
        data-look="cutout"
        open
      >
        <summary className="panel-head">
          <span className="motion-section-icon" aria-hidden="true">
            <svg viewBox="0 0 16 16">
              <path d="M3 3h4v4H3zM9 3h4v4H9zM3 9h4v4H3zM9 9h4v4H9z" />
            </svg>
          </span>
          <span className="title">Transform</span>
          <span className="meta">{itemText || "Phrase"}</span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          {/* Phrase group selector + Split / Group buttons */}
          <div className="lyrics-word-group-bar">
            <div className="lyrics-word-chips">
              {design.items.map((grp, idx) => {
                const text = grp.word_ids
                  .map((id) => lineWords.find((w) => w.word_id === id)?.text)
                  .filter(Boolean)
                  .join(" ");
                return (
                  <button
                    key={`${grp.word_ids.join("_")}_${idx}`}
                    id={`words-group-chip-${idx}`}
                    type="button"
                    className={`lyrics-chip ${idx === activeItemIndex ? "active" : ""}`}
                    onClick={() => onSelectItemIndex(idx)}
                  >
                    {text || `Group ${idx + 1}`}
                  </button>
                );
              })}
            </div>
            <div className="lyrics-group-actions">
              <button
                id="words-split-tokens"
                type="button"
                className="ghost"
                title="Split every word on this line into its own independent box"
                onClick={() => {
                  if (!lineWords.length) return;
                  onUpdateDesign((draft) => {
                    draft.items = lineWords.map((w, idx) => ({
                      word_ids: [w.word_id],
                      x: 180 + (idx % 4) * 390,
                      y: 280 + Math.floor(idx / 4) * 210,
                      w: 360,
                      h: 130,
                      lettering: idx === 0 ? "serif-italic" : "song-bold",
                      size: idx === 0 ? 96 : 74,
                      color: idx === 0 ? "#D95B38" : "#173F49",
                      outline: "#FFF8E9",
                      stroke_width: 6,
                      tracking: 0,
                      rotate: 0,
                      skew: 0,
                      skew_axis: 0,
                      scale_x: 100,
                      scale_y: 100,
                      warp_path: "none",
                      move: "pop",
                      dx: 0,
                      dy: -28,
                    }));
                  });
                  onSelectItemIndex(0);
                }}
              >
                Split
              </button>
              <button
                id="words-merge-line"
                type="button"
                className="ghost"
                title="Group words back into expressive phrase blocks"
                onClick={() => {
                  onUpdateDesign((draft) => {
                    const fresh = buildDefaultLineDesign(
                      lineWords,
                      draft.choreography || "cascade",
                    );
                    draft.items = fresh.items;
                  });
                  onSelectItemIndex(0);
                }}
              >
                Group
              </button>
            </div>
          </div>

          <StyleClipboard
            id="word-style-clipboard"
            title="Word style"
            design={design as unknown as { items: Record<string, unknown>[]; draws: unknown[] }}
            itemIndex={activeItemIndex}
            onChange={(next) =>
              onUpdateDesign((draft) => {
                draft.items = next.items as unknown as WordItem[];
              })
            }
            onToast={onToast}
          />

          <div className="lyrics-dial-grid">
            <div className="craft-number" data-control="dial">
              <label className="craft-number-title" htmlFor="words-item-rotate-dial">
                <span className="craft-control-label">Rotate (360°)</span>
                <output className="craft-number-readout">{Math.round(rotateVal)}°</output>
              </label>
              <span className="craft-number-body">
                <Dial
                  id="words-item-rotate"
                  label="Rotate"
                  value={rotateVal}
                  min={-360}
                  max={360}
                  step={1}
                  unit="°"
                  onChange={(v) => patchItem({ rotate: v })}
                />
              </span>
            </div>

            <div className="craft-number" data-control="dial">
              <label className="craft-number-title" htmlFor="words-item-skew-dial">
                <span className="craft-control-label">Skew Slant</span>
                <output className="craft-number-readout">{Math.round(skewVal)}°</output>
              </label>
              <span className="craft-number-body">
                <Dial
                  id="words-item-skew"
                  label="Skew"
                  value={skewVal}
                  min={-70}
                  max={70}
                  step={1}
                  unit="°"
                  onChange={(v) => patchItem({ skew: v })}
                />
              </span>
            </div>

            <div className="craft-number" data-control="dial">
              <label className="craft-number-title" htmlFor="words-item-skew-axis-dial">
                <span className="craft-control-label">Skew Axis</span>
                <output className="craft-number-readout">{Math.round(skewAxisVal)}°</output>
              </label>
              <span className="craft-number-body">
                <Dial
                  id="words-item-skew-axis"
                  label="Skew Axis"
                  value={skewAxisVal}
                  min={-180}
                  max={180}
                  step={1}
                  unit="°"
                  onChange={(v) => patchItem({ skew_axis: v })}
                />
              </span>
            </div>
          </div>

          {/* Quick angle presets */}
          <div className="lyrics-quick-angles">
            {[-90, -15, 0, 15, 90, 180].map((deg) => (
              <button
                key={deg}
                id={`words-angle-${deg}`}
                type="button"
                className={`ghost ${Math.round(rotateVal) === deg ? "active" : ""}`}
                onClick={() => patchItem({ rotate: deg })}
              >
                {deg > 0 ? `+${deg}°` : `${deg}°`}
              </button>
            ))}
          </div>

          <Slider
            id="words-item-scale-x"
            label="Stretch X"
            value={scaleXVal}
            min={25}
            max={350}
            step={1}
            unit="%"
            onChange={(v) => patchItem({ scale_x: v })}
          />
          <Slider
            id="words-item-scale-y"
            label="Stretch Y"
            value={scaleYVal}
            min={25}
            max={350}
            step={1}
            unit="%"
            onChange={(v) => patchItem({ scale_y: v })}
          />
          <Toggle
            id="words-item-3d"
            label="EffectCraft 3D Per-Character Layer"
            checked={Boolean(item?.per_char_3d)}
            onChange={(v) => patchItem({ per_char_3d: v })}
          />
        </div>
      </details>

      {/* 2. Lettering, Contrast & Warp Path (Chalk Look) */}
      <details
        id="lyrics-type-section"
        className="panel motion-section"
        data-look="chalk"
        open={mode === "design"}
      >
        <summary className="panel-head">
          <span className="motion-section-icon" aria-hidden="true">
            <svg viewBox="0 0 16 16">
              <path d="M3 3h10v2H9v8H7V5H3V3z" />
            </svg>
          </span>
          <span className="title">Lettering</span>
          <span className="meta">{item?.lettering ?? "serif-italic"}</span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          <div className="lyrics-lettering-pills">
            {letterings.map((letPreset) => (
              <button
                key={letPreset.id}
                id={`words-lettering-${letPreset.id}`}
                type="button"
                className={`lyrics-chip ${item?.lettering === letPreset.id ? "active" : ""}`}
                title={letPreset.does}
                onClick={() => patchItem({ lettering: letPreset.id })}
              >
                {letPreset.name}
              </button>
            ))}
          </div>

          <Slider
            id="words-item-size"
            label="Font Size"
            value={sizeVal}
            min={28}
            max={260}
            step={2}
            unit="px"
            onChange={(v) => patchItem({ size: v })}
          />

          <Slider
            id="words-item-tracking"
            label="Tracking"
            value={trackingVal}
            min={-80}
            max={400}
            step={5}
            onChange={(v) => patchItem({ tracking: v })}
          />

          <div className="lyrics-color-row">
            <label className="lyrics-color-picker">
              <span>Fill</span>
              <input
                id="words-item-color"
                type="color"
                value={fillHex.startsWith("#") ? fillHex : "#D95B38"}
                onChange={(e) => {
                  const nextColor = e.target.value;
                  const nextOutline =
                    isDarkHex(nextColor) && (!item?.outline || item.outline === "none")
                      ? "#FFF8E9"
                      : item?.outline;
                  patchItem({ color: nextColor, outline: nextOutline });
                }}
              />
            </label>
            <label className="lyrics-color-picker">
              <span>Outline</span>
              <input
                id="words-item-outline"
                type="color"
                value={outlineHex.startsWith("#") ? outlineHex : "#FFF8E9"}
                onChange={(e) => patchItem({ outline: e.target.value })}
              />
            </label>
            <button
              id="words-item-contrast-fix"
              type="button"
              className="ghost"
              title="Ensure high-contrast cream halo outline behind dark lettering"
              onClick={() =>
                patchItem({
                  outline: isDarkHex(fillHex) ? "#FFF8E9" : "#11222B",
                  stroke_width: Math.max(6, strokeWidthVal),
                })
              }
            >
              Halo
            </button>
          </div>

          <Slider
            id="words-item-stroke-width"
            label="Outline Halo"
            value={strokeWidthVal}
            min={0}
            max={28}
            step={1}
            unit="px"
            onChange={(v) => patchItem({ stroke_width: v })}
          />

          <div className="lyrics-sublabel">EffectCraft Text-on-Path Warp</div>
          <div className="lyrics-warp-pills">
            {(
              [
                ["none", "Flat"],
                ["arc_up", "⤴ Arc Up"],
                ["arc_down", "⤵ Arc Down"],
                ["wave", "〰 Wave"],
                ["circle", "◯ Orbit"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                id={`words-warp-${id}`}
                type="button"
                className={`lyrics-chip ${warpVal === id ? "active" : ""}`}
                onClick={() => patchItem({ warp_path: id })}
              >
                {label}
              </button>
            ))}
          </div>

          <div className="lyrics-sublabel">Line Choreography Layout</div>
          <div className="lyrics-warp-pills">
            {choreographies.map((ch) => (
              <button
                key={ch.id}
                id={`words-choreo-${ch.id}`}
                type="button"
                className={`lyrics-chip ${design.choreography === ch.id ? "active" : ""}`}
                title={ch.does}
                onClick={() => {
                  onUpdateDesign((draft) => {
                    const built = buildDefaultLineDesign(lineWords, ch.id);
                    draft.choreography = ch.id;
                    draft.items = built.items;
                  });
                }}
              >
                {ch.name}
              </button>
            ))}
          </div>
        </div>
      </details>

      <LyricsMotionMarks
        mode={mode}
        item={item}
        design={design}
        activeItemIndex={activeItemIndex}
        patchItem={patchItem}
        onUpdateDesign={onUpdateDesign}
        moves={moves}
        easings={easings}
        textPresets={textPresets}
        strokes={strokes}
        motionProps={motionProps}
        compBindingLabel={compBindingLabel}
        motionSpec={motionSpec}
        setMotionSpec={setMotionSpec}
        defaultMotionSpec={defaultMotionSpec}
        motionBusy={motionBusy}
        applyNativeKeyframes={applyNativeKeyframes}
        drawMode={drawMode}
        onToggleDrawMode={onToggleDrawMode}
        brushColor={brushColor}
        onBrushColorChange={onBrushColorChange}
        brushWidth={brushWidth}
        onBrushWidthChange={onBrushWidthChange}
        onToast={onToast}
      />
      {/* 5. Storyboard & Text (Cutout Look) */}
      <details
        id="lyrics-storyboard-section"
        className="panel motion-section"
        data-look="cutout"
      >
        <summary className="panel-head">
          <span className="motion-section-icon" aria-hidden="true">
            <svg viewBox="0 0 16 16">
              <rect x="2" y="3" width="12" height="10" rx="1.5" />
            </svg>
          </span>
          <span className="title">Storyboard</span>
          <span className="meta">P{(currentSbPage?.page ?? 0) + 1}</span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          <div className="lyrics-sb-bar">
            <select
              id="words-sb-page-select"
              value={selectedSbPage}
              onChange={(e) => setSelectedSbPage(Number(e.target.value))}
            >
              {sbPages.map((pg) => (
                <option key={pg.page} value={pg.page}>
                  P{pg.page + 1} · {pg.title || pg.archetype || "Scene"} ({pg.layout || "Spread"})
                </option>
              ))}
            </select>
            <button
              id="words-sb-open-dc"
              type="button"
              className="ghost"
              onClick={() =>
                void executeCommand("storyboard.open", {}).catch((e) =>
                  onToast(String(e)),
                )
              }
            >
              DesignCraft ↗
            </button>
          </div>
          {textSlots.slice(0, 6).map((slot, i) => (
            <label
              key={slot.id || slot.label || i}
              className="lyrics-sb-slot"
            >
              <span>{(slot.label || slot.id || `Slot ${i + 1}`).toUpperCase()}</span>
              <input
                type="text"
                defaultValue={slot.text ?? ""}
                onBlur={(e) => {
                  if (!currentSbPage || !slot.label) return;
                  void executeCommand("storyboard.set", {
                    page: currentSbPage.page,
                    label: slot.label,
                    text: e.target.value,
                  }).catch((err) => onToast(String(err)));
                }}
              />
            </label>
          ))}
        </div>
      </details>
    </aside>
  );
};
