import { createSignal, JSX } from "solid-js";
import { ProGate } from "./ProGate";

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

type Provider = "groq" | "anthropic";

const PROVIDER_DESCRIPTIONS: Record<Provider, string> = {
  groq: "Very fast (typically <1s). Cheapest option. Adequate for cleanup tasks. Uses Llama 3.3 70B.",
  anthropic: "Higher quality prompt-following. Slightly slower (~1-2s). Uses Claude Haiku 4.5.",
};

export function AICleanupSection() {
  const [enabled, setEnabled] = createSignal(true);
  const [provider, setProvider] = createSignal<Provider>("groq");
  const [apiKey, setApiKey] = createSignal("");

  return (
    <ProGate feature="ai-cleanup" title="AI cleanup">
      <div style={glass}>
        <div style={{ ...label, "margin-bottom": "14px" }}>AI cleanup</div>
        <p style={{ "font-size": "12px", color: "#5a5140", "margin-bottom": "14px", "max-width": "520px" }}>
          Polishes dictated text with an LLM: fixes punctuation, capitalisation, typos, and removes filler words. Your wording is preserved. Runs only when enabled; key stored locally on your device.
        </p>

        <div style={{ display: "flex", "align-items": "center", gap: "12px", "margin-bottom": "12px" }}>
          <span style={{ "font-size": "13px", color: "#1a1a1a", "font-family": monoFont }}>Enabled</span>
          <input type="checkbox" checked={enabled()} onChange={(e) => setEnabled(e.currentTarget.checked)} />
        </div>

        <label style={label}>Provider</label>
        <select
          value={provider()}
          onChange={(e) => setProvider(e.currentTarget.value as Provider)}
          style={{ ...inputBase, "margin-bottom": "10px" }}
        >
          <option value="groq">Groq</option>
          <option value="anthropic">Anthropic</option>
        </select>
        <p style={{ "font-size": "11px", color: "#5a5140", "margin-bottom": "14px" }}>
          {PROVIDER_DESCRIPTIONS[provider()]}
        </p>

        <label style={label}>API key</label>
        <input
          type="password"
          value={apiKey()}
          onInput={(e) => setApiKey(e.currentTarget.value)}
          placeholder="Paste your BYOK key"
          style={{ ...inputBase, "margin-bottom": "10px" }}
        />

        <button
          disabled
          style={{
            ...inputBase,
            width: "auto",
            padding: "6px 14px",
            cursor: "not-allowed",
            background: "#d4c9b5",
          }}
        >
          Test (not yet wired)
        </button>
      </div>
    </ProGate>
  );
}
