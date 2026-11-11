import React, { useEffect, useState } from "react";
import {
  AnimationStyle,
  Character3DClip,
  CharacterMetaResponse,
  CharacterPart,
  CharacterSpriteSummary,
  CharacterViewSummary,
  executeCommand,
} from "../../api";
import { Slider, Toggle } from "../../components/controls";
import type { RigKeys } from "./AnimationTheatre";
import { AnimationActionPanel } from "./AnimationActionPanel";
import { AnimationRigPartsPanel } from "./AnimationRigPartsPanel";
import { AnimationPuppetPanel } from "./AnimationPuppetPanel";

interface AnimationPoseInspectorProps {
  selectedCharId: string;
  selectedView: string;
  animationStyle: AnimationStyle;
  onSelectView: (view: string) => void;
  onChangeAnimationStyle: (style: AnimationStyle) => void;
  selectedBone: string;
  onSelectBone: (bone: string) => void;
  meta: CharacterMetaResponse | null;
  sprites: CharacterSpriteSummary[];
  selectedSprite: CharacterSpriteSummary | null;
  onSelectSprite: (sprite: CharacterSpriteSummary | null) => void;
  activeActionName: string;
  onTriggerAction: (actionName: string) => void;
  onRefreshCharacter: () => Promise<void>;
  pathPointIdx: number;
  onSelectPathPoint: (idx: number) => void;
  pathX: number;
  onChangePathX: (val: number) => void;
  pathY: number;
  onChangePathY: (val: number) => void;
  pathDuration: number;
  onChangePathDuration: (val: number) => void;
  sizeStart: number;
  onChangeSizeStart: (val: number) => void;
  sizeEnd: number;
  onChangeSizeEnd: (val: number) => void;
  isLooping: boolean;
  onToggleLoop: (val: boolean) => void;
  playbackSpeed: number;
  onChangePlaybackSpeed: (val: number) => void;
  motionAmount: number;
  onChangeMotionAmount: (val: number) => void;
  rotation: number;
  onChangeRotation: (val: number) => void;
  onCommitRotation: (val: number) => void;
  scale: number;
  onChangeScale: (val: number) => void;
  onCommitScale: (val: number) => void;
  opacity: number;
  onChangeOpacity: (val: number) => void;
  onCommitOpacity: (val: number) => void;
  onKeyTransform: (propsPayload?: Record<string, unknown>) => void;
  availableActions: Array<{ id: string; label: string }>;
  rigKeys: RigKeys | null;
  currentTime: number;
  pinMode: "position" | "starch" | "overlap" | "bend";
  onChangePinMode: (mode: "position" | "starch" | "overlap" | "bend") => void;
  onClearPath: () => void;
  onUndoPath: () => void;
  onApplyPath: () => void;
  onRenderClip: () => void;
  placeTrack: string;
  onChangePlaceTrack: (val: string) => void;
  placeTime: number;
  onChangePlaceTime: (val: number) => void;
  lastRenderedClip: string;
  onPlaceOnTimeline: () => void;
  busy: boolean;
  onToast: (msg: string) => void;
}

const FALLBACK_ACTIONS = [
  { id: "walk_left", btnId: "motion-action-left", label: "Left" },
  { id: "walk_right", btnId: "motion-action-right", label: "Right" },
  { id: "walk_up", btnId: "motion-action-up", label: "Up" },
  { id: "walk_down", btnId: "motion-action-down", label: "Down" },
  { id: "idle", btnId: "motion-action-idle", label: "Idle" },
  { id: "jump", btnId: "motion-action-jump", label: "Jump" },
  { id: "turn", btnId: "motion-action-sway", label: "Turn" },
  { id: "wave", btnId: "motion-action-wave", label: "Wave" },
  { id: "rest", btnId: "motion-action-rest", label: "Rest" },
] as const;

