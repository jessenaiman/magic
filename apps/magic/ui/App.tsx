import { AnimationView } from "./views/AnimationView";
import { FidgetPanel } from "./components/FidgetPanel";
import { LyricsView } from "./views/LyricsView";
import { MotionView } from "./views/MotionView";
import { AppFooter } from "./components/AppFooter";
import { StageAssets } from "./components/StageAssets";
import { Tooltip } from "./components/Tooltip";
import { Icon } from "./components/controls";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { importDroppedArtwork } from "./artwork-transfer";
import { connectDesktop } from "./desktop";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { executeCommand, toggleWindowSnap, WorkshopData } from "./api";
import { FilmView } from "./views/FilmView";
import { GalleryView } from "./views/GalleryView";
import { HomeView } from "./views/HomeView";
import { ProjectView } from "./views/ProjectView";
import { PreservedStoryboard } from "./views/StoryboardView";

export type ViewMode =
  | "home"
  | "project"
  | "storyboard"
  | "design"
  | "motion"
  | "gallery"
  | "animation"
  | "studio";

const formatClock = (sec: number): string => {
  if (!Number.isFinite(sec) || sec < 0) return "0:00";
  const s = Math.floor(sec);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};

export const App: React.FC = () => {
  const [view, setView] = useState<ViewMode>("home");
  const [projectPath, setProjectPath] = useState<string | null>(null);
  const [selectedCharacterId, setSelectedCharacterId] = useState<string | undefined>(undefined);
  const [assetsOpen, setAssetsOpen] = useState<boolean>(false);
  const [data, setData] = useState<WorkshopData | null>(null);
  const [toastMsg, setToastMsg] = useState<string>("");
  const [toastVisible, setToastVisible] = useState<boolean>(false);
  const [busy, setBusy] = useState<boolean>(false);
  const [fidgetOpen, setFidgetOpen] = useState<boolean>(false);
  const [audioPlaying, setAudioPlaying] = useState<boolean>(false);
  const [audioTime, setAudioTime] = useState<number>(0);
  const [audioDuration, setAudioDuration] = useState<number>(0);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  const showToast = useCallback((msg: string) => {
    setToastMsg(msg);
    setToastVisible(true);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToastVisible(false), 6000);
  }, []);
  useEffect(() => () => window.clearTimeout(toastTimer.current), []);

  const liveState = useRef({ view, projectPath, busy, toastMsg });
  liveState.current = { view, projectPath, busy, toastMsg };
  const refreshVersion = useRef(0);
  const initialized = useRef(false);
  const transferBusy = useRef(false);

  const refreshProject = async (initial = false) => {
    const version = ++refreshVersion.current;
    const snapshot = await invoke<{ project: string | null; data: WorkshopData | null; initialView: ViewMode }>("session_snapshot");
    if (version !== refreshVersion.current) return;
    const changed = snapshot.project !== liveState.current.projectPath;
    setProjectPath(snapshot.project);
    setData(snapshot.data);
    if (initial && !initialized.current) setView(snapshot.initialView);
    else if (changed) setView(snapshot.project ? "project" : "home");
    initialized.current = true;
  };

  const navigateTo = useCallback((nextView: ViewMode, characterId?: string) => {
    if (characterId) {
      setSelectedCharacterId(characterId);
    }
    setView(nextView);
  }, []);

  const openProject = async (path: string, initialView: ViewMode = "project", characterId?: string) => {
    setBusy(true);
    try {
      setSelectedCharacterId(characterId);
      await executeCommand("file.open", { path });
      await refreshProject();
      setView(initialView);
      showToast(`Opened ${path.split("/").pop()}`);
    } catch (err) {
      showToast(`Could not open ${path}: ${err}`);
    } finally {
      setBusy(false);
    }
  };

  const closeProject = async () => {
    try {
      audioRef.current?.pause();
      await executeCommand("file.close");
      await refreshProject();
      setView("home");
    } catch (error) {
      showToast(`Could not close the project: ${error}`);
    }
  };

  const launchCraftApp = async (app: string, label: string) => {
    try {
      await executeCommand("workspace.open", { app });
      showToast(`Launched ${label}`);
    } catch (error) {
      showToast(`Could not launch ${label}: ${error}`);
    }
  };

  const openScribbles = async () => {
    try {
      const res = await executeCommand<{
        ingested?: { character_id?: string; registered_count?: number };
      }>("scribbles.open", {
        character: selectedCharacterId,
      });
      const count = res?.ingested?.registered_count ?? 0;
      if (count > 0) {
        await refreshProject();
        showToast(`Opened Scribbles · ingested ${count} asset${count === 1 ? "" : "s"}`);
      } else {
        showToast("Opened Scribbles");
      }
    } catch (error) {
      showToast(`Could not open Scribbles: ${error}`);
    }
  };

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type !== "drop" || disposed) return;
        const project = liveState.current.projectPath;
        if (!project) {
          showToast("Open a project before dropping artwork");
          return;
        }
        if (transferBusy.current || liveState.current.busy) {
          showToast("Wait for the current operation before dropping artwork");
          return;
        }
        transferBusy.current = true;
        setBusy(true);
        void importDroppedArtwork(event.payload.paths, project, executeCommand)
          .then(async (count) => {
            await refreshProject();
            if (disposed || liveState.current.projectPath !== project) return;
            showToast(`Imported ${count} artwork file${count === 1 ? "" : "s"}`);
          })
          .catch((error) => {
            if (!disposed) showToast(`Artwork import failed: ${error}`);
          })
          .finally(() => {
            transferBusy.current = false;
            if (!disposed) setBusy(false);
          });
      })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch((error) => showToast(`File drop unavailable: ${error}`));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    let disconnect: (() => void) | undefined;
    void (async () => {
      disconnect = await connectDesktop(
        () => {
          const state = liveState.current;
          return {
            ok: true,
            renderer: "tauri",
            ready: true,
            view: state.projectPath || state.view === "gallery" ? state.view : "home",
            requestedView: state.view,
            project: state.projectPath,
            busy: state.busy,
            status: state.toastMsg,
          };
        },
        (next) => setView(next as ViewMode),
        () => refreshProject(),
      );
      if (disposed) {
        disconnect();
        return;
      }
      await refreshProject(true);
      if (!disposed) await invoke("desktop_ready");
    })().catch((error) => showToast(`Desktop connection failed: ${error}`));
    return () => {
      disposed = true;
      ++refreshVersion.current;
      disconnect?.();
    };
  }, [connectDesktop]);

  useEffect(() => {
    if (projectPath && initialized.current) {
      void refreshProject().catch((error) => showToast(`Project refresh failed: ${error}`));
    }
  }, [view]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((!event.ctrlKey && !event.metaKey) || ["Control", "Meta", "Shift", "Alt"].includes(event.key)) return;
      const key = event.key.length === 1 ? event.key.toUpperCase() : event.key;
      const chord = `Cmd+${event.altKey ? "Alt+" : ""}${event.shiftKey ? "Shift+" : ""}${key}`;
      const typing = /^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement?.tagName ?? "");
      void invoke("key_pressed", { chord, typing }).catch((error) => showToast(String(error)));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [showToast]);

  const projectName =
    data?.name ||
    projectPath?.split("/").pop()?.replace(/\.magic$/, "") ||
    "Home";

  const songAudioPath = data?.status?.stages?.find((s) => s.id === "song")?.inputs?.[0]?.path || null;

  useEffect(() => {
    setAudioPlaying(false);
    setAudioTime(0);
    setAudioDuration(0);
  }, [songAudioPath]);

  const toggleTopAudio = () => {
    const el = audioRef.current;
    if (!el || !songAudioPath) return;
    if (el.paused) {
      void el.play().catch((err) => showToast(`Audio preview failed: ${err}`));
    } else {
      el.pause();
    }
  };

  return (
    <div id="app-shell" className="app-shell">
      <header id="app-bar" data-tauri-drag-region="deep" className="app-bar">
        <div className="app-identity app-identity-row">
          <Tooltip label={projectPath ? "Return to Home" : "Magic"}>
            <button id="brand" type="button" className="top-brand-btn" aria-label="Magic" onClick={() => void closeProject()}>
              <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" className="app-logo">
                <path d="M12 2l2.2 6.3L20.5 10.5l-6.3 2.2L12 19l-2.2-6.3L3.5 10.5l6.3-2.2L12 2zm6.5 12l1 2.8 2.8 1-2.8 1-1 2.8-1-2.8-2.8-1 2.8-1 1-2.8z" />
              </svg>
              {!projectPath && <span className="app-title">Magic</span>}
            </button>
          </Tooltip>

          {projectPath ? (
            <button
              id="tab-project"
              type="button"
              className={`top-project-pill ${view === "project" ? "active" : ""}`}
              onClick={() => setView("project")}
              title={`Project · ${projectName}`}
            >
              <Icon name="book" />
              <span id="project-name">{projectName}</span>
            </button>
          ) : (
            <span id="project-name" className="muted app-project-name">
              {projectName}
            </span>
          )}

          <Tooltip label="Snap window">
            <button
              id="window-snap"
              type="button"
              aria-label="Snap"
              className="top-icon-ghost"
              onClick={() =>
                void toggleWindowSnap()
                  .then((to) => showToast(`Window moved to ${to}`))
                  .catch((e) => showToast(`Window snap failed: ${e}`))
              }
            >
              <Icon name="external" />
            </button>
          </Tooltip>
        </div>

        <nav id="mode-tabs" aria-label="Workspace views" className="top-mode-nav">
          {(
            [
              ["storyboard", "Storyboard"],
              ["design", "Lyrics"],
              ["motion", "Motion"],
              ["animation", "Animation"],
              ["studio", "Studio"],
              ["gallery", "Gallery"],
            ] as Array<[ViewMode, string]>
          )
            .filter(([id]) => projectPath || id === "gallery")
            .map(([id, label]) => (
              <button
                key={id}
                id={`tab-${id}`}
                type="button"
                className={`top-mode-tab ${view === id ? "active" : ""}`}
                onClick={() => setView(id)}
              >
                {id === "storyboard" && (
                  <Tooltip label={data?.storyboard?.error || `${data?.storyboard?.count ?? 0} unfilled fields`}>
                    <span
                      aria-label={data?.storyboard?.complete ? "Storyboard complete" : "Storyboard incomplete"}
                      className={`app-storyboard-dot ${
                        data?.storyboard?.complete
                          ? "complete"
                          : data?.storyboard?.traffic_light === "amber" || data?.storyboard?.status === "review"
                            ? "review"
                            : "incomplete"
                      }`}
                    >
                      ●
                    </span>
                  </Tooltip>
                )}
                <span className="ui-btn-label">{label}</span>
              </button>
            ))}
        </nav>

        <div id="app-bar-tools" className="top-right-tools">
          {songAudioPath && (
            <>
              <audio
                ref={audioRef}
                src={convertFileSrc(songAudioPath)}
                preload="metadata"
                onPlay={() => setAudioPlaying(true)}
                onPause={() => setAudioPlaying(false)}
                onEnded={() => setAudioPlaying(false)}
                onTimeUpdate={(e) => setAudioTime(e.currentTarget.currentTime)}
                onLoadedMetadata={(e) => setAudioDuration(e.currentTarget.duration || 0)}
              />
              <Tooltip label={`${audioPlaying ? "Pause" : "Play"} song (${formatClock(audioTime)} / ${formatClock(audioDuration)})`}>
                <button
                  id="app-audio-toggle"
                  type="button"
                  aria-label={audioPlaying ? "Pause" : "Play"}
                  className={`top-audio-pill ${audioPlaying ? "playing" : ""}`}
                  onClick={toggleTopAudio}
                >
                  <Icon name={audioPlaying ? "pause" : "play"} />
                  <span className="top-audio-clock">{formatClock(audioTime)}</span>
                </button>
              </Tooltip>
            </>
          )}

          <div className="top-craft-dock" aria-label="ArtCraft Suite">
            <Tooltip label="PhotoCraft">
              <button
                id="dock-photocraft"
                type="button"
                aria-label="PhotoCraft"
                className="craft-dock-btn craft-photo"
                onClick={() => void launchCraftApp("photocraft", "PhotoCraft")}
              >
                <Icon name="image" />
              </button>
            </Tooltip>
            <Tooltip label="VectorCraft">
              <button
                id="dock-vectorcraft"
                type="button"
                aria-label="VectorCraft"
                className="craft-dock-btn craft-vector"
                onClick={() => void launchCraftApp("vectorcraft", "VectorCraft")}
              >
                <Icon name="pen" />
              </button>
            </Tooltip>
            <Tooltip label="DesignCraft">
              <button
                id="dock-designcraft"
                type="button"
                aria-label="DesignCraft"
                className="craft-dock-btn craft-design"
                onClick={() => void launchCraftApp("designcraft", "DesignCraft")}
              >
                <Icon name="book" />
              </button>
            </Tooltip>
            <Tooltip label="EffectCraft">
              <button
                id="dock-effectcraft"
                type="button"
                aria-label="EffectCraft"
                className="craft-dock-btn craft-effect"
                onClick={() => void launchCraftApp("effectcraft", "EffectCraft")}
              >
                <Icon name="sparkles" />
              </button>
            </Tooltip>
            <Tooltip label="FilmCraft">
              <button
                id="dock-filmcraft"
                type="button"
                aria-label="FilmCraft"
                className="craft-dock-btn craft-film"
                onClick={() => void launchCraftApp("filmcraft", "FilmCraft")}
              >
                <Icon name="film" />
              </button>
            </Tooltip>
            <Tooltip label="Fidget">
              <button
                id="fidget-toggle"
                type="button"
                className="craft-dock-btn craft-fidget"
                onClick={() => setFidgetOpen((v) => !v)}
                aria-label="Fidget"
                aria-expanded={fidgetOpen}
              >
                <Icon name="wand" />
              </button>
            </Tooltip>
            <Tooltip label="Scribbles">
              <button
                id="scribbles-toggle"
                type="button"
                className="craft-dock-btn craft-scribbles"
                onClick={() => void openScribbles()}
                aria-label="Scribbles"
              >
                <img src="/scribbles-cat-work.svg" alt="Scribbles" width={20} height={20} className="craft-scribbles-icon" />
              </button>
            </Tooltip>
            <FidgetPanel open={fidgetOpen} onClose={() => setFidgetOpen(false)} onToast={showToast} />
          </div>
        </div>
      </header>

      {projectPath && view !== "home" && view !== "gallery" && (
        <StageAssets
          projectPath={projectPath}
          view={view}
          data={data}
          open={assetsOpen}
          onToggleOpen={() => setAssetsOpen((v) => !v)}
          selectedCharacterId={selectedCharacterId}
          onSelectCharacter={(charId, goAnim) => {
            setSelectedCharacterId(charId);
            if (goAnim && view !== "animation") {
              setView("animation");
            }
          }}
          onNavigate={navigateTo}
          onRefresh={() => refreshProject()}
          onToast={showToast}
        />
      )}

      <div className="app-view-host">
        {view === "gallery" ? (
          <GalleryView />
        ) : view === "home" || !projectPath ? (
          <HomeView onOpenProject={openProject} onToast={showToast} />
        ) : view === "studio" ? (
          <FilmView projectPath={projectPath} onToast={showToast} />
        ) : view === "animation" ? (
          <AnimationView
            projectPath={projectPath}
            initialCharacterId={selectedCharacterId}
            characters={data?.characters || []}
            assets={data?.assets?.assets || []}
            onSelectCharacter={(charId) => setSelectedCharacterId(charId)}
            onRefresh={() => refreshProject()}
            onToast={showToast}
          />
        ) : view === "motion" ? (
          <MotionView
            key={`motion-${projectPath}`}
            projectPath={projectPath}
            onNavigateStudio={() => setView("studio")}
            onToast={showToast}
          />
        ) : view === "design" ? (
          <LyricsView
            key={`lyrics-${projectPath}`}
            projectPath={projectPath}
            mode="design"
            onModeChange={(nextMode) => setView(nextMode)}
            onNavigateStudio={() => setView("studio")}
            onToast={showToast}
          />
        ) : view === "project" ? (
          <ProjectView
            projectPath={projectPath}
            data={data}
            onRefresh={() => refreshProject()}
            onNavigate={navigateTo}
            onToast={showToast}
          />
        ) : (
          <PreservedStoryboard
            key={`storyboard-${projectPath}`}
            active={view === "storyboard"}
            projectPath={projectPath}
            storyboard={data?.storyboard}
            onRefresh={() => refreshProject()}
            onNavigateStudio={() => setView("studio")}
            onToast={showToast}
          />
        )}
      </div>

      <AppFooter
        view={view}
        data={data}
        projectPath={projectPath}
        onNavigate={setView}
        onToast={showToast}
      />

      <div id="app-toast" className={`toast app-toast ${toastVisible ? "show" : ""}`} role="status" aria-live="polite">
        {toastMsg}
      </div>
    </div>
  );
};

export default App;
