import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Check,
  ChevronDown,
  ExternalLink,
  LoaderCircle,
  Lock,
  LogOut,
  RefreshCw,
  ShieldCheck,
  Sparkles,
  X,
} from "lucide-react";
import { papers } from "./papers";
import type {
  ProviderCatalogEntry,
  ProviderEvent,
  ProviderState,
  ProviderSetupState,
} from "./types";
import type { AgentApi } from "./use-agent";

const TEST_PROMPT =
  "Provider health check: reply with exactly `PAPERS_PROVIDER_TEST_OK`, then stop.";
const TEST_MARKER = "PAPERS_PROVIDER_TEST_OK";

const authSupportBadge = (entry: ProviderCatalogEntry): string => {
  if (entry.auth_method === "oauth_portal") return "Browser sign-in";
  if (entry.auth_method === "api_key") return "API key";
  if (entry.auth_method === "local") return "Local daemon";
  return "External CLI";
};

const setupStateHeadline = (state: ProviderSetupState): string => {
  switch (state.kind) {
    case "unconfigured":
      return "Not set up yet";
    case "auth_in_progress":
      return "Waiting for browser sign-in";
    case "awaiting_pkce_code":
      return "Paste the callback code";
    case "configured_no_model":
      return "Signed in — pick a model";
    case "configured_model_selected":
      return "Ready to test";
    case "validation_failed":
      return "Key rejected";
    case "runtime_test_passed":
      return "Tested working";
    case "runtime_test_failed":
      return "Test failed";
    case "external_setup_required":
      return "Set up outside Papers";
    case "recognized_not_guided":
      return "Recognized, not yet guided";
    // ready_for_test collapses into configured_model_selected's headline above
    default:
      return "Ready to test";
  }
};

