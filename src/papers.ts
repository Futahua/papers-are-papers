import { invoke } from "@tauri-apps/api/core";
import type {
  AgentProviderStatus,
  BootstrapStatus,
  ChangeRecord,
  GatewayEvent,
  InspectSelection,
  PapersSession,
  PolicyDecision,
  RunState,
} from "./types";

export const papers = {
  bootstrapStatus: () => invoke<BootstrapStatus>("bootstrap_status"),
  installHermes: () => invoke<BootstrapStatus>("install_hermes"),
  startHermes: () => invoke<BootstrapStatus>("start_hermes"),
  stopHermes: () => invoke<void>("stop_hermes"),
  startNousLogin: () => invoke<string>("start_nous_login"),
  agentProviderStatus: () =>
    invoke<AgentProviderStatus>("agent_provider_status"),
  setAgentProvider: (provider: string, model: string) =>
    invoke<AgentProviderStatus>("set_agent_provider", { provider, model }),
  startProviderLogin: (provider: string) =>
    invoke<string>("start_provider_login", { provider }),
  validateAgentProvider: () =>
    invoke<AgentProviderStatus>("validate_agent_provider"),
  showCompanion: () => invoke<void>("show_companion"),
  hideCompanion: () => invoke<void>("hide_companion"),
  showMain: () => invoke<void>("show_main"),
  foregroundApp: () => invoke<string>("foreground_app"),
  listSessions: () => invoke<PapersSession[]>("list_sessions"),
  createSession: (title: string, mode: "operator" | "builder") =>
    invoke<PapersSession>("create_session", { title, mode }),
  renameSession: (id: string, title: string) =>
    invoke<PapersSession>("rename_session", { id, title }),
  deleteSession: (id: string) => invoke<void>("delete_session", { id }),
  bindHermesSession: (id: string, hermesSessionId: string) =>
    invoke<void>("bind_hermes_session", { id, hermesSessionId }),
  updateSessionState: (id: string, state: RunState) =>
    invoke<void>("update_session_state", { id, stateName: state }),
  recordEvent: (sessionId: string, event: GatewayEvent) =>
    invoke<void>("record_agent_event", { sessionId, event }),
  classifyAction: (
    kind: string,
    target: string,
    payload: string,
  ) =>
    invoke<PolicyDecision>("classify_action", { kind, target, payload }),
  createChange: (title: string, request: string, selection?: InspectSelection) =>
    invoke<ChangeRecord>("create_change", { title, request, selection }),
  listChanges: () => invoke<ChangeRecord[]>("list_changes"),
  buildChange: (id: string) => invoke<ChangeRecord>("build_change", { id }),
  launchChangePreview: (id: string) =>
    invoke<ChangeRecord>("launch_change_preview", { id }),
  acceptChange: (id: string) => invoke<ChangeRecord>("accept_change", { id }),
  rejectChange: (id: string) => invoke<void>("reject_change", { id }),
  rollbackLast: () => invoke<string>("rollback_last"),
};
