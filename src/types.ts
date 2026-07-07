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