export function SettingsWizard({
  agent,
  onClose,
  onSetupMessage,
}: {
  agent: AgentApi;
  onClose: () => void;
  onSetupMessage: (message: string) => void;
}) {
  const [catalog, setCatalog] = useState<ProviderCatalogEntry[]>([]);
  const [providerId, setProviderId] = useState("nous" as string);
  const [state, setState] = useState<ProviderState | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [modelDraft, setModelDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [pkceCode, setPkceCode] = useState("");
  const [onlineHint, setOnlineHint] = useState("");
  const [testing, setTesting] = useState(false);
  const pollRef = useRef<(() => void) | null>(null);

  const entry = useMemo(
    () => catalog.find((item) => item.id === providerId) ?? null,
    [catalog, providerId],
  );

  const refresh = useCallback(async () => {
    setBusy(true);
    try {
      const [next, listed] = await Promise.all([
        papers.getProviderState(providerId),
        papers.listProviderModels(providerId).catch(() => [] as string[]),
      ]);
      setState(next);
      setModels(listed);
      setModelDraft(next.selected_model || listed[0] || "");
      onSetupMessage(next.message);
    } finally {
      setBusy(false);
    }
  }, [providerId, onSetupMessage]);

  useEffect(() => {
    void papers
      .listProviders()
      .then(setCatalog)
      .then(refresh)
      .catch((reason: unknown) =>
        agent.setError(reason instanceof Error ? reason.message : String(reason)),
      );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Listen for provider events so the wizard reacts to backend-driven state
  // (sign-in approved/denied, key validated, runtime test outcome, active
  // provider changed). The Work rail / companion can reuse the same channel.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<ProviderEvent>("papers://provider-event", (event) => {
      const payload = event.payload;
      if (!payload) return;
      const mine = "provider_id" in payload && payload.provider_id === providerId;
      if (!mine) return;
      if (payload.kind === "auth_approved" || payload.kind === "key_validated" || payload.kind === "model_selected" || payload.kind === "active_provider_changed") {
        void refresh();
      }
      if (payload.kind === "auth_expired" || payload.kind === "auth_denied" || payload.kind === "auth_interrupted") {
        const message =
          payload.kind === "auth_interrupted"
            ? payload.message
            : payload.kind === "auth_expired"
              ? "Previous sign-in expired. Start it again."
              : payload.message;
        agent.setError(message);
        void refresh();
      }
    }).then((stop) => {
      unlisten = stop;
    });
    return () => unlisten?.();
  }, [agent, providerId, refresh]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // Stop any polling loop on unmount.
  useEffect(() => () => pollRef.current?.(), []);

  const startPoll = useCallback(
    (sessionId: string) => {
      pollRef.current?.();
      let alive = true;
      const tick = async () => {
        if (!alive) return;
        try {
          const event = await papers.pollProviderAuth(providerId, sessionId);
          if (event.kind === "auth_approved") {
            alive = false;
            pollRef.current = null;
            await refresh();
            return;
          }
          if (event.kind === "auth_denied" || event.kind === "auth_expired" || event.kind === "auth_interrupted") {
            alive = false;
            pollRef.current = null;
            const message =
              event.kind === "auth_interrupted"
                ? event.message
                : event.kind === "auth_expired"
                  ? "Previous sign-in expired. Start it again."
                  : event.message;
            agent.setError(message);
            await refresh();
            return;
          }
          if (event.kind === "auth_polling") {
            setTimeout(tick, 2000);
          }
        } catch (reason) {
          alive = false;
          pollRef.current = null;
          agent.setError(reason instanceof Error ? reason.message : String(reason));
        }
      };
      pollRef.current = () => {
        alive = false;
      };
      setTimeout(tick, 2000);
    },
    [agent, providerId, refresh],
  );

  const beginAuth = useCallback(async () => {
    if (!entry) return;
    setBusy(true);
    try {
      const event = await papers.beginProviderAuth(providerId);
      if (event.kind === "auth_started") {
        if (event.needs_code_submit) {
          setOnlineHint(
            `Open the sign-in page, authorize Papers, then paste the code that appears in the redirect URL.`,
          );
        }
        if (event.user_code || event.verification_url) {
          setOnlineHint(
            `Code: ${event.user_code ?? ""}${event.verification_url ? ` — open ${event.verification_url}` : ""}`,
          );
        }
        startPoll(event.session_id);
      }
      await refresh();
    } catch (reason) {
      agent.setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }, [agent, entry, providerId, refresh, startPoll]);

  const submitCode = useCallback(async () => {
    if (state?.setup_state.kind !== "awaiting_pkce_code") return;
    setBusy(true);
    try {
      await papers.submitProviderAuthCode(
        providerId,
        state.setup_state.session_id,
        pkceCode.trim(),
      );
      setPkceCode("");
      await refresh();
    } catch (reason) {
      agent.setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }, [agent, providerId, pkceCode, refresh, state]);

  const enterApiKey = useCallback(async () => {
    // Opens the isolated credential prompt window owned by Rust. The key is
    // typed there and posted straight back to Rust — it never enters this
    // wizard's React state. This window just reacts to the `key_validated`
    // event above.
    try {
      await papers.openApiKeyWindow(providerId);
      setOnlineHint("Enter the key in the small Papers window that opened.");
    } catch (reason) {
      agent.setError(reason instanceof Error ? reason.message : String(reason));
    }
  }, [agent, providerId]);

  const saveModel = useCallback(async () => {
    if (!modelDraft.trim()) return;
    setBusy(true);
    try {
      await papers.setProviderModelV2(providerId, modelDraft.trim());
      await refresh();
    } catch (reason) {
      agent.setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }, [agent, modelDraft, providerId, refresh]);

  const runLiveTest = useCallback(async () => {
    if (!agent.ready) {
      agent.setError("Start Hermes before running a live model test.");
      return;
    }
    setTesting(true);
    try {
      const messagesBefore = agent.messages.length;
      await agent.send(TEST_PROMPT, { title: "Provider test" });
      // The turn streams into agent.messages; settle on the assistant reply.
      const echo = await new Promise<string|null>((resolve) => {
        const interval = window.setInterval(() => {
          const after = agent.messages.slice(messagesBefore);
          const last = after[after.length - 1];
          if (last && last.role === "assistant" && !last.pending) {
            window.clearInterval(interval);
            resolve(last.text);
          }
        }, 500);
        window.setTimeout(() => {
          window.clearInterval(interval);
          resolve(null);
        }, 60_000);
      });
      const passed = echo?.includes(TEST_MARKER) ?? false;
      const result = await papers.recordProviderTestResult(
        providerId,
        modelDraft.trim() || state?.selected_model || "",
        passed ? echo : null,
        passed ? null : echo ?? "The provider did not echo back the test marker.",
      );
      if (result.passed) {
        await papers.setActiveProvider(providerId, modelDraft.trim(), false).catch(
          (reason: unknown) =>
            agent.setError(reason instanceof Error ? reason.message : String(reason)),
        );
      }
      await refresh();
    } finally {
      setTesting(false);
    }
  }, [agent, modelDraft, providerId, refresh, state?.selected_model]);

  const activate = useCallback(async () => {
    setBusy(true);
    try {
      await papers.setActiveProvider(providerId, modelDraft.trim() || state?.selected_model || "", false);
      await refresh();
    } catch (reason) {
      agent.setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }, [agent, modelDraft, providerId, refresh, state?.selected_model]);

  const disconnect = useCallback(async () => {
    if (!window.confirm(`Disconnect ${providerId} and clear its saved credentials?`)) return;
    setBusy(true);
    try {
      await papers.disconnectProvider(providerId);
      await refresh();
    } catch (reason) {
      agent.setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }, [agent, providerId, refresh]);

  const closeWindow = useCallback(async () => {
    try {
      await getCurrentWebviewWindow().close();
    } catch {
      onClose();
    }
  }, [onClose]);

  const guided = entry ? entry.support_level === "guided_tested" : false;
  const renderKeyEntry = entry?.auth_method === "api_key";
  const isPkce = state?.setup_state.kind === "awaiting_pkce_code";
  const canActivate =
    state?.setup_state.kind === "runtime_test_passed" ||
    state?.setup_state.kind === "runtime_test_failed";

  return (
    <section className="settings-panel" role="dialog" aria-modal="true">
      <div className="settings-header">
        <div>
          <p className="eyebrow">Settings</p>
          <h2>Agent provider</h2>
        </div>
        <button onClick={() => void closeWindow()} aria-label="Close settings">
          <X size={16} />
        </button>
      </div>
      <p className="settings-copy">
        Papers changes Hermes&apos; private config only. Credentials stay with
        Hermes — this screen never shows API keys or OAuth tokens.
      </p>

      <div className="provider-picker">
        <label className="provider-select">
          <span>Provider</span>
          <div className="select-wrap">
            <select
              value={providerId}
              onChange={(event) => {
                setProviderId(event.target.value);
                setOnlineHint("");
              }}
            >
              {catalog.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.label}
                  {item.support_level !== "guided_tested" ? " — not yet guided" : ""}
                </option>
              ))}
            </select>
            <ChevronDown size={14} />
          </div>
        </label>
        <button
          className="icon-action"
          onClick={() => void refresh()}
          disabled={busy}
          title="Refresh provider state"
        >
          <RefreshCw size={14} className={busy ? "spin" : ""} />
        </button>
      </div>

      {entry && (
        <div className="provider-badges">
          <span className="pill">{authSupportBadge(entry)}</span>
          {entry.support_level === "guided_tested" && (
            <span className="pill good">Guided</span>
          )}
          {entry.support_level === "recognized_not_guided" && (
            <span className="pill warn">Recognized</span>
          )}
          {entry.docs_hint && (
            <a
              className="pill link"
              href={entry.docs_hint}
              target="_blank"
              rel="noreferrer"
            >
              Learn more <ExternalLink size={11} />
            </a>
          )}
        </div>
      )}

      <p className="provider-copy">{entry?.setup_copy}</p>

      {state && (
        <div
          className={`provider-status-card ${
            state.verified_by_hermes ? "" : "unverified"
          }`}
        >
          <strong>{entry?.label ?? providerId}</strong>
          <small>
            {setupStateHeadline(state.setup_state)}
            {!state.verified_by_hermes && " · not verified (Hermes stopped)"}
          </small>
          {onlineHint && <small className="hint">{onlineHint}</small>}
          <div>
            <span>
              Auth:&nbsp;
              {state.authenticated
                ? "verified"
                : state.configured
                  ? "saved"
                  : "not set up"}
            </span>
            <span>Hermes: {agent.ready ? "ready" : "not ready"}</span>
            {state.selected_model && <span>Model: {state.selected_model}</span>}
          </div>
        </div>
      )}

      {/* Auth section renders from state, not from a step number. */}
      {state && (
        <div className="wizard-actions">
          {!guided && (
            <p className="settings-footnote warn">
              Papers recognizes {providerId} but has not yet guided its setup.
              Set it up in Hermes for now; this flow will be exercised in a later pass.
            </p>
          )}

          {state.setup_state.kind === "external_setup_required" && (
            <div className="external-cli">
              <p>This provider is set up by an external CLI. Run it yourself:</p>
              <pre>{state.setup_state.cli_command}</pre>
            </div>
          )}

          {guided && state.setup_state.kind === "unconfigured" && (
            <>
              {entry?.auth_method === "oauth_portal" && (
                <button
                  className="primary"
                  onClick={() => void beginAuth()}
                  disabled={busy || !agent.ready}
                  title={agent.ready ? "Open browser sign-in" : "Start Hermes first"}
                >
                  <Sparkles size={14} /> Start {entry?.label} sign-in
                </button>
              )}
              {renderKeyEntry && (
                <button
                  className="primary"
                  onClick={() => void enterApiKey()}
                  disabled={busy}
                  title="Open the isolated key entry window"
                >
                  <Lock size={14} /> Enter API key
                </button>
              )}
            </>
          )}

          {state.setup_state.kind === "validation_failed" && (
            <p className="inline-error">{state.setup_state.message}</p>
          )}

          {isPkce && (
            <form
              onSubmit={(event) => {
                event.preventDefault();
                void submitCode();
              }}
            >
              <label className="code-input">
                <span>Paste the callback code</span>
                <input
                  value={pkceCode}
                  onChange={(event) => setPkceCode(event.target.value)}
                  placeholder="code from the redirect URL"
                />
              </label>
              <button className="primary" type="submit" disabled={busy || !pkceCode.trim()}>
                Submit code
              </button>
            </form>
          )}

          {(state.setup_state.kind === "configured_no_model" ||
            state.setup_state.kind === "configured_model_selected" ||
            state.setup_state.kind === "runtime_test_passed" ||
            state.setup_state.kind === "runtime_test_failed") && (
            <div className="model-section">
              <label className="model-input">
                <span>Model</span>
                <input
                  list="papers-model-suggestions"
                  value={modelDraft}
                  onChange={(event) => setModelDraft(event.target.value)}
                  placeholder="provider/model or model id"
                />
                <datalist id="papers-model-suggestions">
                  {models.map((model) => (
                    <option value={model} key={model} />
                  ))}
                </datalist>
              </label>
              <button
                className="secondary"
                onClick={() => void saveModel()}
                disabled={busy || !modelDraft.trim() || modelDraft === state.selected_model}
              >
                Save model
              </button>
            </div>
          )}

          {state.setup_state.kind === "configured_model_selected" && (
            <button
              className="primary"
              onClick={() => void runLiveTest()}
              disabled={busy || testing || !agent.ready}
              title={agent.ready ? "Send a tiny real turn" : "Start Hermes first"}
            >
              {testing ? <LoaderCircle size={14} className="spin" /> : <ShieldCheck size={14} />}
              {testing ? "Testing…" : "Run live test"}
            </button>
          )}

          {state.setup_state.kind === "runtime_test_passed" && (
            <div className="test-ok">
              <Check size={14} /> Provider tested working. Activate it as your agent provider:
            </div>
          )}
          {state.setup_state.kind === "runtime_test_failed" && (
            <p className="inline-error">
              Test failed: {state.setup_state.reason}. Try a different model or re-enter the key.
            </p>
          )}

          {canActivate && (
            <button
              className="primary"
              onClick={() => void activate()}
              disabled={busy}
            >
              Activate {providerId}
            </button>
          )}

          {state.can_disconnect && state.setup_state.kind !== "unconfigured" && (
            <button
              className="danger-link"
              onClick={() => void disconnect()}
              disabled={busy}
            >
              <LogOut size={13} /> Disconnect {providerId}
            </button>
          )}
        </div>
      )}

      <p className="settings-footnote">
        Honest badges: saved means a credential is present; verified (authenticated)
        is confirmed by Hermes only; tested working means a real turn echoed the
        marker. No active provider is selected until a live test passes.
      </p>
    </section>
  );
}