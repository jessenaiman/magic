import React, { useMemo, useState } from "react";
import type { ViewMode } from "../App";
import {
  CharacterSpriteSummary,
  CharacterSummary,
  CharacterViewSummary,
  executeCommand,
  fileUrl,
  pickFile,
  ProjectAsset,
  WorkshopData,
} from "../api";
import { Button, ControlGroup, Segmented, Toolbar } from "./controls";

interface StageAssetsProps {
  projectPath: string;
  view: ViewMode;
  data: WorkshopData | null;
  open: boolean;
  onToggleOpen: () => void;
  selectedCharacterId?: string;
  onSelectCharacter: (charId: string, navigateToAnimation?: boolean) => void;
  onNavigate: (view: ViewMode, charId?: string) => void;
  onRefresh: () => Promise<void>;
  onToast: (message: string) => void;
}

interface UnifiedCard {
  id: string;
  charId?: string;
  assetId?: string;
  name: string;
  badge: string;
  thumb: string | null;
  role: "character" | "background";
  path?: string;
  sourceProject?: string | null;
}

function getCharacterThumb(c: CharacterSummary): string | null {
  if (c.image) return c.image;
  if (Array.isArray(c.views)) {
    for (const v of c.views) {
      if (typeof v === "object" && v !== null && "drawing" in v && (v as CharacterViewSummary).drawing) {
        return (v as CharacterViewSummary).drawing || null;
      }
    }
  }
  if (Array.isArray(c.sprites)) {
    for (const s of c.sprites) {
      if (typeof s === "object" && s !== null) {
        const sp = s as CharacterSpriteSummary;
        if (sp.frames && sp.frames.length > 0) return sp.frames[0];
        if (sp.preview) return sp.preview;
      }
    }
  }
  return null;
}

function getCharacterProject(c: CharacterSummary): string | null {
  if (Array.isArray(c.views)) {
    for (const v of c.views) {
      if (typeof v === "object" && v !== null && (v as CharacterViewSummary).ecproj) {
        return (v as CharacterViewSummary).ecproj || null;
      }
    }
  }
  return null;
}

/**
 * Top horizontal Asset Drawer built on the Tauri Base Control System (`Toolbar`, `Button`, `Segmented`)
 * so every toggle, filter tab, and action button aligns horizontally and vertically with single-word labels.
 */