export const AnimationPoseInspector: React.FC<AnimationPoseInspectorProps> = ({
  selectedCharId,
  selectedView,
  animationStyle,
  onSelectView,
  onChangeAnimationStyle,
  selectedBone,
  onSelectBone,
  meta,
  sprites,
  selectedSprite,
  onSelectSprite,
  activeActionName,
  onTriggerAction,
  onRefreshCharacter,
  pathPointIdx,
  onSelectPathPoint,
  pathX,
  onChangePathX,
  pathY,
  onChangePathY,
  pathDuration,
  onChangePathDuration,
  sizeStart,
  onChangeSizeStart,
  sizeEnd,
  onChangeSizeEnd,
  isLooping,
  onToggleLoop,
  playbackSpeed,
  onChangePlaybackSpeed,
  motionAmount,
  onChangeMotionAmount,
  rotation,
  onChangeRotation,
  onCommitRotation,
  scale,
  onChangeScale,
  onCommitScale,
  opacity,
  onChangeOpacity,
  onCommitOpacity,
  onKeyTransform,
  availableActions,
  rigKeys,
  currentTime,
  pinMode,
  onChangePinMode,
  onClearPath,
  onUndoPath,
  onApplyPath,
  onRenderClip,
  placeTrack,
  onChangePlaceTrack,
  placeTime,
  onChangePlaceTime,
  lastRenderedClip,
  onPlaceOnTimeline,
  busy,
  onToast,
}) => {
  // Layers state
  const [layerDepth, setLayerDepth] = useState<number>(0);
  const [layerReason, setLayerReason] = useState<string>("");
  const [hiddenParts, setHiddenParts] = useState<Record<string, boolean>>({});

  // Part artwork drawer tab
  const [partsTab, setPartsTab] = useState<"parts" | "compare">("parts");

  // Part coordinate state
  const [partPosX, setPartPosX] = useState<number>(0);
  const [partPosY, setPartPosY] = useState<number>(0);
  const [partScaleX, setPartScaleX] = useState<number>(100);
  const [partScaleY, setPartScaleY] = useState<number>(100);
  const [partPivotX, setPartPivotX] = useState<number>(0);
  const [partPivotY, setPartPivotY] = useState<number>(0);

  // Action authoring & scene placement
  const [actionLabel, setActionLabel] = useState<string>(
    selectedSprite?.name || activeActionName || "idle"
  );
  const [actionDuration, setActionDuration] = useState<number>(2.0);
  const [placeRate, setPlaceRate] = useState<number>(1.0);
  const [placeX, setPlaceX] = useState<number>(960);
  const [placeY, setPlaceY] = useState<number>(540);
  const [placeW, setPlaceW] = useState<number>(520);
  const [placeH, setPlaceH] = useState<number>(520);
  const [poseRef, setPoseRef] = useState<string>("rest");

  const activeViewSummary: CharacterViewSummary | undefined =
    meta?.views?.find((v) => v.view === selectedView) || meta?.views?.[0];
  const parts: CharacterPart[] = activeViewSummary?.parts || meta?.parts || [];
  const activePart = parts.find((p) => p.id === selectedBone) || parts[0];

  useEffect(() => {
    const joint = activePart?.joint;
    if (Array.isArray(joint) && joint.length === 2) {
      setPartPosX(Math.round(joint[0]));
      setPartPosY(Math.round(joint[1]));
      setPartPivotX(Math.round(joint[0]));
      setPartPivotY(Math.round(joint[1]));
    }
    const layerEntry = rigKeys?.layers?.find((l) => l.name === selectedBone);
    if (layerEntry?.channels) {
      const findChan = (p: string) =>
        layerEntry.channels.find((c) => c.path === p || c.name?.toLowerCase() === p);
      const firstVal = (p: string) => {
        const ch = findChan(p);
        return ch?.keys?.[0]?.value ?? ch?.samples?.[0]?.value;
      };
      const posVal = firstVal("transform/position");
      if (Array.isArray(posVal) && posVal.length >= 2) {
        setPartPosX(Math.round(Number(posVal[0]) || 0));
        setPartPosY(Math.round(Number(posVal[1]) || 0));
      }
      const ancVal = firstVal("transform/anchor");
      if (Array.isArray(ancVal) && ancVal.length >= 2) {
        setPartPivotX(Math.round(Number(ancVal[0]) || 0));
        setPartPivotY(Math.round(Number(ancVal[1]) || 0));
      }
      const scVal = firstVal("transform/scale");
      if (Array.isArray(scVal) && scVal.length >= 2) {
        setPartScaleX(Math.round(Number(scVal[0]) || 100));
        setPartScaleY(Math.round(Number(scVal[1]) || 100));
      }
    }
  }, [selectedBone, activePart, rigKeys]);

  const actionsList = availableActions.length > 0
    ? [
        ...availableActions.map((a) => ({
          id: a.id,
          btnId: `motion-action-${a.id.replace(/^walk_/, "")}`,
          label: a.label,
        })),
        { id: "rest", btnId: "motion-action-rest", label: "Rest" },
      ]
    : [...FALLBACK_ACTIONS];

  const isMarked = Boolean(activeViewSummary?.marked || parts.length > 0);
  const isCut = Boolean(activeViewSummary?.cut);
  const isRigged = Boolean(activeViewSummary?.rigged || activeViewSummary?.ecproj);
  const hasSpritesOr3D = Boolean(
    sprites.length > 0 || (meta?.model_3d && (meta.model_3d as { available?: boolean }).available)
  );
  const passedChecks = [isMarked, isCut, isRigged, hasSpritesOr3D].filter(Boolean).length;

  const clips3D: Character3DClip[] = meta?.model_3d?.clips || [];
  const modelClipNames: string[] = meta?.model_3d?.model_clips || [];
  const drawnViews = (meta?.views || []).filter((v) => Boolean(v.drawing || v.parts?.length || v.ecproj));
  const hasMultipleDrawnViews = drawnViews.length > 1;
  const sprite2DCycles = sprites.filter((s) => s.animation_style === "sprite_2d");
  const rig25DCycles = sprites.filter((s) => s.animation_style !== "sprite_2d");

  const runPrepareStep = async (cmd: string, label: string) => {
    try {
      onToast(`${label} (${selectedCharId} · ${selectedView})…`);
      await executeCommand(cmd, { character: selectedCharId, view: selectedView });
      await onRefreshCharacter();
      onToast(`${label} complete`);
    } catch (err) {
      onToast(String(err));
    }
  };

  const runAssembleEffectRig = async (openGui: boolean, is3D: boolean) => {
    try {
      if (openGui) {
        onToast(`Opening ${selectedCharId} in EffectCraft (--control)…`);
        await executeCommand("character.open", {
          character: selectedCharId,
          view: selectedView,
          app: "effectcraft",
        });
        onToast(`Opened ${selectedCharId} in EffectCraft`);
        return;
      }
      onToast(
        is3D
          ? `Building EffectCraft advanced3D composition (${selectedCharId})…`
          : `Building EffectCraft 2.5D layered puppet rig (${selectedCharId})…`
      );
      if (is3D) {
        await executeCommand("character.model3d.animate", {
          character: selectedCharId,
        });
      } else {
        await executeCommand("character.rig", {
          character: selectedCharId,
          view: selectedView,
        });
      }
      await onRefreshCharacter();
      onToast(`EffectCraft composition ready (${selectedCharId})`);
    } catch (err) {
      onToast(String(err));
    }
  };

  return (
    <aside
      id="motion-inspector"
      className="motion-inspector"
      data-ui-config="motion-inspector"
      data-animation-style={animationStyle}
      aria-label="Animation controls"
    >
      {/* 0. ANIMATION STYLE BAR (Sprite vs 2.5D Layered Rig vs 3D Model Animations) */}
      <div
        id="anim-style-bar"
        className="panel"
        style={{ padding: "var(--space-2) var(--space-3)", marginBottom: "var(--space-2)" }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "6px",
          }}
        >
          <span className="title" style={{ fontSize: "var(--text-xs)", fontWeight: 600 }}>
            Animation Style
          </span>
          <span id="anim-style-badge" className="meta">
            {animationStyle === "sprite_2d"
              ? "Sprite Sheet"
              : animationStyle === "rig_3d"
                ? "3D Model Animations"
                : "2.5D Cutout Rig"}
          </span>
        </div>
        <div className="segmented" role="tablist" aria-label="Character animation style">
          <button
            id="style-tab-sprite-2d"
            type="button"
            role="tab"
            aria-selected={animationStyle === "sprite_2d"}
            className={animationStyle === "sprite_2d" ? "active" : ""}
            onClick={() => onChangeAnimationStyle("sprite_2d")}
          >
            Sprite
          </button>
          <button
            id="style-tab-cutout-2-5d"
            type="button"
            role="tab"
            aria-selected={animationStyle === "cutout_2_5d" || animationStyle === "cutout"}
            className={
              animationStyle === "cutout_2_5d" || animationStyle === "cutout" ? "active" : ""
            }
            onClick={() => onChangeAnimationStyle("cutout_2_5d")}
          >
            Rig
          </button>
          <button
            id="style-tab-rig-3d"
            type="button"
            role="tab"
            aria-selected={animationStyle === "rig_3d"}
            className={animationStyle === "rig_3d" ? "active" : ""}
            onClick={() => onChangeAnimationStyle("rig_3d")}
          >
            3D
          </button>
        </div>
      </div>

      <AnimationActionPanel
        selectedCharId={selectedCharId}
        selectedView={selectedView}
        onSelectView={onSelectView}
        animationStyle={animationStyle}
        parts={parts}
        drawnViews={drawnViews}
        hasMultipleDrawnViews={hasMultipleDrawnViews}
        sprites={sprites}
        sprite2DCycles={sprite2DCycles}
        rig25DCycles={rig25DCycles}
        clips3D={clips3D}
        modelClipNames={modelClipNames}
        selectedSprite={selectedSprite}
        onSelectSprite={onSelectSprite}
        activeActionName={activeActionName}
        onTriggerAction={onTriggerAction}
        actionsList={actionsList}
        actionLabel={actionLabel}
        setActionLabel={setActionLabel}
        actionDuration={actionDuration}
        setActionDuration={setActionDuration}
        isLooping={isLooping}
        onToggleLoop={onToggleLoop}
        placeTrack={placeTrack}
        onChangePlaceTrack={onChangePlaceTrack}
        placeTime={placeTime}
        onChangePlaceTime={onChangePlaceTime}
        placeRate={placeRate}
        setPlaceRate={setPlaceRate}
        placeX={placeX}
        setPlaceX={setPlaceX}
        placeY={placeY}
        setPlaceY={setPlaceY}
        placeW={placeW}
        setPlaceW={setPlaceW}
        placeH={placeH}
        setPlaceH={setPlaceH}
        lastRenderedClip={lastRenderedClip}
        onPlaceOnTimeline={onPlaceOnTimeline}
        runAssembleEffectRig={runAssembleEffectRig}
        busy={busy}
      />
      {/* 2. PATH & DEPTH */}
      <details id="motion-path-section" className="panel motion-section" data-look="depth">
        <summary className="panel-head" data-look="section">
          <span className="ico" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
              <path d="M12 20h9" />
              <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
            </svg>
          </span>
          <span className="title">Path</span>
          <span id="motion-path-meta" className="meta">
            {sizeStart}%→{sizeEnd}%
          </span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          <p className="motion-help">
            Draw a walk path on the stage or adjust start/end depth scale for perspective travel.
          </p>
          <label className="seq-field">
            <span>Path root</span>
            <select
              id="motion-path-layer"
              value={selectedBone}
              onChange={(e) => onSelectBone(e.target.value)}
            >
              <option value="root">root</option>
              {parts.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.id}
                </option>
              ))}
            </select>
          </label>
          <label className="seq-field">
            <span>Edit point</span>
            <select
              id="motion-path-point"
              value={pathPointIdx}
              onChange={(e) => onSelectPathPoint(Number(e.target.value))}
            >
              <option value={0}>Point 1 (Start)</option>
              <option value={1}>Point 2 (Mid)</option>
              <option value={2}>Point 3 (End)</option>
            </select>
          </label>
          <div className="craft-axis-controls">
            <Slider
              id="motion-path-x"
              label="Point X"
              value={pathX}
              min={0}
              max={1920}
              step={1}
              unit="px"
              onChange={onChangePathX}
            />
            <Slider
              id="motion-path-y"
              label="Point Y"
              control="vertical"
              value={pathY}
              min={0}
              max={1080}
              step={1}
              unit="px"
              onChange={onChangePathY}
            />
          </div>
          <Slider
            id="motion-path-duration"
            label="Travel duration"
            value={pathDuration}
            min={0.2}
            max={12}
            step={0.1}
            unit="s"
            onChange={onChangePathDuration}
          />
          <Slider
            id="motion-size-start"
            label="Start size"
            value={sizeStart}
            min={20}
            max={220}
            step={5}
            unit="%"
            onChange={onChangeSizeStart}
          />
          <Slider
            id="motion-size-end"
            label="End size"
            value={sizeEnd}
            min={20}
            max={220}
            step={5}
            unit="%"
            onChange={onChangeSizeEnd}
          />
          <div className="motion-path-actions">
            <button
              id="motion-path-clear"
              type="button"
              className="ghost"
              onClick={onClearPath}
            >
              Clear
            </button>
            <button
              id="motion-path-undo"
              type="button"
              className="ghost"
              onClick={onUndoPath}
            >
              Undo
            </button>
            <button
              id="motion-path-apply"
              type="button"
              className="start"
              disabled={busy}
              onClick={onApplyPath}
            >
              Apply
            </button>
          </div>
        </div>
      </details>

      {/* 3. MOTION & TIMING */}
      <details id="motion-timing" className="panel motion-section" data-look="motion">
        <summary className="panel-head" data-look="section">
          <span className="ico" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="currentColor">
              <path d="M8 5v14l11-7z" />
            </svg>
          </span>
          <span className="title">Timing</span>
          <span id="motion-timing-meta" className="meta">
            {playbackSpeed.toFixed(2)}×
          </span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          <Toggle
            id="motion-preview-loop"
            label="Loop preview"
            checked={isLooping}
            onChange={onToggleLoop}
          />
          <Slider
            id="motion-speed"
            label="Playback speed"
            value={playbackSpeed}
            min={0.25}
            max={2.25}
            step={0.05}
            unit="×"
            onChange={onChangePlaybackSpeed}
          />
          <Slider
            id="motion-amount"
            label="Motion amount"
            value={motionAmount}
            min={0}
            max={100}
            step={5}
            unit="%"
            onChange={onChangeMotionAmount}
          />
        </div>
      </details>

      <AnimationRigPartsPanel
        selectedCharId={selectedCharId}
        selectedView={selectedView}
        onSelectView={onSelectView}
        selectedBone={selectedBone}
        onSelectBone={onSelectBone}
        parts={parts}
        activePart={activePart}
        activeViewSummary={activeViewSummary}
        drawnViews={drawnViews}
        hasMultipleDrawnViews={hasMultipleDrawnViews}
        layerDepth={layerDepth}
        setLayerDepth={setLayerDepth}
        layerReason={layerReason}
        setLayerReason={setLayerReason}
        hiddenParts={hiddenParts}
        setHiddenParts={setHiddenParts}
        partsTab={partsTab}
        setPartsTab={setPartsTab}
        isMarked={isMarked}
        isCut={isCut}
        isRigged={isRigged}
        runPrepareStep={runPrepareStep}
        busy={busy}
        onToast={onToast}
      />

      <AnimationPuppetPanel
        selectedCharId={selectedCharId}
        selectedView={selectedView}
        onSelectView={onSelectView}
        selectedBone={selectedBone}
        activePart={activePart}
        drawnViews={drawnViews}
        hasMultipleDrawnViews={hasMultipleDrawnViews}
        hiddenParts={hiddenParts}
        setHiddenParts={setHiddenParts}
        partPosX={partPosX}
        setPartPosX={setPartPosX}
        partPosY={partPosY}
        setPartPosY={setPartPosY}
        partScaleX={partScaleX}
        setPartScaleX={setPartScaleX}
        partScaleY={partScaleY}
        setPartScaleY={setPartScaleY}
        partPivotX={partPivotX}
        setPartPivotX={setPartPivotX}
        partPivotY={partPivotY}
        setPartPivotY={setPartPivotY}
        rotation={rotation}
        onChangeRotation={onChangeRotation}
        onCommitRotation={onCommitRotation}
        scale={scale}
        onChangeScale={onChangeScale}
        onCommitScale={onCommitScale}
        opacity={opacity}
        onChangeOpacity={onChangeOpacity}
        onCommitOpacity={onCommitOpacity}
        onKeyTransform={onKeyTransform}
        currentTime={currentTime}
        pinMode={pinMode}
        onChangePinMode={onChangePinMode}
        onRefreshCharacter={onRefreshCharacter}
        busy={busy}
        onToast={onToast}
      />
      {/* 7. QUALITY CHECKS */}
      <details id="motion-quality" className="panel motion-section" data-look="check">
        <summary className="panel-head" data-look="section">
          <span className="ico" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
              <path d="M20 6 9 17l-5-5" />
            </svg>
          </span>
          <span className="title">Checks</span>
          <span id="motion-quality-meta" className="meta">
            {passedChecks}/4
          </span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          <p id="motion-quality-summary" className="motion-help">
            {passedChecks}/4 asset structure checks passed
          </p>
          <ul id="motion-checks" className="companion-state">
            <li>{isMarked ? "✓" : "✗"} Parts and joints marked (Scribbles)</li>
            <li>{isCut ? "✓" : "✗"} Parts cut into layers (PhotoCraft)</li>
            <li>{isRigged ? "✓" : "✗"} Puppet rigged (EffectCraft)</li>
            <li>{hasSpritesOr3D ? "✓" : "✗"} Action cycles / 3D rig ready</li>
          </ul>
        </div>
      </details>

      {/* 8. EXPORT & INTEGRATIONS */}
      <details id="motion-export-section" className="panel motion-section" data-look="export">
        <summary className="panel-head" data-look="section">
          <span className="ico" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
          </span>
          <span className="title">Export</span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          <button
            id="motion-export"
            type="button"
            className="start anim-full-btn"
            disabled={busy}
            onClick={onRenderClip}
          >
            {busy ? "Rendering…" : "Save animation (.mov)"}
          </button>
          <span id="motion-export-status" className="muted" role="status">
            {lastRenderedClip
              ? `Saved: ${lastRenderedClip.split("/").pop()}`
              : "Renders ProRes 4444 with alpha via EffectCraft."}
          </span>

          <div className="anim-two-col">
            <label className="anim-field-label">
              <span>Track</span>
              <select value={placeTrack} onChange={(e) => onChangePlaceTrack(e.target.value)}>
                <option value="V2">V2 (Characters)</option>
                <option value="V1">V1 (Background)</option>
                <option value="V3">V3 (Overlays)</option>
              </select>
            </label>
            <label className="anim-field-label">
              <span>At Seconds</span>
              <input
                type="number"
                min={0}
                step={0.5}
                value={placeTime}
                onChange={(e) => onChangePlaceTime(Number(e.target.value))}
              />
            </label>
          </div>

          <button
            id="anim-place-clip-btn"
            type="button"
            className="ghost anim-full-btn"
            disabled={busy || !lastRenderedClip}
            onClick={onPlaceOnTimeline}
          >
            Timeline
          </button>

          <div className="seq-side-block">
            <label className="seq-field">
              <span>Pose frame (s)</span>
              <input
                id="authoring-reference"
                type="text"
                value={poseRef}
                onChange={(e) => setPoseRef(e.target.value)}
              />
            </label>
            <button
              id="authoring-pose-export"
              type="button"
              className="ghost anim-full-btn"
              disabled={busy}
              onClick={async () => {
                try {
                  const res = await executeCommand<{ frame?: string }>("character.frame", {
                    character: selectedCharId,
                    view: selectedView,
                    time: currentTime,
                  });
                  onToast(`Rendered pose frame at ${currentTime.toFixed(2)}s: ${res.frame?.split("/").pop() || poseRef}`);
                } catch (err) {
                  onToast(String(err));
                }
              }}
            >
              Frame
            </button>
          </div>
        </div>
      </details>

      <div id="motion-source-context" className="motion-source-context" />
    </aside>
  );
};
