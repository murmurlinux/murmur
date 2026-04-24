import { createSignal, onMount, onCleanup, JSX } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { ProGate } from "./ProGate";
import {
  loadCleanupSettings,
  saveCleanupSetting,
  type CleanupProvider,
} from "../lib/settings";

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
  groq: "Very fast (typically <1s). Cheapest option. Adequate for cleanup tasks. Uses Llama 3.3 70B.",
  anthropic: "Higher quality prompt-following. Slightly slower (approx 1-2s). Uses Claude Haiku 4.5.",
};

type TestCleanupResult = {
  success: boolean;
  cleaned: string | null;
  error: string | null;
  duration_ms: number;
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
    } catch (err: unknown) {
      setTestResult({
        success: false,
        cleaned: null,
        error: String(err),
        duration_ms: 0,
      });
    } finally {
      setTesting(false);
    }
  };

  return (
    <ProGate feature="ai-cleanup" title="AI cleanup">
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
          Polishes dictated text with an LLM: fixes punctuation, capitalisation, typos,
          and removes filler words. Your wording is preserved. Runs only when enabled;
          key stored locally on your device.
        </p>

        <label
          style={{
            display: "flex",
            "align-items": "center",
            gap: "12px",
            "margin-bottom": "12px",
            cursor: "pointer",
          }}
        >
          <input
            type="checkbox"
            checked={enabled()}
            onChange={(e) => {
              const v = e.currentTarget.checked;
              setEnabled(v);
              persist("enabled", v);
            }}
          />
          <span style={{ "font-size": "13px", color: "#1a1a1a", "font-family": monoFont }}>
            Enabled
          </span>
        </label>

        <label for="ai-cleanup-provider" style={label}>
          Provider
        </label>
        <select
          id="ai-cleanup-provider"
          value={provider()}
          onChange={(e) => {
            const v = e.currentTarget.value as CleanupProvider;
            setProvider(v);
            persist("provider", v);
          }}
          style={{ ...inputBase, "margin-bottom": "10px" }}
        >
          <option value="groq">Groq</option>
          <option value="anthropic">Anthropic</option>
        </select>
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
          placeholder="Paste your BYOK key"
          style={{ ...inputBase, "margin-bottom": "10px" }}
        />

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
              <span style={{ color: "#c9482b" }}>
                OK: {testResult()!.cleaned} ({testResult()!.duration_ms}ms)
              </span>
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
    </ProGate>
  );
}
