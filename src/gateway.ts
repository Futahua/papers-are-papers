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
  private socket: WebSocket | null = null;
  private nextId = 0;
  private pending = new Map<number, PendingCall>();
  private eventHandlers = new Set<(event: GatewayEvent) => void>();
  private stateHandlers = new Set<(state: ConnectionState) => void>();
  private state: ConnectionState = "idle";

  get connectionState(): ConnectionState {
    return this.state;
  }

  async connect(url: string): Promise<void> {
    if (this.socket?.readyState === WebSocket.OPEN) {
      return;
    }

    this.setState("connecting");
    const socket = new WebSocket(url);
    this.socket = socket;

    socket.addEventListener("message", (message) => {
      if (this.socket === socket) {
        this.handleMessage(String(message.data));
      }
    });

    socket.addEventListener("close", () => {
      if (this.socket !== socket) {
        return;
      }
      this.socket = null;
      this.rejectPending(new Error("Hermes disconnected"));
      this.setState("closed");
    });

    await new Promise<void>((resolve, reject) => {
      const timer = window.setTimeout(() => {
        socket.close();
        reject(new Error("Hermes did not open its control channel in time"));
      }, 15_000);

      socket.addEventListener(
        "open",
        () => {
          window.clearTimeout(timer);
          this.setState("open");
          resolve();
        },
        { once: true },
      );

      socket.addEventListener(
        "error",
        () => {
          window.clearTimeout(timer);
          this.setState("error");
          reject(new Error("Could not connect to Hermes"));
        },
        { once: true },
      );
    });
  }

  close(): void {
    this.socket?.close();
    this.socket = null;
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
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
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

      this.socket?.send(
        JSON.stringify({ jsonrpc: "2.0", id, method, params }),
      );
    });
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
