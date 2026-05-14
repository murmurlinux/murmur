import { createSignal, onMount, onCleanup, JSX } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  loadCleanupSettings,
  saveCleanupSetting,
  type CleanupProvider,
} from "../lib/settings";
import { Toggle } from "./Toggle";
import { Select } from "./Select";

const monoFont = "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace";

const glass: JSX.CSSProperties = {
  "margin-bottom": "16px",
  padding: "18px",
  background: "#ece4d0",
  "border-radius": "0",
  border: "1px solid #d4c9b5",
  position: "relative",
};

const label: JSX.CSSProperties = {
  display: "block",
  "font-size": "11px",
  "font-weight": "700",
  "text-transform": "uppercase",
  "letter-spacing": "0.08em",
  color: "#c9482b",
  "margin-bottom": "10px",
};

const inputBase: JSX.CSSProperties = {
  width: "100%",
  padding: "8px 12px",
  background: "#f5f0e6",
  border: "1px solid #1a1a1a",
  "border-radius": "0",
  color: "#1a1a1a",
  "font-size": "13px",
  "font-family": monoFont,
  "box-sizing": "border-box",
  outline: "none",
};

const PROVIDER_DESCRIPTIONS: Record<CleanupProvider, string> = {
  groq: "Fast and cheap, usually under a second. Uses Llama 3.3 70B on Groq Inc.'s LPU hardware (easy to mix up with xAI's Grok).",
  anthropic: "More careful, a touch slower. Around one to two seconds. Uses Claude Haiku 4.5.",
  xai: "Grok, the fast version. Trained on more recent data than the others. Uses Grok 4 Fast (xAI, not Groq Inc.).",
};

type TestCleanupResult = {
  success: boolean;
  input: string;
  cleaned: string | null;
  error: string | null;
  duration_ms: number;
  provider: string;
};

type CleanupStatusPayload = {
  status: "success" | "raw_fallback";
  reason?: string;
  duration_ms: number;
};

type SttFallbackPayload = {
  primary: string;
  fallback: string;
  reason: string;
};

type StorageMode = "keyring" | "plaintext";

