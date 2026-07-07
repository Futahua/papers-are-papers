import {
  Activity,
  AlertTriangle,
  ArrowUp,
  Check,
  ChevronDown,
  Circle,
  Eye,
  FileClock,
  Laptop,
  LoaderCircle,
  Menu,
  MessageSquareText,
  PanelLeftClose,
  PanelLeftOpen,
  Pause,
  Pencil,
  Play,
  Plus,
  RotateCcw,
  Settings,
  ShieldCheck,
  Sparkles,
  Square,
  Trash2,
  Wrench,
  X,
} from "lucide-react";
import {
  FormEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { inspectElement } from "./inspect";
import { papers } from "./papers";
import type { ChangeRecord, InspectSelection } from "./types";
import { useAgent } from "./use-agent";

const stateCopy: Record<string, string> = {
  idle: "Ready",
  planning: "Understanding",
  acting: "Working",
  awaiting_approval: "Waiting for you",
  previewing: "Preview ready",
  completed: "Done",
  paused: "Paused",
  cancelled: "Stopped",
  failed: "Needs attention",
};

const renderInlineMarkdown = (text: string): ReactNode[] => {
  const pieces: ReactNode[] = [];
  const pattern = /(\*\*[^*]+?\*\*|`[^`]+?`)/g;
  let cursor = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    if (match.index > cursor) {
      pieces.push(text.slice(cursor, match.index));
    }

    const token = match[0];
    const key = `${match.index}-${token}`;
    if (token.startsWith("**")) {
      pieces.push(<strong key={key}>{token.slice(2, -2)}</strong>);
    } else {
      pieces.push(<code key={key}>{token.slice(1, -1)}</code>);
    }
    cursor = match.index + token.length;
  }

  if (cursor < text.length) {
    pieces.push(text.slice(cursor));
  }

  return pieces;
};

function MarkdownText({ text }: { text: string }) {
  const blocks: Array<
    | { kind: "paragraph"; text: string }
    | { kind: "list"; items: string[] }
  > = [];
  let paragraph: string[] = [];
  let list: string[] = [];

  const flushParagraph = () => {
    if (paragraph.length > 0) {
      blocks.push({ kind: "paragraph", text: paragraph.join(" ") });
      paragraph = [];
    }
  };

  const flushList = () => {
    if (list.length > 0) {
      blocks.push({ kind: "list", items: list });
      list = [];
    }
  };

  text.split(/\r?\n/).forEach((line) => {
    const bullet = line.match(/^\s*[-*]\s+(.+)$/);
    if (bullet) {
      flushParagraph();
      list.push(bullet[1]);
      return;
    }

    if (!line.trim()) {
      flushParagraph();
      flushList();
      return;
    }

    flushList();
    paragraph.push(line.trim());
  });

  flushParagraph();
  flushList();

  return (
    <div className="markdown-text">
      {blocks.map((block, index) =>
        block.kind === "list" ? (
          <ul key={index}>
            {block.items.map((item, itemIndex) => (
              <li key={itemIndex}>{renderInlineMarkdown(item)}</li>
            ))}
          </ul>
        ) : (
          <p key={index}>{renderInlineMarkdown(block.text)}</p>
        ),
      )}
    </div>
  );
}

export function App() {
  const agent = useAgent();
  const [draft, setDraft] = useState("");
  const [menuOpen, setMenuOpen] = useState(false);
  const [sessionRailOpen, setSessionRailOpen] = useState(true);
  const [activityOpen, setActivityOpen] = useState(true);
  const [inspectMode, setInspectMode] = useState(false);
  const [selection, setSelection] = useState<InspectSelection | null>(null);
  const [selectionPrompt, setSelectionPrompt] = useState("");
  const [clarifyAnswer, setClarifyAnswer] = useState("");
  const [foreground, setForeground] = useState("your current app");
  const [changes, setChanges] = useState<ChangeRecord[]>([]);
  const [setupMessage, setSetupMessage] = useState("");
  const messagesEnd = useRef<HTMLDivElement>(null);
  const channel = useMemo(() => new BroadcastChannel("papers-agent"), []);

  useEffect(() => {
    channel.postMessage({
      type: "status",
      status: agent.statusLabel,
      state: agent.runState,
      active: ["planning", "acting", "awaiting_approval"].includes(agent.runState),
    });
  }, [agent.runState, agent.statusLabel, channel]);

  useEffect(() => {
    channel.onmessage = (event) => {
      const type = event.data?.type;
      if (type === "pause") void agent.pause();
      if (type === "resume") void agent.resume();
      if (type === "stop") void agent.stop();
      if (type === "new") agent.newConversation();
      if (type === "quickPrompt" && typeof event.data.prompt === "string") {
        void agent.send(event.data.prompt, {
          context: `The user invoked Papers while working with ${event.data.target || foreground}.`,
        });
      }
    };
  }, [agent, channel, foreground]);

  useEffect(() => {
    messagesEnd.current?.scrollIntoView({ behavior: "smooth" });
  }, [agent.messages]);

  useEffect(() => {
    void papers.foregroundApp().then(setForeground).catch(() => undefined);
    void papers.listChanges().then(setChanges).catch(() => undefined);
  }, []);

  useEffect(() => {
    if (!inspectMode) {
      document.documentElement.classList.remove("is-inspecting");
      return;
    }

    document.documentElement.classList.add("is-inspecting");
    const onPointerOver = (event: PointerEvent) => {
      const target = event.target as HTMLElement | null;
      target?.closest<HTMLElement>("[data-papers-node]")?.classList.add("inspect-hover");
    };
    const onPointerOut = (event: PointerEvent) => {
      const target = event.target as HTMLElement | null;
      target?.closest<HTMLElement>("[data-papers-node]")?.classList.remove("inspect-hover");
    };
    const onClick = (event: MouseEvent) => {
      const target = (event.target as HTMLElement | null)?.closest<HTMLElement>(
        "[data-papers-node]",
      );
      if (!target || target.closest(".inspect-popover") || target.closest(".topbar")) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      document
        .querySelectorAll(".inspect-selected")
        .forEach((node) => node.classList.remove("inspect-selected"));
      target.classList.add("inspect-selected");
      setSelection(inspectElement(target));
      setInspectMode(false);
    };

    document.addEventListener("pointerover", onPointerOver, true);
    document.addEventListener("pointerout", onPointerOut, true);
    document.addEventListener("click", onClick, true);
    return () => {
      document.documentElement.classList.remove("is-inspecting");
      document.removeEventListener("pointerover", onPointerOver, true);
      document.removeEventListener("pointerout", onPointerOut, true);
      document.removeEventListener("click", onClick, true);
      document
        .querySelectorAll(".inspect-hover")
        .forEach((node) => node.classList.remove("inspect-hover"));
    };
  }, [inspectMode]);

  const submit = useCallback(
    async (event?: FormEvent) => {
      event?.preventDefault();
      if (!draft.trim() || !agent.ready) return;
      const text = draft;
      setDraft("");
      await papers.showCompanion().catch(() => undefined);
      await agent.send(text, {
        context: `The user is currently working with ${foreground}.`,
      });
    },
    [agent, draft, foreground],
  );

  const renameConversation = useCallback(
    async (sessionId: string, currentTitle: string) => {
      const title = window.prompt("Rename this conversation", currentTitle);
      if (title == null || title.trim() === currentTitle.trim()) {
        return;
      }
      const session = agent.sessions.find((item) => item.id === sessionId);
      if (!session) {
        return;
      }
      try {
        await agent.renameSession(session, title);
      } catch (reason) {
        agent.setError(reason instanceof Error ? reason.message : String(reason));
      }
    },
    [agent],
  );

  const deleteConversation = useCallback(
    async (sessionId: string, title: string) => {
      const session = agent.sessions.find((item) => item.id === sessionId);
      if (!session) {
        return;
      }
      if (!window.confirm(`Delete "${title}" from Papers history?`)) {
        return;
      }
      try {
        await agent.deleteSession(session);
      } catch (reason) {
        agent.setError(reason instanceof Error ? reason.message : String(reason));
      }
    },
    [agent],
  );

  const submitInspect = useCallback(async () => {
    if (!selection || !selectionPrompt.trim() || !agent.ready) return;
    const request = selectionPrompt.trim();
    const change = await papers.createChange(
      request.slice(0, 70),
      request,
      selection,
    );
    setChanges((current) => [change, ...current]);
    setSelectionPrompt("");
    setSelection(null);
    document
      .querySelectorAll(".inspect-selected")
      .forEach((node) => node.classList.remove("inspect-selected"));
    await agent.send(request, {
      mode: "builder",
      cwd: change.worktree_path,
      selection,
      changeId: change.id,
    });
  }, [agent, selection, selectionPrompt]);

  const connectAction = agent.runtime?.installed ? agent.start : agent.install;
  const isWorking = agent.runState === "planning" || agent.runState === "acting";

  return (
    <div
      className={`app-frame ${sessionRailOpen ? "" : "sessions-collapsed"} ${
        activityOpen ? "" : "work-collapsed"
      }`}
    >
      <header className="topbar">
        <div className="topbar-left">
          <button
            className="pill-button"
            onClick={() => setMenuOpen((open) => !open)}
            aria-expanded={menuOpen}
          >
            <Menu size={16} />
            <span>Basic</span>
          </button>
          <button
            className="pill-button icon-pill"
            onClick={() => setSessionRailOpen((open) => !open)}
            title={sessionRailOpen ? "Hide chat history" : "Show chat history"}
          >
            {sessionRailOpen ? (
              <PanelLeftClose size={16} />
            ) : (
              <PanelLeftOpen size={16} />
            )}
            <span>History</span>
          </button>
        </div>

        <div className="wordmark">
          <span>Papers</span>
          <Circle size={5} fill="currentColor" />
          <span>Papers</span>
        </div>

        <div className="topbar-actions">
          <button
            className={`inspect-button ${inspectMode ? "active" : ""}`}
            onClick={() => setInspectMode((active) => !active)}
            title="Point at part of Papers and ask to change it"
          >
            <Eye size={15} />
            <span>{inspectMode ? "Choose something" : "Inspect"}</span>
          </button>
          <span className={`agent-badge ${agent.ready ? "ready" : ""}`}>
            <i />
            {agent.statusLabel}
          </span>
        </div>
      </header>

      {menuOpen && (
        <div className="basic-menu">
          <p className="eyebrow">Basic</p>
          <button className="basic-row active">
            <MessageSquareText size={18} />
            <span>
              <strong>Agent</strong>
              <small>The first foundation</small>
            </span>
            <span className="row-value">{agent.ready ? "Ready" : "Setup"}</span>
          </button>
          <button className="basic-row" disabled>
            <Sparkles size={18} />
            <span>
              <strong>Backpacks</strong>
              <small>The agent will make these later</small>
            </span>
            <span className="row-value">Deferred</span>
          </button>
          <button className="basic-row" disabled>
            <Wrench size={18} />
            <span>
              <strong>Tools</strong>
              <small>Provided through Hermes</small>
            </span>
            <span className="row-value">Managed</span>
          </button>
          <button
            className="basic-row"
            disabled={!agent.runtime?.installed}
            onClick={async () => {
              try {
                setSetupMessage(await papers.startNousLogin());
              } catch (reason) {
                agent.setError(
                  reason instanceof Error ? reason.message : String(reason),
                );
              }
            }}
          >
            <Settings size={18} />
            <span>
              <strong>Nous account</strong>
              <small>
                {setupMessage || "Sign in for models and managed web tools"}
              </small>
            </span>
            <span className="row-value">
              {agent.runtime?.installed ? "Open" : "After install"}
            </span>
          </button>
        </div>
      )}

      <aside className={`session-rail ${sessionRailOpen ? "" : "collapsed"}`}>
        <button className="new-thread" onClick={agent.newConversation}>
          <Plus size={16} />
          New conversation
        </button>
        <div className="rail-section">
          <p className="eyebrow">Chat history</p>
          {agent.sessions.length === 0 ? (
            <p className="rail-empty">Your conversations will live here.</p>
          ) : (
            agent.sessions.slice(0, 30).map((session) => (
              <div
                className={`session-row ${
                  agent.activeSession?.id === session.id ? "selected" : ""
                }`}
                key={session.id}
              >
                <button
                  className="session-open"
                  disabled={!agent.ready || !session.hermes_session_id}
                  onClick={() => void agent.openSession(session)}
                  title={
                    session.hermes_session_id
                      ? "Resume this Hermes conversation"
                      : "This conversation did not reach Hermes"
                  }
                >
                  <span>{session.title}</span>
                  <small>
                    {session.mode === "builder" ? "Builder" : "Operator"} ·{" "}
                    {stateCopy[session.state] ?? session.state}
                  </small>
                </button>
                <div className="session-actions">
                  <button
                    onClick={() => void renameConversation(session.id, session.title)}
                    title="Rename conversation"
                  >
                    <Pencil size={13} />
                  </button>
                  <button
                    onClick={() => void deleteConversation(session.id, session.title)}
                    title="Delete conversation"
                  >
                    <Trash2 size={13} />
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
        <div className="rail-bottom">
          <div className="runtime-line">
            <ShieldCheck size={16} />
            <span>
              <strong>Guarded actions</strong>
              <small>Consequences stop for preview</small>
            </span>
          </div>
          <div className="runtime-line">
            <FileClock size={16} />
            <span>
              <strong>Recovery</strong>
              <small>Previous version remembered</small>
            </span>
          </div>
        </div>
      </aside>

      <main className="conversation">
        {agent.messages.length === 0 ? (
          <section className="welcome">
            <div className="agent-orbit">
              <span className="orbit orbit-one" />
              <span className="orbit orbit-two" />
              <span className="agent-core">
                <Sparkles size={23} />
              </span>
            </div>
            <p className="eyebrow">Agent first</p>
            <h1>What should we make happen?</h1>
            <p className="welcome-copy">
              Papers can become the place where you ask, point, watch, and approve.
              Hermes does the work; Papers keeps it understandable and recoverable.
            </p>

            {!agent.ready ? (
              <div className="setup-card">
                <div className="setup-icon">
                  {agent.installing ? (
                    <LoaderCircle className="spin" size={22} />
                  ) : (
                    <Laptop size={22} />
                  )}
                </div>
                <div>
                  <strong>
                    {agent.runtime?.installed
                      ? "Hermes is installed but stopped"
                      : "Install the local agent engine"}
                  </strong>
                  <p>
                    {agent.runtime?.message ||
                      "Papers installs its pinned Hermes runtime in its own private folder."}
                  </p>
                </div>
                <button onClick={() => void connectAction()} disabled={agent.installing}>
                  {agent.installing
                    ? "Installing…"
                    : agent.runtime?.installed
                      ? "Start"
                      : "Install Hermes"}
                </button>
              </div>
            ) : (
              <div className="suggestions">
                <button onClick={() => setDraft("Organize the files on my desktop into a clear plan without moving anything yet.")}>
                  <span>Understand my desktop</span>
                  <small>Inspect first, change nothing</small>
                </button>
                <button onClick={() => setDraft("Research a topic with me and keep the useful sources together.")}>
                  <span>Research with me</span>
                  <small>Use the web and report sources</small>
                </button>
                <button onClick={() => setInspectMode(true)}>
                  <span>Change Papers itself</span>
                  <small>Point at something visible</small>
                </button>
              </div>
            )}
          </section>
        ) : (
          <section className="message-list" aria-live="polite">
            <div className="conversation-heading">
              <p className="eyebrow">
                {agent.activeSession?.mode === "builder"
                  ? "Papers Builder"
                  : "Computer Operator"}
              </p>
              <h2>{agent.activeSession?.title}</h2>
            </div>
            {agent.messages.map((message) => (
              <article className={`message ${message.role}`} key={message.id}>
                <div className="message-label">
                  {message.role === "user" ? "You" : "Papers"}
                </div>
                <div className="message-body">
                  {message.text ? (
                    <MarkdownText text={message.text} />
                  ) : message.pending ? (
                      <span className="typing">
                        <i />
                        <i />
                        <i />
                      </span>
                    ) : null}
                </div>
              </article>
            ))}
            <div ref={messagesEnd} />
          </section>
        )}

        {agent.error && (
          <div className="inline-error">
            <AlertTriangle size={17} />
            <span>{agent.error}</span>
            <button onClick={() => agent.setError(null)} aria-label="Dismiss error">
              <X size={15} />
            </button>
          </div>
        )}

        {agent.approval && (
          <section className="approval-card">
            <div className="approval-top">
              <span className="risk-icon">
                <ShieldCheck size={20} />
              </span>
              <div>
                <p className="eyebrow">Exact preview required</p>
                <h3>{agent.approval.description}</h3>
              </div>
            </div>
            {agent.approval.command && (
              <pre>{agent.approval.command}</pre>
            )}
            <div className="approval-facts">
              <span>Risk: {agent.approval.risk}</span>
              <span>May be only partly reversible</span>
            </div>
            <div className="approval-actions">
              <button className="secondary" onClick={() => void agent.answerApproval("deny")}>
                Don’t do it
              </button>
              <button className="primary" onClick={() => void agent.answerApproval("once")}>
                <Check size={15} />
                Allow this once
              </button>
            </div>
          </section>
        )}

        {agent.clarify && (
          <form
            className="clarify-card"
            onSubmit={(event) => {
              event.preventDefault();
              void agent.answerClarify(clarifyAnswer);
              setClarifyAnswer("");
            }}
          >
            <p className="eyebrow">The agent needs your judgment</p>
            <h3>{agent.clarify.question}</h3>
            {agent.clarify.choices.length > 0 && (
              <div className="choice-row">
                {agent.clarify.choices.map((choice) => (
                  <button
                    type="button"
                    key={choice}
                    onClick={() => setClarifyAnswer(choice)}
                    className={clarifyAnswer === choice ? "selected" : ""}
                  >
                    {choice}
                  </button>
                ))}
              </div>
            )}
            <div className="clarify-input">
              <input
                value={clarifyAnswer}
                onChange={(event) => setClarifyAnswer(event.target.value)}
                placeholder="Answer in your own words"
              />
              <button type="submit">Continue</button>
            </div>
          </form>
        )}

        <form className="composer" onSubmit={submit}>
          <div className="composer-context">
            <span>
              <Laptop size={13} />
              {foreground}
            </span>
            <span className="permission-note">
              Consequential actions pause for you
            </span>
          </div>
          <textarea
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                void submit();
              }
            }}
            placeholder={
              agent.ready
                ? "Ask Papers to understand, create, organize, or operate…"
                : "Connect Hermes to begin"
            }
            disabled={!agent.ready}
            rows={2}
          />
          <div className="composer-actions">
            <span>Enter to send · Shift+Enter for a new line</span>
            <div>
              {isWorking && (
                <>
                  <button
                    type="button"
                    className="control"
                    onClick={() => void agent.pause()}
                    title="Pause after interrupting the current turn"
                  >
                    <Pause size={15} />
                  </button>
                  <button
                    type="button"
                    className="control"
                    onClick={() => void agent.stop()}
                    title="Stop"
                  >
                    <Square size={13} fill="currentColor" />
                  </button>
                </>
              )}
              {agent.runState === "paused" && (
                <button
                  type="button"
                  className="control"
                  onClick={() => void agent.resume()}
                  title="Continue"
                >
                  <Play size={15} fill="currentColor" />
                </button>
              )}
              <button
                type="submit"
                className="send"
                disabled={!agent.ready || !draft.trim()}
                aria-label="Send"
              >
                <ArrowUp size={18} />
              </button>
            </div>
          </div>
        </form>
      </main>

      <aside className={`activity-rail work-rail ${activityOpen ? "" : "collapsed"}`}>
        <button
          className="activity-toggle"
          onClick={() => setActivityOpen((open) => !open)}
        >
          <Activity size={16} />
          <span>Work</span>
          <ChevronDown size={14} />
        </button>
        {activityOpen && (
          <>
            <div className="current-state">
              <span className={`state-pulse ${isWorking ? "working" : ""}`}>
                {isWorking ? <LoaderCircle className="spin" size={16} /> : <Check size={15} />}
              </span>
              <div>
                <strong>{stateCopy[agent.runState]}</strong>
                <small>{agent.statusLabel}</small>
              </div>
            </div>
            <div className="work-section">
              <p className="eyebrow">Now</p>
              {agent.workItems.length === 0 ? (
                <p className="activity-empty">
                  Reasoning summaries, tool steps, files, diffs, and approvals will appear here.
                </p>
              ) : (
                agent.workItems.slice(0, 10).map((item) => (
                  <div className={`activity-item work-item ${item.type}`} key={item.id}>
                    <span className="activity-dot" />
                    <div>
                      <strong>{item.title}</strong>
                      {item.detail && <small>{item.detail}</small>}
                      <em>{item.type.replaceAll("_", " ")}</em>
                    </div>
                  </div>
                ))
              )}
            </div>
            {agent.activities.length > 0 && (
              <div className="work-section compact">
                <p className="eyebrow">Timeline</p>
                {agent.activities.slice(0, 6).map((item) => (
                  <div className={`activity-item ${item.kind}`} key={item.id}>
                    <span className="activity-dot" />
                    <div>
                      <strong>{item.title}</strong>
                      {item.detail && <small>{item.detail}</small>}
                    </div>
                  </div>
                ))}
              </div>
            )}
            {changes.length > 0 && (
              <div className="change-list work-section">
                <p className="eyebrow">Self-edits</p>
                {changes.slice(0, 3).map((change) => (
                  <div className="change-item" key={change.id}>
                    <strong>{change.title}</strong>
                    <small>{change.status.replaceAll("_", " ")}</small>
                    <div>
                      {change.status === "staging" && (
                        <button
                          onClick={async () => {
                            const updated = await papers.buildChange(change.id);
                            setChanges((items) =>
                              items.map((item) =>
                                item.id === updated.id ? updated : item,
                              ),
                            );
                          }}
                        >
                          Build preview
                        </button>
                      )}
                      {change.status === "preview_ready" && (
                        <>
                          <button
                            onClick={() => void papers.launchChangePreview(change.id)}
                          >
                            Experience
                          </button>
                          <button
                            onClick={async () => {
                              const updated = await papers.acceptChange(change.id);
                              setChanges((items) =>
                                items.map((item) =>
                                  item.id === updated.id ? updated : item,
                                ),
                              );
                            }}
                          >
                            Keep
                          </button>
                        </>
                      )}
                      {!["accepted", "rejected"].includes(change.status) && (
                        <button
                          className="danger-link"
                          onClick={async () => {
                            await papers.rejectChange(change.id);
                            setChanges((items) =>
                              items.filter((item) => item.id !== change.id),
                            );
                          }}
                        >
                          Reject
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
            <button
              className="rollback-link"
              onClick={async () => {
                const result = await papers.rollbackLast();
                agent.setError(result);
              }}
            >
              <RotateCcw size={14} />
              Return to last working version
            </button>
          </>
        )}
      </aside>

      {inspectMode && (
        <div className="inspect-banner">
          <Eye size={16} />
          <span>Choose the part of Papers you want to change</span>
          <button onClick={() => setInspectMode(false)}>Cancel</button>
        </div>
      )}

      {selection && (
        <section className="inspect-popover">
          <button
            className="popover-close"
            onClick={() => {
              setSelection(null);
              document
                .querySelectorAll(".inspect-selected")
                .forEach((node) => node.classList.remove("inspect-selected"));
            }}
          >
            <X size={15} />
          </button>
          <p className="eyebrow">Change this part</p>
          <strong>{selection.text || selection.role}</strong>
          <small>{selection.source}</small>
          <textarea
            autoFocus
            rows={3}
            value={selectionPrompt}
            onChange={(event) => setSelectionPrompt(event.target.value)}
            placeholder="Describe how this should look or behave…"
          />
          <button
            className="primary"
            disabled={!agent.ready || !selectionPrompt.trim()}
            onClick={() => void submitInspect()}
          >
            <Sparkles size={15} />
            Make a temporary version
          </button>
        </section>
      )}
    </div>
  );
}
