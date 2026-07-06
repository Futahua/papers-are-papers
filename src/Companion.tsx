import { Activity, Maximize2, Pause, Play, Square, X } from "lucide-react";
import { FormEvent, useEffect, useMemo, useState } from "react";
import { papers } from "./papers";

interface CompanionStatus {
  status: string;
  state: string;
  active: boolean;
}

export function Companion() {
  const [status, setStatus] = useState<CompanionStatus>({
    status: "Agent ready",
    state: "idle",
    active: false,
  });
  const [target, setTarget] = useState("Current app");
  const [draft, setDraft] = useState("");
  const channel = useMemo(() => new BroadcastChannel("papers-agent"), []);

  useEffect(() => {
    channel.onmessage = (event) => {
      if (event.data?.type === "status") {
        setStatus(event.data as CompanionStatus);
      }
    };
    void papers.foregroundApp().then(setTarget).catch(() => undefined);
  }, [channel]);

  const paused = status.state === "paused";
  const submit = (event: FormEvent) => {
    event.preventDefault();
    const prompt = draft.trim();
    if (!prompt) return;
    channel.postMessage({ type: "quickPrompt", prompt, target });
    setDraft("");
  };

  return (
    <div className={`companion ${status.active ? "active" : ""}`}>
      <div className="companion-top">
        <span className="companion-mark">
          <Activity size={17} />
        </span>
        <div className="companion-copy">
          <strong>{status.status}</strong>
          <small>{target}</small>
        </div>
        <div className="companion-actions">
          <button
            onClick={() => channel.postMessage({ type: paused ? "resume" : "pause" })}
            title={paused ? "Continue" : "Pause"}
          >
            {paused ? <Play size={14} fill="currentColor" /> : <Pause size={14} />}
          </button>
          <button onClick={() => channel.postMessage({ type: "stop" })} title="Stop">
            <Square size={11} fill="currentColor" />
          </button>
          <button onClick={() => void papers.showMain()} title="Open Papers">
            <Maximize2 size={14} />
          </button>
          <button onClick={() => void papers.hideCompanion()} title="Hide">
            <X size={14} />
          </button>
        </div>
      </div>
      <form className="companion-ask" onSubmit={submit}>
        <input
          autoFocus
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          placeholder="Ask Papers about this app…"
        />
        <button type="submit">Ask</button>
      </form>
    </div>
  );
}
