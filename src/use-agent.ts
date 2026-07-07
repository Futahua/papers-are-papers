import { useCallback, useEffect, useMemo, useRef, useState } from "react";

// Exported so .tsx files can name the agent's API without writing
// `ReturnType<typeof useAgent>` inline — the papers-inspect-manifest Vite
// plugin rewrites lowercase identifiers right after `<` (e.g. `<typeof`)
// as JSX, which would break those generics. `.ts` files are not transformed.
import { HermesGateway, type ConnectionState } from "./gateway";
import { papers } from "./papers";
import type {
  ActivityItem,
  ApprovalRequest,
  BootstrapStatus,
  ChatMessage,
  ClarifyRequest,
  GatewayEvent,
  InspectSelection,
  PapersSession,
  RunState,
  WorkItem,
} from "./types";

interface SessionCreateResponse {
  session_id: string;
  stored_session_id?: string;
  messages?: Array<{
    role: "assistant" | "user" | "system" | "tool";
    text?: unknown;
    content?: unknown;
    timestamp?: number;
  }>;
}

interface SendOptions {
  mode?: "operator" | "builder";
  cwd?: string;
  selection?: InspectSelection;
  changeId?: string;
  context?: string;
  title?: string;
}

const gateway = new HermesGateway();
const newId = () => crypto.randomUUID();
const payloadText = (payload: Record<string, unknown> | undefined) => {
  const value = payload?.text ?? payload?.rendered ?? payload?.message;
  return typeof value === "string" ? value : "";
};
const shortJson = (value: unknown, limit = 360) => {
  if (value == null) return "";
  const text =
    typeof value === "string" ? value : JSON.stringify(value, null, 2);
  return text.length > limit ? `${text.slice(0, limit).trim()}…` : text;
};
const readableToolName = (name: string) =>
  name
    .replace(/[_-]+/g, " ")
    .replace(/\bterminal\b/i, "Command")
    .replace(/\bstdout\b/i, "output")
    .trim();
const toolDetail = (payload: Record<string, unknown>) => {
  if (typeof payload.description === "string" && payload.description.trim()) {
    return payload.description;
  }
  if (typeof payload.message === "string" && payload.message.trim()) {
    return payload.message;
  }

  const result = payload.result;
  if (result && typeof result === "object" && !Array.isArray(result)) {
    const record = result as Record<string, unknown>;
    if (record.exit_code === 0 || record.ok === true || record.success === true) {
      return "Finished successfully.";
    }
    if (typeof record.error === "string" && record.error.trim()) {
      return record.error;
    }
    if (typeof record.output === "string" && record.output.trim()) {
      const firstLine = record.output.trim().split(/\r?\n/)[0];
      return firstLine.length > 120 ? `${firstLine.slice(0, 120).trim()}…` : firstLine;
    }
  }

  return "Finished.";
};
const artifactDetail = (payload: Record<string, unknown>) => {
  const value =
    payload.diff ??
    payload.path ??
    payload.file ??
    payload.screenshot ??
    payload.preview ??
    payload.url;
  return value == null ? "" : shortJson(value);
};

