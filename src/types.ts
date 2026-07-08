export type RunState =
  | "idle"
  | "planning"
  | "acting"
  | "awaiting_approval"
  | "previewing"
  | "completed"
  | "paused"
  | "cancelled"
  | "failed";

export type RuntimePhase =
  | "not_installed"
  | "installing"
  | "stopped"
  | "starting"
  | "ready"
  | "error";

export interface BootstrapStatus {
  installed: boolean;
  running: boolean;
  connected: boolean;
  phase: RuntimePhase;
  package_version: string;
  release_tag: string;
  hermes_home: string;
  install_dir: string;
  ws_url?: string;
  message: string;
}

export interface AgentProviderStatus {
  provider: string;
  model: string;
  configured_provider: string;
  auth_provider?: string;
  authenticated: boolean;
  runtime_ready: boolean;
  hermes_home: string;
  config_path: string;
  known_providers: string[];
  suggested_models: string[];
  message: string;
}

// --- Provider orchestration layer (new) --------------------------------------
// Mirrors the Rust structs in provider_catalog/provider_state/provider_runtime.
// Kept in lockstep; the wizard renders from `ProviderSetupState`, not step
// numbers + booleans.

export type AuthMethod = "oauth_portal" | "api_key" | "local" | "external";
export type AuthFlow = "device_code" | "pkce" | "loopback" | "manual_cli";
export type SupportLevel = "guided_tested" | "recognized_not_guided" | "future";

export interface ProviderCatalogEntry {
  id: string;
  label: string;
  auth_method: AuthMethod;
  flow?: AuthFlow;
  supports_model_listing: boolean;
  supports_disconnect: boolean;
  supports_live_validation: boolean;
  supports_runtime_test: boolean;
  setup_copy: string;
  docs_hint?: string;
  support_level: SupportLevel;
}

export type ProviderSetupState =
  | { kind: "unconfigured" }
  | {
      kind: "auth_in_progress";
      flow: string;
      session_id: string;
      user_code?: string;
      verification_url?: string;
      expires_at?: string;
    }
  | { kind: "auth_pending_resume"; session_id: string }
  | {
      kind: "awaiting_pkce_code";
      session_id: string;
      expires_at?: string;
    }
  | { kind: "configured_no_model" }
  | { kind: "configured_model_selected" }
  | { kind: "validation_failed"; message: string }
  | { kind: "ready_for_test" }
  | { kind: "runtime_test_passed"; at: string; marker: string }
  | { kind: "runtime_test_failed"; reason: string; at: string }
  | { kind: "external_setup_required"; instructions: string; cli_command: string }
  | { kind: "recognized_not_guided"; hint: string };

export interface ValidationResult {
  ok: boolean;
  reachable: boolean;
  message: string;
  at: string;
}

export interface RuntimeTestResult {
  passed: boolean;
  marker: string;
  reason?: string;
  at: string;
}

export interface ProviderState {
  provider_id: string;
  setup_state: ProviderSetupState;
  configured: boolean;
  authenticated: boolean;
  selected_model?: string;
  can_disconnect: boolean;
  last_validation?: ValidationResult;
  last_runtime_test?: RuntimeTestResult;
  /** True only when Hermes was reachable when this state was computed. */
  verified_by_hermes: boolean;
  message: string;
}

export interface ProviderRuntimeHealth {
  provider_id: string;
  gateway_ok: boolean;
  reachable: boolean;
  can_stream: boolean;
  provider_error?: string;
  model_error?: string;
  auth_error?: string;
  rate_limited: boolean;
  last_tested_at: string;
}

/** Offline weak hint read from Hermes-managed files when Hermes is down.
 *  Always `verified: false` — never a "working" claim. */
export type CredentialHint = "none" | "present";
export type OfflineSource = "auth_json" | "env" | "config";
export interface OfflineProviderHint {
  provider_id: string;
  credential_hint: CredentialHint;
  selected_model?: string;
  source: OfflineSource;
  verified: false;
}

/** Tagged `papers://provider-event` payloads emitted by Rust. */
export type ProviderEvent =
  | {
      kind: "auth_started";
      provider_id: string;
      session_id: string;
      auth_url?: string;
      user_code?: string;
      verification_url?: string;
      needs_code_submit: boolean;
    }
  | { kind: "auth_polling"; provider_id: string; session_id: string }
  | { kind: "auth_approved"; provider_id: string }
  | { kind: "auth_denied"; provider_id: string; message: string }
  | { kind: "auth_expired"; provider_id: string }
  | { kind: "auth_interrupted"; provider_id: string; message: string }
  | {
      kind: "key_validated";
      provider_id: string;
      ok: boolean;
      reachable: boolean;
      message: string;
    }
  | { kind: "model_selected"; provider_id: string; model: string }
  | {
      kind: "runtime_test_passed";
      provider_id: string;
      model: string;
      marker: string;
      at: string;
    }
  | { kind: "runtime_test_failed"; provider_id: string; reason: string; at: string }
  | { kind: "active_provider_changed"; provider_id: string; model: string }
  | { kind: "runtime_health"; health: ProviderRuntimeHealth };

export interface PapersSession {
  id: string;
  hermes_session_id?: string;
  title: string;
  mode: "operator" | "builder";
  state: RunState;
  created_at: string;
  updated_at: string;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "notice";
  text: string;
  createdAt: number;
  pending?: boolean;
}

export interface ActivityItem {
  id: string;
  kind: "status" | "tool" | "result" | "warning" | "error";
  title: string;
  detail?: string;
  state?: "running" | "done" | "failed";
  createdAt: number;
}

export interface WorkItem {
  id: string;
  type:
    | "reasoning_summary"
    | "tool_step"
    | "artifact"
    | "approval"
    | "change"
    | "status";
  title: string;
  detail?: string;
  state?: "running" | "done" | "failed" | "waiting";
  metadata?: Record<string, string | number | boolean>;
  createdAt: number;
}

export interface ApprovalRequest {
  sessionId?: string;
  command: string;
  description: string;
  allowPermanent: boolean;
  effect: string;
  reversibility: "reversible" | "partly_reversible" | "irreversible";
  risk: "low" | "medium" | "high" | "blocked";
}

export interface ClarifyRequest {
  requestId: string;
  sessionId?: string;
  question: string;
  choices: string[];
}

export interface InspectSelection {
  nodeId: string;
  source: string;
  tag: string;
  role: string;
  text: string;
  ariaLabel: string;
  rect: { x: number; y: number; width: number; height: number };
  appearance: {
    color: string;
    background: string;
    font: string;
    fontSize: string;
    border: string;
  };
}

export interface ChangeRecord {
  id: string;
  title: string;
  request: string;
  status:
    | "staging"
    | "building"
    | "preview_ready"
    | "accepted"
    | "rejected"
    | "failed"
    | "conflict";
  branch: string;
  worktree_path: string;
  base_commit: string;
  accepted_commit?: string;
  created_at: string;
  updated_at: string;
}

export interface PolicyDecision {
  action_id: string;
  decision: "allow" | "preview" | "block";
  risk: "low" | "medium" | "high" | "blocked";
  reason: string;
  reversible: boolean;
}

export interface GatewayEvent<P = Record<string, unknown>> {
  type: string;
  session_id?: string;
  payload?: P;
}

export interface GatewayFrame {
  jsonrpc: "2.0";
  id?: number | string | null;
  method?: string;
  params?: GatewayEvent;
  result?: unknown;
  error?: { code?: number; message?: string };
}