export const StageAssets: React.FC<StageAssetsProps> = ({
  projectPath,
  view,
  data,
  open,
  onToggleOpen,
  selectedCharacterId,
  onSelectCharacter,
  onNavigate,
  onRefresh,
  onToast,
}) => {
  const [tab, setTab] = useState<"characters" | "backgrounds">("characters");

  const rawAssets: ProjectAsset[] = data?.assets?.assets ?? [];
  const rawCharacters: CharacterSummary[] = data?.characters ?? [];

  const { characterCards, backgroundCards } = useMemo(() => {
    const chars: UnifiedCard[] = [];
    const seenAssetIds = new Set<string>();

    for (const c of rawCharacters) {
      const cid = c.id;
      const matchingAsset = rawAssets.find(
        (a) => a.id === cid || a.id.replace(/[^a-zA-Z0-9]+/g, "_") === cid || a.character === cid,
      );
      if (matchingAsset) {
        seenAssetIds.add(matchingAsset.id);
      }
      const thumb =
        getCharacterThumb(c) ||
        (matchingAsset?.path && /\.(png|jpe?g|webp|gif|svg)$/i.test(matchingAsset.path) ? matchingAsset.path : null);
      chars.push({
        id: matchingAsset?.id || cid,
        charId: cid,
        assetId: matchingAsset?.id,
        name: c.name || cid,
        badge: c.animation_style === "cutout_2_5d" ? "Rig" : c.animation_style === "sprite_2d" ? "Sprite" : "Cutout",
        thumb,
        role: "character",
        path: thumb || matchingAsset?.path,
        sourceProject: getCharacterProject(c),
      });
    }

    const bgs: UnifiedCard[] = [];
    for (const asset of rawAssets) {
      const isImg = Boolean(asset.path && /\.(png|jpe?g|webp|gif|svg)$/i.test(asset.path));
      if (asset.role === "background") {
        bgs.push({
          id: asset.id,
          assetId: asset.id,
          name: asset.name || asset.id,
          badge: "Plate",
          thumb: isImg ? asset.path! : null,
          role: "background",
          path: asset.path,
          sourceProject: null,
        });
      } else if (!seenAssetIds.has(asset.id)) {
        const normId = (asset.character || asset.id).replace(/[^a-zA-Z0-9]+/g, "_");
        seenAssetIds.add(asset.id);
        chars.push({
          id: asset.id,
          charId: normId,
          assetId: asset.id,
          name: asset.name || asset.id,
          badge: asset.kind === "model_3d" ? "3D" : "Image",
          thumb: isImg ? asset.path! : null,
          role: "character",
          path: asset.path,
          sourceProject: null,
        });
      }
    }

    return { characterCards: chars, backgroundCards: bgs };
  }, [rawAssets, rawCharacters]);

  const totalCount = characterCards.length + backgroundCards.length;
  const visible = tab === "backgrounds" ? backgroundCards : characterCards;
  const activeCharId = selectedCharacterId || characterCards[0]?.charId || "";

  const handleAddAsset = async () => {
    try {
      const picked = await pickFile({
        title: "Connect artwork, plate, or 3D model",
        extensions: ["png", "jpg", "jpeg", "webp", "svg", "glb", "gltf", "pcraft", "ecproj"],
      });
      if (typeof picked !== "string" || !picked) return;
      await executeCommand("project.add_asset", {
        path: picked,
        role: tab === "backgrounds" ? "background" : "character",
        _expected_project: projectPath,
      });
      await onRefresh();
      onToast(`Added ${picked.split("/").pop()}`);
    } catch (err) {
      onToast(String(err));
    }
  };

  return (
    <section id="stage-assets" className="stage-assets-bar" aria-label="Project Assets Drawer">
      <Toolbar dense align="between" className="stage-assets-header">
        <ControlGroup>
          <Button
            id="stage-assets-toggle"
            size="sm"
            variant="ghost"
            icon={open ? "chevronDown" : "chevronRight"}
            label="Assets"
            badge={totalCount}
            active={open}
            aria-expanded={open}
            title="Toggle project assets drawer"
            onClick={onToggleOpen}
          />

          {open && (
            <Segmented
              size="sm"
              ariaLabel="Asset category"
              value={tab}
              onChange={setTab}
              options={[
                {
                  id: "stage-assets-tab-characters",
                  value: "characters",
                  label: "Characters",
                  icon: "user",
                  badge: characterCards.length,
                  title: "Characters and puppet rigs",
                },
                {
                  id: "stage-assets-tab-backgrounds",
                  value: "backgrounds",
                  label: "Plates",
                  icon: "image",
                  badge: backgroundCards.length,
                  title: "Scene background plates",
                },
              ]}
            />
          )}
        </ControlGroup>

        {open && (
          <ControlGroup>
            <Button
              id="stage-assets-prepare"
              size="sm"
              variant="ghost"
              icon="scissors"
              label="Prepare"
              title="Open Animation view to isolate subject in PhotoCraft and rig puppet layers"
              onClick={() => onNavigate("animation", activeCharId || undefined)}
            />
            <Button
              id="stage-assets-add"
              size="sm"
              variant="primary"
              icon="plus"
              label="Add"
              title="Add character artwork or background plate to project"
              onClick={() => void handleAddAsset()}
            />
          </ControlGroup>
        )}
      </Toolbar>

      {open && (
        <div className="stage-assets-strip">
          {visible.length === 0 ? (
            <span className="muted">No {tab} connected yet.</span>
          ) : (
            visible.map((card) => {
              const selected = card.charId ? card.charId === activeCharId : false;
              return (
                <div
                  key={card.id}
                  id={`asset-card-${card.id}`}
                  className={`stage-asset-card ${selected ? "selected" : ""}`}
                  draggable={Boolean(card.path)}
                  onDragStart={(e) => {
                    if (!card.path) return;
                    const payload = JSON.stringify({
                      path: card.path,
                      source_project: card.sourceProject || undefined,
                      name: card.name,
                    });
                    e.dataTransfer.setData("application/x-magic-asset", payload);
                    e.dataTransfer.setData("text/plain", card.path);
                    e.dataTransfer.effectAllowed = "copy";
                  }}
                  onClick={() => {
                    if (card.charId) {
                      onSelectCharacter(card.charId, view === "animation");
                    }
                  }}
                  title={card.path ? `Drag onto a Storyboard slot or click to select · ${card.name}` : card.name}
                >
                  {card.thumb ? (
                    <img className="stage-asset-thumb" src={fileUrl(card.thumb)} alt={card.name} />
                  ) : (
                    <div className="stage-asset-thumb" aria-hidden="true" />
                  )}
                  <div style={{ minWidth: 0 }}>
                    <div style={{ fontSize: 12, fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {card.name}
                    </div>
                    <div className="muted" style={{ fontSize: 11 }}>
                      {card.badge}
                    </div>
                  </div>
                </div>
              );
            })
          )}
        </div>
      )}
    </section>
  );
};
