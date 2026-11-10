import React, { useCallback, useEffect, useState } from "react";
import { executeCommand } from "../api";
import { Button, IconButton, SelectControl } from "./controls";

interface Props {
  open: boolean;
  onClose: () => void;
  onToast: (msg: string) => void;
}

export const FidgetPanel: React.FC<Props> = ({ open, onClose, onToast }) => {
  const [config, setConfig] = useState<Record<string, unknown> | null>(null);
  const [status, setStatus] = useState<string>("");
  const [prompt, setPrompt] = useState<string>("");
  const [brief, setBrief] = useState<string>("");

  const load = useCallback(async () => {
    try {
      const c = (await executeCommand<Record<string, unknown>>("agent.config")) as Record<string, unknown>;
      setConfig(c);
      const roles = ((c.craft_ai as Record<string, unknown> | undefined)?.roles as Array<Record<string, unknown>> | undefined) ?? [];
      const f = roles.find((r) => r.id === "fidget");
      setStatus(f ? `Fidget: ${(f.status as Record<string, unknown> | undefined)?.state ?? "unavailable"}` : "Fidget: no role");
    } catch (error) {
      setStatus(String(error));
    }
  }, []);

  useEffect(() => {
    if (open) void load();
  }, [open, load]);

  const save = async (agent: string) => {
    try {
      setConfig(await executeCommand<Record<string, unknown>>("agent.use", { agent }) as Record<string, unknown>);
      onToast(`Fidget agent: ${agent}`);
      void load();
    } catch (error) {
      onToast(`Could not save agent: ${error}`);
    }
  };

  const prepare = async () => {
    try {
      const r = (await executeCommand<Record<string, unknown>>("fidget.prepare", { kind: "custom", prompt })) as Record<string, unknown>;
      setBrief(String((r.fidget as Record<string, unknown> | undefined)?.markdown ?? JSON.stringify(r, null, 2)));
    } catch (error) {
      onToast(String(error));
    }
  };

  if (!open) return null;
  const agents = Array.isArray(config?.agents) ? (config?.agents as Array<Record<string, unknown>>) : [];

  return (
    <div id="fidget-panel" className="fidget-panel" role="region" aria-label="Fidget agent">
      <div className="fidget-panel-head">
        <strong>Fidget</strong>
        <span className="fidget-panel-sub">{status}</span>
        <IconButton id="fidget-panel-close" size="xs" variant="ghost" icon="close" label="Close" onClick={onClose} />
      </div>
      <section className="fidget-panel-section">
        <h4>Agent</h4>
        <SelectControl
          id="fidget-agent-select"
          controlSize="sm"
          value={String(config?.agent ?? "")}
          onChange={(e) => void save(e.target.value)}
          aria-label="Fidget agent"
        >
          {agents.length === 0 && <option value="">Loading…</option>}
          {agents.map((a, i) => (
            <option key={i} value={String(a.name ?? "")}>{String(a.name ?? "")}</option>
          ))}
        </SelectControl>
      </section>
      <section className="fidget-panel-section">
        <h4>Brief</h4>
        <textarea
          id="fidget-prompt"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          rows={3}
          placeholder="Describe the drawing the brief should prepare…"
          aria-label="Fidget brief prompt"
        />
        <Button id="fidget-prepare" size="sm" variant="primary" icon="wand" label="Prepare" title="Prepare structured drawing brief" onClick={() => void prepare()} />
        {brief && <details open><summary>Brief</summary><pre>{brief}</pre></details>}
      </section>
    </div>
  );
};
