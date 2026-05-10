import { createSignal, onMount, onCleanup, JSX, For } from "solid-js";

const monoFont = "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace";

// Custom dropdown styled to match the terminal-cream theme. Single
// source of truth for dropdowns across settings, onboarding, and
// the AI cleanup section. Replaces native <select> for visual
// consistency.
export function Select<T extends string | number>(props: {
  value: T;
  onChange: (v: T) => void;
  options: { value: T; label: string }[];
}): JSX.Element {
  const [open, setOpen] = createSignal(false);
  let containerRef: HTMLDivElement | undefined;

  const selected = () =>
    props.options.find((o) => o.value === props.value)?.label ?? String(props.value);

  const handleClickOutside = (e: MouseEvent) => {
    if (!containerRef?.contains(e.target as Node)) {
      setOpen(false);
    }
  };

  onMount(() => document.addEventListener("mousedown", handleClickOutside));
  onCleanup(() => document.removeEventListener("mousedown", handleClickOutside));

  return (
    <div ref={containerRef} style={{ position: "relative" }}>
      <button
        onClick={() => setOpen(!open())}
        style={{
          width: "100%",
          padding: "8px 12px",
          background: "#f5f0e6",
          border: "1px solid #1a1a1a",
          "border-radius": "0",
          color: "#1a1a1a",
          "font-size": "13px",
          "font-family": monoFont,
          cursor: "pointer",
          "text-align": "left",
          display: "flex",
          "justify-content": "space-between",
          "align-items": "center",
        }}
      >
        {selected()}
        <span style={{ color: "#6b655a", "font-size": "10px" }}>
          {open() ? "▲" : "▼"}
        </span>
      </button>
      {open() && (
        <div
          style={{
            position: "absolute",
            top: "100%",
            left: "0",
            right: "0",
            background: "#f5f0e6",
            border: "1px solid #1a1a1a",
            "border-top": "none",
            "max-height": "200px",
            "overflow-y": "auto",
            "z-index": "10",
          }}
        >
          <For each={props.options}>
            {(opt) => (
              <div
                onClick={() => {
                  props.onChange(opt.value);
                  setOpen(false);
                }}
                style={{
                  padding: "6px 12px",
                  cursor: "pointer",
                  "font-size": "13px",
                  "font-family": monoFont,
                  color: opt.value === props.value ? "#c9482b" : "#1a1a1a",
                  "font-weight": opt.value === props.value ? "700" : "400",
                  background: opt.value === props.value ? "#ece4d0" : "transparent",
                }}
                onMouseEnter={(e) => {
                  if (opt.value !== props.value)
                    e.currentTarget.style.background = "#ece4d0";
                }}
                onMouseLeave={(e) => {
                  if (opt.value !== props.value)
                    e.currentTarget.style.background = "transparent";
                }}
              >
                {opt.label}
              </div>
            )}
          </For>
        </div>
      )}
    </div>
  );
}