export function AICleanupSection() {
  const [enabled, setEnabled] = createSignal(true);
  const [provider, setProvider] = createSignal<CleanupProvider>("groq");
  // The cleartext key never lives in a signal. We mirror just what we
  // need to render the input: whether a key is set, a masked hint, and
  // a transient buffer for the user's current keystrokes that is wiped
  // immediately after the explicit Save action persists the key.
  const [pendingKey, setPendingKey] = createSignal("");
  const [hasKey, setHasKey] = createSignal(false);
  const [keyHint, setKeyHint] = createSignal<string | null>(null);
  const [storageMode, setStorageMode] = createSignal<StorageMode>("plaintext");
  const [saving, setSaving] = createSignal(false);
  const [justSaved, setJustSaved] = createSignal(false);
  const [testing, setTesting] = createSignal(false);
  const [testResult, setTestResult] = createSignal<TestCleanupResult | null>(null);
  const [lastToast, setLastToast] = createSignal<string | null>(null);
  let justSavedTimeout: ReturnType<typeof setTimeout> | null = null;

  const refreshKeyState = async (p: CleanupProvider) => {
    try {
      const [has, hint] = await Promise.all([
        invoke<boolean>("byok_has_key", { provider: p }),
        invoke<string | null>("byok_key_hint", { provider: p }),
      ]);
      setHasKey(has);
      setKeyHint(hint);
    } catch (err) {
      console.error("byok state read failed:", err);
      setHasKey(false);
      setKeyHint(null);
    }
  };

  onMount(async () => {
    const s = await loadCleanupSettings();
    setEnabled(s.enabled);
    setProvider(s.provider);
    try {
      const mode = await invoke<StorageMode>("byok_storage_mode");
      setStorageMode(mode);
    } catch (err) {
      console.error("byok mode read failed:", err);
    }
    await refreshKeyState(s.provider);
  });

  onMount(() => {
    let unlistenCleanup: UnlistenFn | null = null;
    let unlistenFallback: UnlistenFn | null = null;

    (async () => {
      unlistenCleanup = await listen<CleanupStatusPayload>("cleanup-status", (e) => {
        if (e.payload.status === "raw_fallback") {
          setLastToast(
            `Cleanup unavailable: pasted raw (${e.payload.reason ?? "unknown"})`,
          );
        } else if (e.payload.status === "success") {
          // Clear any stale "unavailable" toast once cleanup recovers.
          setLastToast(null);
        }
      });
      unlistenFallback = await listen<SttFallbackPayload>("stt-fallback", (e) => {
        setLastToast(`Cloud STT unavailable: used local Whisper (${e.payload.reason})`);
      });
    })();

    onCleanup(() => {
      unlistenCleanup?.();
      unlistenFallback?.();
    });
  });

  const persist = <K extends "enabled" | "provider">(
    key: K,
    value: unknown,
  ) => {
    saveCleanupSetting(key as any, value as any).catch((err) =>
      console.error("cleanup setting save failed:", err),
    );
  };

  const saveKey = async () => {
    const value = pendingKey().trim();
    if (value.length === 0 || saving()) return;
    setSaving(true);
    setTestResult(null);
    try {
      await invoke("byok_set_key", { provider: provider(), key: value });
      setPendingKey("");
      await refreshKeyState(provider());
      // A fresh key invalidates the previous "unavailable" toast.
      setLastToast(null);
      // Transient confirmation next to the button row.
      if (justSavedTimeout !== null) clearTimeout(justSavedTimeout);
      setJustSaved(true);
      justSavedTimeout = setTimeout(() => setJustSaved(false), 2500);
    } catch (err) {
      console.error("byok_set_key failed:", err);
      setLastToast(`Could not save key: ${err}`);
    } finally {
      setSaving(false);
    }
  };

  const clearKey = async () => {
    try {
      await invoke("byok_clear_key", { provider: provider() });
      setPendingKey("");
      setJustSaved(false);
      setTestResult(null);
      await refreshKeyState(provider());
    } catch (err) {
      console.error("byok_clear_key failed:", err);
      setLastToast(`Could not clear key: ${err}`);
    }
  };

  onCleanup(() => {
    if (justSavedTimeout !== null) clearTimeout(justSavedTimeout);
  });

  const onProviderChange = async (v: CleanupProvider) => {
    setProvider(v);
    persist("provider", v);
    setPendingKey("");
    await refreshKeyState(v);
  };

  const runTest = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      const res = await invoke<TestCleanupResult>("test_cleanup", {
        provider: provider(),
      });
      setTestResult(res);
      if (res.success) {
        // Test passed -- clear any stale "unavailable" toast.
        setLastToast(null);
      }
    } catch (err: unknown) {
      setTestResult({
        success: false,
        input: "",
        cleaned: null,
        error: String(err),
        duration_ms: 0,
        provider: provider(),
      });
    } finally {
      setTesting(false);
    }
  };

  return (
    <div style={glass}>
      <div style={{ ...label, "margin-bottom": "14px" }}>AI cleanup</div>
      <p
        style={{
          "font-size": "12px",
          color: "#5a5140",
          "margin-bottom": "14px",
          "max-width": "520px",
        }}
      >
        Tidies up your dictation. Removes the "um" and "ah", fixes punctuation,
        catches typos. Your wording stays yours. We never see your key.
      </p>

      <div
        style={{
          display: "flex",
          "align-items": "center",
          "justify-content": "space-between",
          "margin-bottom": "14px",
        }}
      >
        <span style={{ "font-size": "13px", color: "#6b655a" }}>Enabled</span>
        <Toggle
          value={enabled()}
          onChange={() => {
            const v = !enabled();
            setEnabled(v);
            persist("enabled", v);
          }}
        />
      </div>

      <label for="ai-cleanup-provider" style={label}>
        Provider
      </label>
      <div style={{ "margin-bottom": "10px" }}>
        <Select<CleanupProvider>
          value={provider()}
          onChange={(v) => {
            void onProviderChange(v);
          }}
          options={[
            { value: "groq", label: "Groq" },
            { value: "anthropic", label: "Anthropic" },
            { value: "xai", label: "xAI (Grok)" },
          ]}
        />
      </div>
      <p style={{ "font-size": "11px", color: "#5a5140", "margin-bottom": "14px" }}>
        {PROVIDER_DESCRIPTIONS[provider()]}
      </p>

      <label for="ai-cleanup-api-key" style={label}>
        API key
      </label>
      <input
        id="ai-cleanup-api-key"
        type="password"
        value={pendingKey()}
        onInput={(e) => {
          setPendingKey(e.currentTarget.value);
          // Once the user starts entering a fresh key, prior feedback
          // is stale — clear it so the slot is ready for the next action.
          if (justSaved()) setJustSaved(false);
          if (testResult() !== null) setTestResult(null);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            void saveKey();
          }
        }}
        placeholder="Paste your provider API key"
        style={{ ...inputBase, "margin-bottom": "6px" }}
      />
      {hasKey() && (
        <p
          style={{
            "font-size": "11px",
            color: "#6b655a",
            "font-family": monoFont,
            margin: "0 0 10px 0",
          }}
        >
          Saved: {keyHint() ?? "****"}
        </p>
      )}

      <div
        style={{
          display: "flex",
          "align-items": "center",
          gap: "8px",
          "margin-bottom": "12px",
          "flex-wrap": "wrap",
        }}
      >
        <button
          type="button"
          disabled={saving() || pendingKey().trim().length === 0}
          onClick={() => {
            void saveKey();
          }}
          style={{
            ...inputBase,
            width: "auto",
            padding: "6px 14px",
            cursor: saving() || pendingKey().trim().length === 0 ? "not-allowed" : "pointer",
            background:
              saving() || pendingKey().trim().length === 0 ? "#d4c9b5" : "#f5f0e6",
            "margin-bottom": "0",
          }}
        >
          {saving() ? "Saving..." : "Save"}
        </button>
        <button
          type="button"
          disabled={!hasKey()}
          onClick={() => {
            void clearKey();
          }}
          style={{
            ...inputBase,
            width: "auto",
            padding: "6px 14px",
            cursor: !hasKey() ? "not-allowed" : "pointer",
            background: !hasKey() ? "#d4c9b5" : "#f5f0e6",
            "margin-bottom": "0",
          }}
        >
          Clear
        </button>
        <button
          type="button"
          disabled={testing() || !hasKey()}
          onClick={runTest}
          style={{
            ...inputBase,
            width: "auto",
            padding: "6px 14px",
            cursor: testing() || !hasKey() ? "not-allowed" : "pointer",
            background: testing() || !hasKey() ? "#d4c9b5" : "#f5f0e6",
            "margin-bottom": "0",
          }}
        >
          {testing() ? "Testing..." : "Test connection"}
        </button>

        {justSaved() && !testResult() && (
          <span style={{ "font-size": "12px", color: "#5a7a3a", "font-family": monoFont }}>
            ✓ Saved
          </span>
        )}
        {testResult() !== null && (
          <span
            style={{
              "font-size": "12px",
              color: testResult()!.success ? "#5a7a3a" : "#a33a2a",
              "font-family": monoFont,
            }}
          >
            {testResult()!.success
              ? `✓ Key works (${testResult()!.duration_ms}ms via ${testResult()!.provider})`
              : `✗ ${testResult()!.error ?? "failed"}`}
          </span>
        )}
      </div>

      <p
        style={{
          "font-size": "11px",
          color: storageMode() === "keyring" ? "#5a5140" : "#a33a2a",
          "margin-bottom": "0",
          "max-width": "520px",
        }}
      >
        {storageMode() === "keyring"
          ? "Your key is encrypted by your system keyring."
          : "No system keyring detected. Your key is saved as plain text in ~/.config/murmur/settings.json. Install gnome-keyring or kwallet (with the secret-service plug-in) to encrypt it."}
      </p>

      {lastToast() !== null && (
        <div
          style={{
            "margin-top": "8px",
            "font-size": "11px",
            color: "#5a5140",
            "font-family": monoFont,
          }}
        >
          {lastToast()}
        </div>
      )}
    </div>
  );
}
