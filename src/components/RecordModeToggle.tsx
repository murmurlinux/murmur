import { For, type JSX } from "solid-js";

export type RecordMode = "hold" | "tap";

const ACCENT = "#c9482b";
const monoFont = "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace";

const MODES: ReadonlyArray<{ value: RecordMode; label: string; description: string }> = [
  { value: "hold", label: "Hold to Record", description: "Press and hold, release to stop" },
  { value: "tap", label: "Tap to Toggle", description: "Tap once to start, tap again to stop" },
];

type Variant = "wizard" | "settings";

interface RecordModeToggleProps {
  value: RecordMode;
  onChange: (mode: RecordMode) => void;
  variant: Variant;
}

function buttonStyle(variant: Variant, selected: boolean): JSX.CSSProperties {
  const base: JSX.CSSProperties = {
    flex: 1,
    background: selected ? "#f5f0e6" : "#ece4d0",
    border: selected ? `1px solid ${ACCENT}` : "1px solid #d4c9b5",
    "border-radius": "0",
    color: selected ? ACCENT : "#6b655a",
    cursor: "pointer",
    "font-size": "12px",
    "font-weight": selected ? "600" : "400",
  };

  if (variant === "wizard") {
    return { ...base, padding: "10px 12px", "text-align": "center" };
  }

  return {
    ...base,
    padding: "8px 12px",
    "font-family": monoFont,
    transition: "all 0.2s ease",
  };
}

export function RecordModeToggle(props: RecordModeToggleProps) {
  return (
    <div style={{ display: "flex", gap: "6px" }}>
      <For each={MODES}>
        {(mode) => (
          <button
            onClick={() => props.onChange(mode.value)}
            style={buttonStyle(props.variant, props.value === mode.value)}
          >
            {props.variant === "wizard" ? (
              <>
                <div>{mode.label}</div>
                <div style={{ "font-size": "9px", "margin-top": "3px", opacity: 0.6 }}>
                  {mode.description}
                </div>
              </>
            ) : (
              mode.label
            )}
          </button>
        )}
      </For>
    </div>
  );
}
