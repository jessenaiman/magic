import React from "react";
import {
  AnimationStyle,
  Character3DClip,
  CharacterPart,
  CharacterSpriteSummary,
  CharacterViewSummary,
  executeCommand,
  fileUrl,
} from "../../api";
import { Slider, Toggle } from "../../components/controls";

export interface AnimationActionPanelProps {
  selectedCharId: string;
  selectedView: string;
  onSelectView: (v: string) => void;
  animationStyle: AnimationStyle;
  parts: CharacterPart[];
  drawnViews: CharacterViewSummary[];
  hasMultipleDrawnViews: boolean;
  sprites: CharacterSpriteSummary[];
  sprite2DCycles: CharacterSpriteSummary[];
  rig25DCycles: CharacterSpriteSummary[];
  clips3D: Character3DClip[];
  modelClipNames: string[];
  selectedSprite: CharacterSpriteSummary | null;
  onSelectSprite: (s: CharacterSpriteSummary | null) => void;
  activeActionName: string;
  onTriggerAction: (actionName: string) => void;
  actionsList: Array<{ id: string; btnId: string; label: string }>;
  actionLabel: string;
  setActionLabel: React.Dispatch<React.SetStateAction<string>>;
  actionDuration: number;
  setActionDuration: React.Dispatch<React.SetStateAction<number>>;
  isLooping: boolean;
  onToggleLoop: (val: boolean) => void;
  placeTrack: string;
  onChangePlaceTrack: (v: string) => void;
  placeTime: number;
  onChangePlaceTime: (v: number) => void;
  placeRate: number;
  setPlaceRate: React.Dispatch<React.SetStateAction<number>>;
  placeX: number;
  setPlaceX: React.Dispatch<React.SetStateAction<number>>;
  placeY: number;
  setPlaceY: React.Dispatch<React.SetStateAction<number>>;
  placeW: number;
  setPlaceW: React.Dispatch<React.SetStateAction<number>>;
  placeH: number;
  setPlaceH: React.Dispatch<React.SetStateAction<number>>;
  lastRenderedClip: string;
  onPlaceOnTimeline: () => void;
  runAssembleEffectRig: (openGui: boolean, is3D: boolean) => Promise<void>;
  busy: boolean;
}

