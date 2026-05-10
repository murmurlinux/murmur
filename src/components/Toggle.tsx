import { JSX } from "solid-js";

// Pill-style boolean toggle. Single source of truth for the on/off
// switch used across the settings panel, onboarding wizard, and
// AI cleanup section.
export function Toggle(props: {
  value: boolean;
  onChange: () => void;
  disabled?: boolean;
}): JSX.Element {
  return (
    <button
      onClick={() => !props.disabled && props.onChange()}
      style={{
        width: "40px",
        height: "22px",
        "border-radius": "11px",
        border: "none",
        cursor: props.disabled ? "not-allowed" : "pointer",
        background: props.disabled ? "#e0d9cc" : props.value ? "#c9482b" : "#d4c9b5",
        position: "relative",
        transition: "background 0.2s ease",
        "flex-shrink": "0",
        opacity: props.disabled ? "0.5" : "1",
      }}
    >
      <div
        style={{
          width: "16px",
          height: "16px",
          "border-radius": "50%",
          background: "#f5f0e6",
          position: "absolute",
          top: "3px",
          left: props.value && !props.disabled ? "21px" : "3px",
          transition: "left 0.2s ease",
        }}
      />
    </button>
  );
}
