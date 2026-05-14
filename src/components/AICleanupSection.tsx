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

export function AICleanupSection() {
  const [enabled, setEnabled] = createSignal(true);
  const [provider, setProvider] = createSignal<CleanupProvider>("groq");
  const [apiKey, setApiKey] = createSignal("");
  const [testing, setTesting] = createSignal(false);
  const [testResult, setTestResult] = createSignal<TestCleanupResult | null>(null);
  const [lastToast, setLastToast] = createSignal<string | null>(null);

  onMount(async () => {
    const s = await loadCleanupSettings();
    setEnabled(s.enabled);
    setProvider(s.provider);
    setApiKey(s.apiKey);
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

  const persist = <K extends "enabled" | "provider" | "apiKey">(
    key: K,
    value: unknown,
  ) => {
    saveCleanupSetting(key as any, value as any).catch((err) =>
      console.error("cleanup setting save failed:", err),
    );
  };

  const runTest = async () => {
    setTesting(true);
    setTestResult(null);
    try {
      const res = await invoke<TestCleanupResult>("test_cleanup", {
        provider: provider(),
        apiKey: apiKey(),
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
            setProvider(v);
            persist("provider", v);
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
        value={apiKey()}
        onInput={(e) => setApiKey(e.currentTarget.value)}
        onBlur={() => persist("apiKey", apiKey())}
        placeholder="Paste your provider API key"
        style={{ ...inputBase, "margin-bottom": "10px" }}
      />
      <p
        style={{
          "font-size": "11px",
          color: "#a33a2a",
          "margin-bottom": "14px",
          "max-width": "520px",
        }}
      >
        Your key is saved as plain text in a file only your user can read.
        Encrypted storage via your system keyring is coming in the next release.
      </p>

      <button
        disabled={testing() || apiKey().length === 0}
        onClick={runTest}
        style={{
          ...inputBase,
          width: "auto",
          padding: "6px 14px",
          cursor: testing() || apiKey().length === 0 ? "not-allowed" : "pointer",
          background: testing() || apiKey().length === 0 ? "#d4c9b5" : "#f5f0e6",
        }}
      >
        {testing() ? "Testing..." : "Test connection"}
      </button>

      {testResult() !== null && (
        <div style={{ "margin-top": "10px", "font-size": "12px", "font-family": monoFont }}>
          {testResult()!.success ? (
            <div style={{ display: "flex", "flex-direction": "column", gap: "4px" }}>
              {testResult()!.input && (
                <div>
                  <span style={{ color: "#6b655a" }}>Input:&nbsp;</span>
                  <span style={{ color: "#1a1a1a" }}>{testResult()!.input}</span>
                </div>
              )}
              <div>
                <span style={{ color: "#6b655a" }}>Output:&nbsp;</span>
                <span style={{ color: "#c9482b" }}>{testResult()!.cleaned}</span>
              </div>
              <div style={{ color: "#6b655a", "font-size": "11px" }}>
                {testResult()!.duration_ms}ms via {testResult()!.provider}
              </div>
            </div>
          ) : (
            <span style={{ color: "#5a5140" }}>Failed: {testResult()!.error}</span>
          )}
        </div>
      )}

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