export const AnimationActionPanel: React.FC<AnimationActionPanelProps> = ({
  selectedCharId,
  selectedView,
  onSelectView,
  animationStyle,
  parts,
  drawnViews,
  hasMultipleDrawnViews,
  sprites,
  sprite2DCycles,
  rig25DCycles,
  clips3D,
  modelClipNames,
  selectedSprite,
  onSelectSprite,
  activeActionName,
  onTriggerAction,
  actionsList,
  actionLabel,
  setActionLabel,
  actionDuration,
  setActionDuration,
  isLooping,
  onToggleLoop,
  placeTrack,
  onChangePlaceTrack,
  placeTime,
  onChangePlaceTime,
  placeRate,
  setPlaceRate,
  placeX,
  setPlaceX,
  placeY,
  setPlaceY,
  placeW,
  setPlaceW,
  placeH,
  setPlaceH,
  lastRenderedClip,
  onPlaceOnTimeline,
  runAssembleEffectRig,
  busy,
}) => (
  <>
      {/* 1. ACTION (closed drawer by default per A3) */}
      <details id="motion-action" className="panel motion-section" data-look="motion">
        <summary className="panel-head" data-look="section">
          <span className="ico" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="currentColor">
              <path d="M8 5v14l11-7z" />
            </svg>
          </span>
          <span className="title">Action</span>
          <span id="motion-action-meta" className="meta">
            {selectedSprite?.name || activeActionName || "Rest"}
          </span>
          <span className="chev" aria-hidden="true">
            ▸
          </span>
        </summary>
        <div className="motion-section-body">
          <div className="motion-actions-grid" role="group" aria-label="Character actions">
            {actionsList.map((act) => {
              const spriteMatch = sprites.find((s) => s.name === act.id);
              const isPressed =
                act.id === "rest"
                  ? !selectedSprite && activeActionName === "rest"
                  : selectedSprite?.name === act.id || activeActionName === act.id;
              return (
                <button
                  key={act.btnId}
                  id={act.btnId}
                  type="button"
                  className="motion-action"
                  aria-pressed={isPressed}
                  onClick={() => {
                    if (act.id === "rest") {
                      onSelectSprite(null);
                      onTriggerAction("rest");
                    } else if (spriteMatch) {
                      onSelectSprite(spriteMatch);
                    } else {
                      onSelectSprite(null);
                      onTriggerAction(act.id);
                    }
                  }}
                >
                  <span>{act.label}</span>
                </button>
              );
            })}
            {sprites
              .filter((s) => !actionsList.some((a) => a.id === s.name))
              .map((s) => {
                const isPressed = selectedSprite?.name === s.name;
                return (
                  <button
                    key={`sprite-${s.name}`}
                    id={`anim-sprite-${s.name}`}
                    type="button"
                    className="motion-action"
                    aria-pressed={isPressed}
                    onClick={() => onSelectSprite(s)}
                  >
                    <span>{s.name}</span>
                  </button>
                );
              })}
          </div>

          <div id="char-magic-panel" className="char-magic-panel">
            {animationStyle === "rig_3d" ? (
              <>
                <div className="char-magic-head">
                  <strong className="char-subhead">3D Model Animations</strong>
                  <span id="char-magic-badge" className="char-magic-badge">
                    {clips3D.length || modelClipNames.length} clips
                  </span>
                </div>
                <p className="motion-help" style={{ margin: "4px 0 8px" }}>
                  3D models play skeletal animation clips (.glb / .blend) instead of 2D sprite sheets.
                </p>
                <div className="char-sheets-row">
                  {clips3D.map((clip) => {
                    const active = activeActionName === clip.id;
                    return (
                      <button
                        key={clip.id}
                        id={`anim-3d-clip-${clip.id}`}
                        type="button"
                        className={`char-sheet-card ${active ? "is-active" : ""}`}
                        onClick={() => {
                          onSelectSprite(null);
                          onTriggerAction(clip.id);
                        }}
                      >
                        <span>{clip.name || clip.id}</span>
                      </button>
                    );
                  })}
                  {clips3D.length === 0 &&
                    modelClipNames.map((name) => {
                      const active = activeActionName === name;
                      return (
                        <button
                          key={name}
                          type="button"
                          className={`char-sheet-card ${active ? "is-active" : ""}`}
                          onClick={() => {
                            onSelectSprite(null);
                            onTriggerAction(name);
                          }}
                        >
                          <span>{name}</span>
                        </button>
                      );
                    })}
                </div>
                <div className="motion-path-actions" style={{ marginTop: "8px" }}>
                  <button
                    id="btn-assemble-3d-effectcraft"
                    type="button"
                    className="ghost"
                    disabled={busy}
                    onClick={() => void runAssembleEffectRig(false, true)}
                  >
                    Stage
                  </button>
                  <button
                    id="btn-open-3d-effectcraft"
                    type="button"
                    className="start"
                    disabled={busy}
                    onClick={() => void runAssembleEffectRig(true, true)}
                  >
                    EffectCraft
                  </button>
                </div>
              </>
            ) : animationStyle === "sprite_2d" ? (
              <>
                <div className="char-magic-head">
                  <strong className="char-subhead">2D Directional Sprite Sheets</strong>
                  <span id="char-magic-badge" className="char-magic-badge">
                    {sprite2DCycles.length || sprites.length} sheets
                  </span>
                </div>
                <p className="motion-help" style={{ margin: "4px 0 8px" }}>
                  2D sprite characters play directional frame sheets (no 3D turnaround views).
                </p>
                {(sprite2DCycles.length > 0 ? sprite2DCycles : sprites).length > 0 && (
                  <div className="char-sheets-row">
                    {(sprite2DCycles.length > 0 ? sprite2DCycles : sprites).map((sp) => {
                      const thumb = sp.frames?.[0] || sp.preview;
                      const active = selectedSprite?.name === sp.name;
                      return (
                        <button
                          key={sp.name}
                          type="button"
                          className={`char-sheet-card ${active ? "is-active" : ""}`}
                          onClick={() => onSelectSprite(sp)}
                        >
                          {thumb ? <img src={fileUrl(thumb)} alt={sp.name} loading="lazy" /> : null}
                          <span>{sp.name}</span>
                        </button>
                      );
                    })}
                  </div>
                )}
              </>
            ) : (
              <>
                <div className="char-magic-head">
                  <strong className="char-subhead">2.5D Layered Rig &amp; Cycles</strong>
                  <span id="char-magic-badge" className="char-magic-badge">
                    {parts.length} layers · {sprites.length} cycles
                  </span>
                </div>
                {hasMultipleDrawnViews && (
                  <div id="char-angle-grid" className="char-angle-grid">
                    {drawnViews.map((vObj) => {
                      const v = vObj.view;
                      const thumb = vObj.drawing;
                      return (
                        <button
                          key={v}
                          type="button"
                          className={`char-thumb-card ${selectedView === v ? "is-active" : ""}`}
                          onClick={() => onSelectView(v)}
                        >
                          {thumb ? <img src={fileUrl(thumb)} alt={v} loading="lazy" /> : null}
                          <span>{v}</span>
                        </button>
                      );
                    })}
                  </div>
                )}
                {(rig25DCycles.length > 0 ? rig25DCycles : sprites).length > 0 && (
                  <div className="char-sheets-row">
                    {sprites.map((sp) => {
                      const thumb = sp.frames?.[0] || sp.preview;
                      const active = selectedSprite?.name === sp.name;
                      return (
                        <button
                          key={sp.name}
                          type="button"
                          className={`char-sheet-card ${active ? "is-active" : ""}`}
                          onClick={() => onSelectSprite(sp)}
                        >
                          {thumb ? <img src={fileUrl(thumb)} alt={sp.name} loading="lazy" /> : null}
                          <span>{sp.name}</span>
                        </button>
                      );
                    })}
                  </div>
                )}
                <div className="motion-path-actions" style={{ marginTop: "8px" }}>
                  <button
                    id="btn-assemble-2-5d-effectcraft"
                    type="button"
                    className="ghost"
                    disabled={busy}
                    onClick={() => void runAssembleEffectRig(false, false)}
                  >
                    Rebuild
                  </button>
                  <button
                    id="btn-open-2-5d-effectcraft"
                    type="button"
                    className="ghost"
                    disabled={busy}
                    onClick={() => void runAssembleEffectRig(true, false)}
                  >
                    EffectCraft
                  </button>
                </div>
              </>
            )}
          </div>

          <button
            id="btn-generate-missing-poses"
            type="button"
            className="ghost"
            onClick={() =>
              void executeCommand("character.open", {
                character: selectedCharId,
                view: selectedView,
                app: "effectcraft",
              })
            }
          >
            EffectCraft
          </button>

          <div className="seq-side-block">
            <label className="seq-field">
              <span>Action name</span>
              <input
                id="authoring-label"
                type="text"
                value={actionLabel}
                onChange={(e) => setActionLabel(e.target.value)}
              />
            </label>
            <Slider
              id="authoring-duration"
              label="Cycle duration"
              value={actionDuration}
              min={0.25}
              max={10}
              step={0.25}
              unit="s"
              onChange={setActionDuration}
            />
            <Toggle
              id="authoring-loop"
              label="Loop action cycle"
              checked={isLooping}
              onChange={onToggleLoop}
            />
            <button
              id="authoring-action-apply"
              type="button"
              className="ghost"
              disabled={busy}
              onClick={() => onTriggerAction(actionLabel || "idle")}
            >
              Save
            </button>
          </div>

          <details className="seq-side-block">
            <summary className="muted" style={{ cursor: "pointer", fontSize: "var(--text-xs)" }}>
              Placement
            </summary>
            <div className="motion-section-body" style={{ padding: "var(--space-2) 0 0" }}>
              <label className="seq-field">
                <span>Track</span>
                <select
                  id="authoring-place-scene"
                  value={placeTrack}
                  onChange={(e) => onChangePlaceTrack(e.target.value)}
                >
                  <option value="V2">V2 (Characters)</option>
                  <option value="V1">V1 (Background)</option>
                  <option value="V3">V3 (Overlays)</option>
                </select>
              </label>
              <div className="motion-coordinate-row">
                <label className="anim-field-label">
                  <span>Start (s)</span>
                  <input
                    id="authoring-place-start"
                    type="number"
                    step={0.5}
                    value={placeTime}
                    onChange={(e) => onChangePlaceTime(Number(e.target.value))}
                  />
                </label>
                <label className="anim-field-label">
                  <span>Duration (s)</span>
                  <input
                    id="authoring-place-duration"
                    type="number"
                    step={0.5}
                    value={actionDuration}
                    onChange={(e) => setActionDuration(Number(e.target.value))}
                  />
                </label>
              </div>
              <Slider
                id="authoring-place-rate"
                label="Clip rate"
                control="dial"
                value={placeRate}
                min={0.25}
                max={2.0}
                step={0.05}
                unit="×"
                onChange={setPlaceRate}
              />
              <div className="motion-coordinate-row">
                <label className="anim-field-label">
                  <span>X</span>
                  <input
                    id="authoring-place-x"
                    type="number"
                    value={placeX}
                    onChange={(e) => setPlaceX(Number(e.target.value))}
                  />
                </label>
                <label className="anim-field-label">
                  <span>Y</span>
                  <input
                    id="authoring-place-y"
                    type="number"
                    value={placeY}
                    onChange={(e) => setPlaceY(Number(e.target.value))}
                  />
                </label>
                <label className="anim-field-label">
                  <span>W</span>
                  <input
                    id="authoring-place-w"
                    type="number"
                    value={placeW}
                    onChange={(e) => setPlaceW(Number(e.target.value))}
                  />
                </label>
                <label className="anim-field-label">
                  <span>H</span>
                  <input
                    id="authoring-place-h"
                    type="number"
                    value={placeH}
                    onChange={(e) => setPlaceH(Number(e.target.value))}
                  />
                </label>
              </div>
              <button
                id="authoring-place-video"
                type="button"
                className="start"
                disabled={busy}
                onClick={onPlaceOnTimeline}
              >
                Place
              </button>
              <span id="authoring-place-status" className="muted" role="status">
                {lastRenderedClip ? `Ready: ${lastRenderedClip.split("/").pop()}` : "Render a clip first"}
              </span>
            </div>
          </details>
        </div>
      </details>

  </>
);
