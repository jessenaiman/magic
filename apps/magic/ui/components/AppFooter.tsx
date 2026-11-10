import { useEffect, useState } from "react";
import type { ViewMode } from "../App";
import type { StoryboardData, WorkshopData } from "../api";
import { workspaceDrafts } from "../workspace-drafts";

/**
 * Magic Studio's Tauri status footer:
 * - Workflow stages (`Song`, `Lyrics`, `Transcript`, `Storyboard`, `Animate`, `Studio`)
 * - Active view summary read from `session_snapshot`
 * - Save state aggregated from `workspaceDrafts()`
 */

type Tone = "ready" | "needs-attention" | "failed" | "working" | "idle";

interface StageDefinition {
  id: string;
  label: string;
  view: ViewMode;
}

const STAGES: ReadonlyArray<StageDefinition> = [
  { id: "song", label: "Song", view: "project" },
  { id: "lyrics", label: "Lyrics", view: "design" },
  { id: "transcript", label: "Transcript", view: "project" },
  { id: "storyboard", label: "Storyboard", view: "storyboard" },
  { id: "animate", label: "Animate", view: "animation" },
  { id: "studio", label: "Studio", view: "studio" },
];

const INCOMPLETE_STATES = new Set(["needs-work", "needs_work", "untraced", "incomplete", "review"]);

interface Prerequisite {
  id?: string;
  state?: string;
  detail?: string;
}

interface Props {
  view: ViewMode;
  data: WorkshopData | null;
  projectPath: string | null;
  onNavigate: (view: ViewMode) => void;
  onToast: (message: string) => void;
}

function prerequisites(data: WorkshopData | null): Record<string, Prerequisite> {
  const raw = data?.status?.prerequisites;
  if (!raw) return {};
  if (Array.isArray(raw)) {
    return Object.fromEntries(raw.filter((entry): entry is Prerequisite & { id: string } => !!entry?.id).map(entry => [entry.id, entry]));
  }
  return raw;
}

function countUntraced(value: StoryboardData["untraced"] | undefined): number {
  if (Array.isArray(value)) return value.length;
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function untracedCount(storyboard: StoryboardData | undefined): number {
  const top = countUntraced(storyboard?.untraced);
  if (top) return top;
  return (storyboard?.pages ?? []).reduce((total, page) => total + countUntraced(page.untraced), 0);
}

function toneFor(state: string | undefined): Tone {
  if (state === "ready") return "ready";
  if (state === "failed") return "failed";
  if (state && INCOMPLETE_STATES.has(state)) return "needs-attention";
  return "idle";
}

interface StageStatus {
  tone: Tone;
  state: string;
  detail: string;
}

function stageStatus(stage: StageDefinition, data: WorkshopData | null): StageStatus {
  const prerequisite = prerequisites(data)[stage.id];
  const engineStage = data?.status?.stages?.find(entry => entry.id === stage.id);
  let state = prerequisite?.state ?? engineStage?.state ?? (engineStage?.done ? "ready" : "not-started");
  let detail = prerequisite?.detail ?? engineStage?.why ?? "";

  if (stage.id === "storyboard") {
    const untraced = untracedCount(data?.storyboard);
    if (untraced) {
      state = "untraced";
      detail = `${untraced} untraced asset${untraced === 1 ? "" : "s"}${detail ? ` · ${detail}` : ""}`;
    } else if (data?.storyboard?.complete === false && state === "ready") {
      state = "needs-work";
    }
  }

  return { tone: toneFor(state), state, detail };
}

function plural(count: number, word: string): string {
  return `${count} ${word}${count === 1 ? "" : "s"}`;
}

interface ProjectFacts {
  pages: number;
  objects: number;
  untraced: number;
}

function projectFacts(data: WorkshopData | null): ProjectFacts {
  const storyboard = data?.storyboard;
  return {
    pages: storyboard?.page_count ?? storyboard?.pages?.length ?? 0,
    objects: data?.assets?.assets?.length ?? 0,
    untraced: untracedCount(storyboard),
  };
}

function storyboardLine(facts: ProjectFacts): string {
  return [
    facts.pages ? `Storyboard · ${plural(facts.pages, "page")}` : "Storyboard · not built yet",
    facts.untraced ? plural(facts.untraced, "untraced asset") : null,
    plural(facts.objects, "object"),
  ].filter(Boolean).join(" · ");
}

function contextText(view: ViewMode, data: WorkshopData | null, projectPath: string | null): string {
  if (!projectPath || view === "home") {
    return view === "gallery" ? "Gallery · browse artwork, or open a project" : "Choose a project or start a new song";
  }

  const facts = projectFacts(data);
  switch (view) {
    case "gallery":
      return `Gallery · ${plural(facts.objects, "object")} in this project`;
    case "studio": {
      const timeline = data?.status?.film_timeline;
      return timeline?.exists ? timeline.path || "Final FilmCraft timeline linked" : "No final FilmCraft timeline linked";
    }
    case "design":
      return `Lyrics · ${plural(facts.objects, "object")} in this project`;
    case "motion":
      return `Motion · ${plural(facts.objects, "object")} in this project`;
    case "animation": {
      const animate = data?.status?.stages?.find(entry => entry.id === "animate");
      const subjects = animate?.assets?.length ?? 0;
      return subjects
        ? `Animation · ${plural(subjects, "animated subject")} · ${plural(facts.objects, "object")}`
        : "Animation · no animated subject chosen";
    }
    default:
      return storyboardLine(facts);
  }
}

function useDraftState() {
  const [drafts, setDrafts] = useState(() => workspaceDrafts());
  useEffect(() => {
    const refresh = () => setDrafts(workspaceDrafts());
    refresh();
    const timer = window.setInterval(refresh, 1000);
    const onFocus = () => refresh();
    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onFocus);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onFocus);
    };
  }, []);
  return drafts;
}