export function useAgent() {
  const [runtime, setRuntime] = useState<BootstrapStatus | null>(null);
  const [connection, setConnection] = useState<ConnectionState>("idle");
  const [runState, setRunState] = useState<RunState>("idle");
  const [sessions, setSessions] = useState<PapersSession[]>([]);
  const [activeSession, setActiveSession] = useState<PapersSession | null>(null);
  const [hermesSessionId, setHermesSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [activities, setActivities] = useState<ActivityItem[]>([]);
  const [workItems, setWorkItems] = useState<WorkItem[]>([]);
  const [approval, setApproval] = useState<ApprovalRequest | null>(null);
  const [clarify, setClarify] = useState<ClarifyRequest | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const assistantMessageId = useRef<string | null>(null);
  const activeSessionRef = useRef<PapersSession | null>(null);
  const hermesSessionRef = useRef<string | null>(null);
  const pausedPrompt = useRef<string>("");

  useEffect(() => {
    activeSessionRef.current = activeSession;
  }, [activeSession]);

  useEffect(() => {
    hermesSessionRef.current = hermesSessionId;
  }, [hermesSessionId]);

  const addActivity = useCallback(
    (
      kind: ActivityItem["kind"],
      title: string,
      detail?: string,
      state?: ActivityItem["state"],
    ) => {
      setActivities((current) =>
        [
          {
            id: newId(),
            kind,
            title,
            detail,
            state,
            createdAt: Date.now(),
          },
          ...current,
        ].slice(0, 40),
      );
    },
    [],
  );

  const addWorkItem = useCallback(
    (
      type: WorkItem["type"],
      title: string,
      detail?: string,
      state?: WorkItem["state"],
      metadata?: WorkItem["metadata"],
    ) => {
      setWorkItems((current) =>
        [
          {
            id: newId(),
            type,
            title,
            detail,
            state,
            metadata,
            createdAt: Date.now(),
          },
          ...current,
        ].slice(0, 80),
      );
    },
    [],
  );

  const persistState = useCallback(async (state: RunState) => {
    setRunState(state);
    const session = activeSessionRef.current;
    if (session) {
      await papers.updateSessionState(session.id, state).catch(() => undefined);
    }
  }, []);

  const handleEvent = useCallback(
    (event: GatewayEvent) => {
      const payload = (event.payload ?? {}) as Record<string, unknown>;
      const sid = event.session_id;
      if (sid && hermesSessionRef.current && sid !== hermesSessionRef.current) {
        return;
      }

      const localSession = activeSessionRef.current;
      if (localSession) {
        void papers.recordEvent(localSession.id, event);
      }

      switch (event.type) {
        case "gateway.ready":
          addActivity("status", "Hermes is ready", "The local agent channel is live.", "done");
          addWorkItem("status", "Agent channel ready", "Hermes is connected through Papers.", "done");
          break;
        case "message.start": {
          void persistState("planning");
          addWorkItem(
            "reasoning_summary",
            "Understanding the request",
            "Papers is asking Hermes to plan the next safe step.",
            "running",
          );
          const id = newId();
          assistantMessageId.current = id;
          setMessages((current) => [
            ...current,
            { id, role: "assistant", text: "", createdAt: Date.now(), pending: true },
          ]);
          break;
        }
        case "message.delta": {
          const text = payloadText(payload);
          const id = assistantMessageId.current;
          if (!id || !text) {
            break;
          }
          setMessages((current) =>
            current.map((message) =>
              message.id === id
                ? { ...message, text: `${message.text}${text}` }
                : message,
            ),
          );
          break;
        }
        case "message.complete": {
          const finalText = payloadText(payload);
          const id = assistantMessageId.current;
          setMessages((current) =>
            current.map((message) =>
              message.id === id
                ? {
                    ...message,
                    text: finalText || message.text,
                    pending: false,
                  }
                : message,
            ),
          );
          assistantMessageId.current = null;
          void persistState("completed");
          addActivity("result", "Finished", "The agent completed this turn.", "done");
          addWorkItem(
            "reasoning_summary",
            "Turn completed",
            "The visible response finished streaming.",
            "done",
          );
          break;
        }
        case "tool.start":
        case "tool.progress":
        case "tool.generating": {
          void persistState("acting");
          const name =
            typeof payload.name === "string"
              ? readableToolName(payload.name)
              : "Working";
          const detail =
            typeof payload.description === "string"
              ? payload.description
              : undefined;
          addActivity("tool", name, detail, "running");
          addWorkItem("tool_step", name, detail || "Tool is running.", "running");
          break;
        }
        case "tool.complete": {
          const name =
            typeof payload.name === "string"
              ? readableToolName(payload.name)
              : "Action";
          const artifact = artifactDetail(payload);
          addActivity("tool", `${name} completed`, undefined, "done");
          addWorkItem(
            "tool_step",
            `${name} completed`,
            toolDetail(payload),
            "done",
          );
          if (artifact) {
            addWorkItem(
              "artifact",
              `${name} output`,
              artifact,
              "done",
            );
          }
          break;
        }
        case "approval.request": {
          void persistState("awaiting_approval");
          const command =
            typeof payload.command === "string" ? payload.command : "";
          const description =
            typeof payload.description === "string"
              ? payload.description
              : "Hermes is requesting permission for a consequential action.";
          setApproval({
            sessionId: sid,
            command,
            description,
            allowPermanent: payload.allow_permanent !== false,
            effect: description,
            reversibility: "partly_reversible",
            risk: "high",
          });
          addActivity("warning", "Approval needed", command || description);
          addWorkItem(
            "approval",
            "Approval needed",
            command || description,
            "waiting",
          );
          break;
        }
        case "clarify.request": {
          const requestId =
            typeof payload.request_id === "string" ? payload.request_id : "";
          const question =
            typeof payload.question === "string" ? payload.question : "";
          if (requestId && question) {
            setClarify({
              requestId,
              question,
              sessionId: sid,
              choices: Array.isArray(payload.choices)
                ? payload.choices.filter(
                    (choice): choice is string => typeof choice === "string",
                  )
                : [],
            });
            void persistState("awaiting_approval");
          }
          break;
        }
        case "error": {
          const message =
            payloadText(payload) || "Hermes reported an unknown error.";
          setError(message);
          void persistState("failed");
          addActivity("error", "Agent error", message, "failed");
          addWorkItem("status", "Agent error", message, "failed");
          break;
        }
        default:
          break;
      }
    },
    [addActivity, addWorkItem, persistState],
  );

  useEffect(() => {
    const removeEvent = gateway.onEvent(handleEvent);
    const removeState = gateway.onState(setConnection);

    if (!("__TAURI_INTERNALS__" in window)) {
      setRuntime({
        installed: false,
        running: false,
        connected: false,
        phase: "not_installed",
        package_version: "0.18.0",
        release_tag: "v2026.7.1",
        hermes_home: "",
        install_dir: "",
        message:
          "This browser is displaying the interface only. Installation is available in the native Papers app.",
      });
      setSessions([]);
      return () => {
        removeEvent();
        removeState();
      };
    }

    void Promise.all([papers.bootstrapStatus(), papers.listSessions()])
      .then(([status, storedSessions]) => {
        setRuntime(status);
        setSessions(storedSessions);
        if (status.installed) {
          return papers.startHermes();
        }
        return status;
      })
      .then(async (status) => {
        setRuntime(status);
        if (status.ws_url) {
          await gateway.connect(status.ws_url);
        }
      })
      .catch((reason: unknown) => {
        setError(reason instanceof Error ? reason.message : String(reason));
      });

    return () => {
      removeEvent();
      removeState();
    };
  }, [handleEvent]);

  const install = useCallback(async () => {
    setInstalling(true);
    setError(null);
    setRuntime((current) =>
      current ? { ...current, phase: "installing", message: "Installing Hermes…" } : current,
    );
    try {
      const installed = await papers.installHermes();
      setRuntime(installed);
      const started = await papers.startHermes();
      setRuntime(started);
      if (started.ws_url) {
        await gateway.connect(started.ws_url);
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      setRuntime(await papers.bootstrapStatus());
    } finally {
      setInstalling(false);
    }
  }, []);

  const start = useCallback(async () => {
    setError(null);
    try {
      const status = await papers.startHermes();
      setRuntime(status);
      if (status.ws_url) {
        await gateway.connect(status.ws_url);
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }, []);

  const send = useCallback(
    async (rawText: string, options: SendOptions = {}) => {
      const text = rawText.trim();
      if (!text || gateway.connectionState !== "open") {
        return;
      }

      setError(null);
      setMessages((current) => [
        ...current,
        { id: newId(), role: "user", text, createdAt: Date.now() },
      ]);

      let localSession = activeSessionRef.current;
      let runtimeSession = hermesSessionRef.current;
      const mode = options.mode ?? "operator";

      if (!localSession || localSession.mode !== mode) {
        localSession = await papers.createSession(
          (options.title ?? text).slice(0, 70),
          mode,
        );
        activeSessionRef.current = localSession;
        setActiveSession(localSession);
        setSessions((current) => [localSession!, ...current]);
        runtimeSession = null;
      }

      if (!runtimeSession) {
        const created = await gateway.request<SessionCreateResponse>(
          "session.create",
          {
            cols: 96,
            source: mode === "builder" ? "papers-builder" : "papers",
            title: (options.title ?? text).slice(0, 70),
            ...(options.cwd ? { cwd: options.cwd } : {}),
            ...(mode === "builder" ? { profile: "papers-builder" } : {}),
          },
          60_000,
        );
        runtimeSession = created.session_id;
        hermesSessionRef.current = runtimeSession;
        setHermesSessionId(runtimeSession);
        const storedSession = created.stored_session_id ?? runtimeSession;
        await papers.bindHermesSession(localSession.id, storedSession);
        localSession = {
          ...localSession,
          hermes_session_id: storedSession,
        };
        activeSessionRef.current = localSession;
        setActiveSession(localSession);
        setSessions((current) =>
          current.map((session) =>
            session.id === localSession!.id ? localSession! : session,
          ),
        );
      }

      let prompt = options.context ? `${options.context}\n\n${text}` : text;
      if (options.selection) {
        prompt = [
          "You are changing Papers itself in its isolated staging worktree.",
          "Use only the Papers builder tools and remain inside the current working directory.",
          "Do not modify protected launcher, permission, credential, installer, MCP, or Hermes lock files.",
          options.changeId ? `Change record: ${options.changeId}` : "",
          `Selected interface element:\n${JSON.stringify(options.selection, null, 2)}`,
          `Requested behavior:\n${text}`,
          "Make the change, run the allowed checks, and explain what the user should experience. Do not commit or push.",
        ]
          .filter(Boolean)
          .join("\n\n");
      }

      await persistState("planning");
      await gateway.request(
        "prompt.submit",
        { session_id: runtimeSession, text: prompt },
        1_800_000,
      );
    },
    [persistState],
  );

  const renameSession = useCallback(
    async (session: PapersSession, title: string) => {
      const updated = await papers.renameSession(session.id, title);
      setSessions((current) =>
        current.map((item) => (item.id === updated.id ? updated : item)),
      );
      if (activeSessionRef.current?.id === updated.id) {
        activeSessionRef.current = updated;
        setActiveSession(updated);
      }
    },
    [],
  );

  const deleteSession = useCallback(
    async (session: PapersSession) => {
      await papers.deleteSession(session.id);
      setSessions((current) => current.filter((item) => item.id !== session.id));
      if (activeSessionRef.current?.id === session.id) {
        setActiveSession(null);
        setHermesSessionId(null);
        activeSessionRef.current = null;
        hermesSessionRef.current = null;
        setMessages([]);
        setActivities([]);
        setWorkItems([]);
        setApproval(null);
        setClarify(null);
        setRunState("idle");
        setError(null);
      }
    },
    [],
  );

  const stop = useCallback(async () => {
    const sid = hermesSessionRef.current;
    if (!sid) {
      return;
    }
    await gateway.request("session.interrupt", { session_id: sid });
    setApproval(null);
    setClarify(null);
    await persistState("cancelled");
  }, [persistState]);

  const pause = useCallback(async () => {
    const sid = hermesSessionRef.current;
    if (!sid) {
      return;
    }
    pausedPrompt.current =
      "Continue the interrupted task from the last verified point. Re-check current state before acting.";
    await gateway.request("session.interrupt", { session_id: sid });
    await persistState("paused");
  }, [persistState]);

  const resume = useCallback(async () => {
    const sid = hermesSessionRef.current;
    if (!sid || !pausedPrompt.current) {
      return;
    }
    const text = pausedPrompt.current;
    pausedPrompt.current = "";
    await persistState("planning");
    await gateway.request(
      "prompt.submit",
      { session_id: sid, text },
      1_800_000,
    );
  }, [persistState]);

  const answerApproval = useCallback(
    async (choice: "once" | "deny") => {
      if (!approval) {
        return;
      }
      await gateway.request("approval.respond", {
        choice,
        session_id: approval.sessionId,
      });
      setApproval(null);
      await persistState("acting");
    },
    [approval, persistState],
  );

  const answerClarify = useCallback(
    async (answer: string) => {
      if (!clarify || !answer.trim()) {
        return;
      }
      await gateway.request("clarify.respond", {
        request_id: clarify.requestId,
        answer: answer.trim(),
      });
      setClarify(null);
      await persistState("acting");
    },
    [clarify, persistState],
  );

  const newConversation = useCallback(() => {
    setActiveSession(null);
    setHermesSessionId(null);
    activeSessionRef.current = null;
    hermesSessionRef.current = null;
    setMessages([]);
    setActivities([]);
    setWorkItems([]);
    setApproval(null);
    setClarify(null);
    setRunState("idle");
    setError(null);
  }, []);

  const openSession = useCallback(
    async (session: PapersSession) => {
      if (!session.hermes_session_id || gateway.connectionState !== "open") {
        return;
      }
      setError(null);
      const resumed = await gateway.request<SessionCreateResponse>(
        "session.resume",
        { session_id: session.hermes_session_id, cols: 96 },
        60_000,
      );
      activeSessionRef.current = session;
      hermesSessionRef.current = resumed.session_id;
      setActiveSession(session);
      setHermesSessionId(resumed.session_id);
      setRunState(session.state);
      setActivities([]);
      setWorkItems([]);
      setMessages(
        (resumed.messages ?? [])
          .filter(
            (message) =>
              message.role === "assistant" || message.role === "user",
          )
          .map((message) => {
            const raw = message.text ?? message.content;
            const text =
              typeof raw === "string"
                ? raw
                : raw == null
                  ? ""
                  : JSON.stringify(raw);
            return {
              id: newId(),
              role: message.role as "assistant" | "user",
              text,
              createdAt: message.timestamp
                ? message.timestamp * 1000
                : Date.now(),
            };
          }),
      );
    },
    [],
  );

  const ready = runtime?.connected && connection === "open";
  const statusLabel = useMemo(() => {
    if (installing) return "Installing Hermes";
    if (!runtime?.installed) return "Agent not installed";
    if (connection === "connecting" || runtime.phase === "starting")
      return "Starting Hermes";
    if (ready) return runState === "idle" ? "Agent ready" : runState.replaceAll("_", " ");
    if (runtime.phase === "error") return "Agent needs attention";
    return "Agent stopped";
  }, [connection, installing, ready, runState, runtime]);

  return {
    runtime,
    connection,
    ready,
    installing,
    statusLabel,
    runState,
    sessions,
    activeSession,
    messages,
    activities,
    workItems,
    approval,
    clarify,
    error,
    install,
    start,
    send,
    stop,
    pause,
    resume,
    answerApproval,
    answerClarify,
    newConversation,
    openSession,
    renameSession,
    deleteSession,
    setError,
  };
}

export type AgentApi = ReturnType<typeof useAgent>;
