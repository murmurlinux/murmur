import { createSignal, onMount, onCleanup, JSX, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  loadCleanupSettings,
  saveCleanupSetting,
  type CleanupProvider,
} from "../lib/settings";
import { Toggle } from "./Toggle";
import { Select } from "./Select";
import { Icon } from "./Icon";
import { AnimatedCheck, AnimatedCross } from "./AnimatedCheck";

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

const ICON_BTN_SIZE = "36px";

// Square icon button: same height as a standard input, sits flush
// against it. Primary variant (Save) uses the brand terracotta as
// background; secondary variant (used elsewhere for trash etc.) is
// the standard cream-with-dark-border treatment.
const iconBtnBase: JSX.CSSProperties = {
  width: ICON_BTN_SIZE,
  height: ICON_BTN_SIZE,
  display: "inline-flex",
  "align-items": "center",
  "justify-content": "center",
  border: "1px solid #1a1a1a",
  "border-radius": "0",
  padding: "0",
  "flex-shrink": "0",
  cursor: "pointer",
};

const PROVIDER_LABEL: Record<CleanupProvider, string> = {
  groq: "Groq",
  anthropic: "Anthropic",
  xai: "xAI",
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

interface AICleanupSectionProps {
  // Called whenever a stored key for any provider is added or removed,
  // so the parent can bump SavedKeysSection's refresh trigger.
  onKeyMutated?: () => void;
}

export function AICleanupSection(props: AICleanupSectionProps): JSX.Element {
  const [enabled, setEnabled] = createSignal(true);
  const [provider, setProvider] = createSignal<CleanupProvider>("groq");
  // The cleartext key never lives in a signal. We mirror just what we
  // need to render the input: whether a key is set, a masked hint, and
  // a transient buffer for the user's current keystrokes that is wiped
  // immediately after the explicit Save action persists the key.
  const [pendingKey, setPendingKey] = createSignal("");
  const [hasKey, setHasKey] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [justSaved, setJustSaved] = createSignal(false);
  const [testing, setTesting] = createSignal(false);
  const [testResult, setTestResult] = createSignal<TestCleanupResult | null>(null);
  const [lastToast, setLastToast] = createSignal<string | null>(null);
  let justSavedTimeout: ReturnType<typeof setTimeout> | null = null;

  const refreshHasKey = async (p: CleanupProvider) => {
    try {
      const has = await invoke<boolean>("byok_has_key", { provider: p });
      setHasKey(has);
    } catch (err) {
      console.error("byok_has_key read failed:", err);
      setHasKey(false);
    }
  };

  onMount(async () => {
    const s = await loadCleanupSettings();
    setEnabled(s.enabled);
    setProvider(s.provider);
    await refreshHasKey(s.provider);
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

  onCleanup(() => {
    if (justSavedTimeout !== null) clearTimeout(justSavedTimeout);
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
      await refreshHasKey(provider());
      setLastToast(null);
      if (justSavedTimeout !== null) clearTimeout(justSavedTimeout);
      setJustSaved(true);
      justSavedTimeout = setTimeout(() => setJustSaved(false), 2000);
      props.onKeyMutated?.();
    } catch (err) {
      console.error("byok_set_key failed:", err);
      setLastToast(`Could not save key: ${err}`);
    } finally {
      setSaving(false);
    }
  };

  const onProviderChange = async (v: CleanupProvider) => {
    setProvider(v);
    persist("provider", v);
    setPendingKey("");
    setTestResult(null);
    setJustSaved(false);
    await refreshHasKey(v);
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

  // Re-fetch hasKey when a sibling component (SavedKeysSection) clears
  // a key for the active provider. The parent's onKeyMutated callback
  // can't reach back into our state directly, so we listen to a
  // synthetic event the parent dispatches.
  onMount(() => {
    const handler = () => {
      void refreshHasKey(provider());
      setTestResult(null);
    };
    window.addEventListener("byok-keys-changed", handler);
    onCleanup(() => window.removeEventListener("byok-keys-changed", handler));
  });

  const saveDisabled = () => saving() || pendingKey().trim().length === 0;
  const testDisabled = () => testing() || !hasKey();

  const testButtonLabel = () => {
    if (testing()) return "Testing...";
    if (!hasKey()) return "Save a key to test";
    return `Test ${PROVIDER_LABEL[provider()]}`;
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

      {/* Row 1: input + Save icon button */}
      <div style={{ display: "flex", gap: "8px", "margin-bottom": "8px" }}>
        <input
          id="ai-cleanup-api-key"
          type="password"
          value={pendingKey()}
          onInput={(e) => {
            setPendingKey(e.currentTarget.value);
            if (justSaved()) setJustSaved(false);
            if (testResult() !== null) setTestResult(null);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              void saveKey();
            }
          }}
          placeholder={`Paste your ${PROVIDER_LABEL[provider()]} API key`}
          style={{ ...inputBase, flex: "1 1 auto", "margin-bottom": "0", height: ICON_BTN_SIZE }}
        />
        <button
          type="button"
          aria-label="Save API key"
          title="Save API key"
          disabled={saveDisabled() && !justSaved()}
          onClick={() => {
            void saveKey();
          }}
          style={{
            ...iconBtnBase,
            background: saveDisabled() && !justSaved() ? "#d4c9b5" : "#c9482b",
            cursor: saveDisabled() && !justSaved() ? "not-allowed" : "pointer",
            transition: "background 0.2s ease",
          }}
        >
          <Show
            when={justSaved()}
            fallback={<Icon name="save" size={18} color="#f5f0e6" />}
          >
            <AnimatedCheck size={20} color="#f5f0e6" />
          </Show>
        </button>
      </div>

      {/* Row 2: Test button + (inline success timing) + result icon slot.
          Success timing stays inline ("547ms"). Errors get their own
          row below so long upstream messages wrap instead of crushing
          the button width. The icon slot is the last flex child so its
          right edge aligns with the save-icon button above. */}
      <div
        style={{
          display: "flex",
          "align-items": "center",
          gap: "8px",
          "margin-bottom": "0",
        }}
      >
        <button
          type="button"
          disabled={testDisabled()}
          onClick={runTest}
          style={{
            ...inputBase,
            flex: "1 1 auto",
            "min-width": "0",
            padding: "0 14px",
            height: ICON_BTN_SIZE,
            cursor: testDisabled() ? "not-allowed" : "pointer",
            background: testDisabled() ? "#d4c9b5" : "#f5f0e6",
            "margin-bottom": "0",
            "white-space": "nowrap",
            overflow: "hidden",
            "text-overflow": "ellipsis",
          }}
        >
          {testButtonLabel()}
        </button>
        <Show when={testResult()?.success === true}>
          <span
            style={{
              "font-size": "12px",
              color: "#5a5140",
              "font-family": monoFont,
              "white-space": "nowrap",
              "flex-shrink": "0",
            }}
          >
            {testResult()!.duration_ms}ms
          </span>
        </Show>
        {/* Reserved 36x36 slot, anchored at the right column to align
            vertically with the save-icon button above. */}
        <div
          style={{
            width: ICON_BTN_SIZE,
            height: ICON_BTN_SIZE,
            display: "inline-flex",
            "align-items": "center",
            "justify-content": "center",
            "flex-shrink": "0",
          }}
        >
          <Show when={testResult() !== null}>
            <Show
              when={testResult()!.success}
              fallback={<AnimatedCross size={22} />}
            >
              <AnimatedCheck size={22} />
            </Show>
          </Show>
        </div>
      </div>

      {/* Error messages get their own full-width row so long upstream
          responses (auth failures, edge-blocked, etc.) wrap freely
          instead of compressing the Test button. */}
      <Show when={testResult()?.success === false}>
        <div
          style={{
            "margin-top": "6px",
            "font-size": "12px",
            color: "#a33a2a",
            "font-family": monoFont,
            "line-height": "1.4",
            "word-break": "break-word",
            "overflow-wrap": "anywhere",
          }}
        >
          {testResult()!.error ?? "failed"}
        </div>
      </Show>

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