export const AppFooter: React.FC<Props> = ({ view, data, projectPath, onNavigate, onToast }) => {
  const drafts = useDraftState();

  const go = (stage: StageDefinition) => {
    if (!projectPath) {
      onToast("Open a project before opening a workflow stage");
      return;
    }
    onNavigate(stage.view);
  };

  const save = drafts.saving
    ? { tone: "working" as Tone, label: "Saving", title: "A document is saving" }
    : drafts.dirty
      ? { tone: "needs-attention" as Tone, label: "Unsaved", title: drafts.drafts.join(" · ") || "A document has unsaved edits" }
      : projectPath
        ? { tone: "ready" as Tone, label: "Saved", title: "No tracked document edits are waiting to be saved" }
        : { tone: "idle" as Tone, label: "Idle", title: "No project is open" };

  const context = contextText(view, data, projectPath);
  const currentStage = view === "storyboard" ? "storyboard"
    : view === "animation" ? "animate"
      : view === "studio" ? "studio"
        : view === "design" || view === "motion" ? "lyrics"
          : null;

  return (
    <footer id="app-footer" className="app-footer" role="contentinfo" aria-label="Application status footer">
      <nav id="footer-stages" className="app-footer-stages" aria-label="Workflow stages">
        {STAGES.map(stage => {
          const status = stageStatus(stage, data);
          return (
            <button
              key={stage.id}
              id={`footer-stage-${stage.id}`}
              type="button"
              className="app-footer-stage"
              data-current={currentStage === stage.id ? "true" : undefined}
              aria-label={`${stage.label}: ${status.state.replace(/-/g, " ")}`}
              title={`${stage.label} · ${status.detail || status.state.replace(/-/g, " ")}`}
              onClick={() => go(stage)}
            >
              <span className={`app-footer-dot ${status.tone}`} aria-hidden="true" />
              <span className="app-footer-stage-label">{stage.label}</span>
            </button>
          );
        })}
      </nav>
      <div id="footer-context" className="app-footer-context" aria-live="polite" title={context}>{context}</div>
      <span id="footer-save-state" className={`app-footer-save app-footer-save-${save.tone}`} title={save.title}>
        <span className={`app-footer-dot ${save.tone}`} aria-hidden="true" />
        {save.label}
      </span>
    </footer>
  );
};
