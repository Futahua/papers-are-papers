// Isolated credential prompt: a minimal privileged window opened by Rust
// (see open_credential_window in main.rs). The creator pastes an API key here;
// it is posted straight to `save_provider_secret` (Rust → Hermes validate + save)
// and this self-closes on success. The main Papers app's React state never
// receives the key — only the sanitized `key_validated` event.
//
// This file is a separate Vite entry (index-key-entry.html) so it ships its own
// minimal bundle, independent of the main app's state machinery.

import { createRoot } from "react-dom/client";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { AlertTriangle, Check, LoaderCircle, Lock, X } from "lucide-react";

// The credential window's Tauri label is "key-entry/{provider}"; it survives
// Tauri's app-URL handling more reliably than a URL query string. Fall back to
// the query for the dev-server case.
function readProviderId(): string {
  const fromLabel = (() => {
    try {
      const label = (getCurrentWebviewWindow() as unknown as { label: string }).label;
      if (label && label.includes("/")) return label.split("/").pop() ?? "";
    } catch {
      /* not in Tauri (browser dev) */
    }
    return "";
  })();
  if (fromLabel) return fromLabel;
  const params = new URLSearchParams(window.location.search);
  return params.get("provider") ?? "";
}

const providerId = readProviderId();
const windowLabel = `key-entry/${providerId}`;

function KeyEntry() {
  const [secret, setSecret] = useState("");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
  const [error, setError] = useState(null as string | null);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!secret.trim()) return;
    setBusy(true);
    setError(null);
    try {
      //ichtlich post straight to Rust; Rust validates + stores via Hermes.
      const result = await invoke<{ ok: boolean; reachable: boolean; message: string }>(
        "save_provider_secret",
        { providerId, secret },
      );
      if (!result.ok) {
        setError(result.message || "Hermes rejected that key.");
        setBusy(false);
        return;
      }
      setDone(true);
      // Give Rust a moment to emit key_validated to the main window, then close.
      window.setTimeout(() => {
        void getCurrentWebviewWindow()
          .close()
          .catch(() => undefined);
      }, 400);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      setBusy(false);
    }
  };

  return (
    <main className="key-entry-root">
      <header>
        <Lock size={16} />
        <span>Enter {providerId} key</span>
        <button
          className="key-close"
          onClick={() => void getCurrentWebviewWindow().close().catch(() => undefined)}
          aria-label="Cancel"
        >
          <X size={14} />
        </button>
      </header>
      <p>
        Papers sends this key only to Hermes for validation and storage. The key
        is never kept by the main Papers window, never written to disk by Papers,
        and never logged.
      </p>
      {done ? (
        <div className="key-done">
          <Check size={16} /> Saved to Hermes.
        </div>
      ) : (
        <form onSubmit={submit}>
          <input
            type="password"
            value={secret}
            onChange={(event) => setSecret(event.target.value)}
            placeholder={`${providerId} API key`}
            autoFocus
            autoComplete="off"
            spellCheck={false}
          />
          <button className="primary" type="submit" disabled={busy || !secret.trim()}>
            {busy ? <LoaderCircle size={14} className="spin" /> : null}
            {busy ? "Validating…" : "Save & validate"}
          </button>
        </form>
      )}
      {error && (
        <div className="key-error">
          <AlertTriangle size={13} /> {error}
        </div>
      )}
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<KeyEntry />);