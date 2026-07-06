import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { GatewayEvent, GatewayFrame } from "./types";

type PendingCall = {
  resolve: (result: unknown) => void;
  reject: (error: Error) => void;
  timer: number;
};

export type ConnectionState =
  | "idle"
  | "connecting"
  | "open"
  | "closed"
  | "error";

export class HermesGateway {
  private unlistenFrame: UnlistenFn | null = null;
  private unlistenClosed: UnlistenFn | null = null;
  private nextId = 0;
  private pending = new Map<number, PendingCall>();
  private eventHandlers = new Set<(event: GatewayEvent) => void>();
  private stateHandlers = new Set<(state: ConnectionState) => void>();
  private state: ConnectionState = "idle";

  get connectionState(): ConnectionState {
    return this.state;
  }

  async connect(_url: string): Promise<void> {
    if (this.state === "open") {
      return;
    }

    this.setState("connecting");
    await this.attachListeners();
    try {
      await invoke<void>("gateway_connect");
      this.setState("open");
    } catch (reason) {
      this.setState("error");
      throw reason;
    }
  }

  close(): void {
    void invoke<void>("gateway_disconnect");
    this.rejectPending(new Error("Hermes disconnected"));
    this.setState("closed");
  }

  onEvent(handler: (event: GatewayEvent) => void): () => void {
    this.eventHandlers.add(handler);
    return () => this.eventHandlers.delete(handler);
  }

  onState(handler: (state: ConnectionState) => void): () => void {
    this.stateHandlers.add(handler);
    handler(this.state);
    return () => this.stateHandlers.delete(handler);
  }

  request<T>(
    method: string,
    params: Record<string, unknown> = {},
    timeoutMs = 120_000,
  ): Promise<T> {
    if (this.state !== "open") {
      return Promise.reject(new Error("Hermes is not connected"));
    }

    const id = ++this.nextId;
    return new Promise<T>((resolve, reject) => {
      const timer = window.setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Hermes request timed out: ${method}`));
      }, timeoutMs);

      this.pending.set(id, {
        resolve: (value) => resolve(value as T),
        reject,
        timer,
      });

      void invoke<void>("gateway_send", {
        frame: JSON.stringify({ jsonrpc: "2.0", id, method, params }),
      }).catch((reason: unknown) => {
        const pending = this.pending.get(id);
        if (!pending) return;
        window.clearTimeout(pending.timer);
        this.pending.delete(id);
        pending.reject(reason instanceof Error ? reason : new Error(String(reason)));
      });
    });
  }

  private async attachListeners(): Promise<void> {
    if (!this.unlistenFrame) {
      this.unlistenFrame = await listen<string>(
        "papers://gateway-frame",
        ({ payload }) => this.handleMessage(payload),
      );
    }
    if (!this.unlistenClosed) {
      this.unlistenClosed = await listen<string>(
        "papers://gateway-closed",
        ({ payload }) => {
          this.rejectPending(new Error(payload || "Hermes disconnected"));
          this.setState("closed");
        },
      );
    }
  }

  private handleMessage(raw: string): void {
    let frame: GatewayFrame;
    try {
      frame = JSON.parse(raw) as GatewayFrame;
    } catch {
      return;
    }

    if (frame.id !== undefined && frame.id !== null) {
      const pending = this.pending.get(Number(frame.id));
      if (!pending) {
        return;
      }
      window.clearTimeout(pending.timer);
      this.pending.delete(Number(frame.id));
      if (frame.error) {
        pending.reject(new Error(frame.error.message || "Hermes request failed"));
      } else {
        pending.resolve(frame.result);
      }
      return;
    }

    if (frame.method === "event" && frame.params?.type) {
      for (const handler of this.eventHandlers) {
        handler(frame.params);
      }
    }
  }

  private rejectPending(error: Error): void {
    for (const [id, call] of this.pending) {
      window.clearTimeout(call.timer);
      call.reject(error);
      this.pending.delete(id);
    }
  }

  private setState(state: ConnectionState): void {
    this.state = state;
    for (const handler of this.stateHandlers) {
      handler(state);
    }
  }
}
