import { createSignal, onMount, onCleanup } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { hexToRgba, lighten } from "../lib/color";
import type { CircleZone } from "../lib/skin-loader";

interface StatusLedProps {
  zone: CircleZone;
  sourceWidth: number;
  sourceHeight: number;
  accentColor: string;
}

type LedState = "idle" | "recording" | "processing";

export function StatusLed(props: StatusLedProps) {
  const [state, setState] = createSignal<LedState>("idle");

  onMount(async () => {
    // Register listener FIRST, before any async work
    const unlisten = await listen<{ state: string }>("recording-state", (event) => {
      const s = event.payload.state;
      if (s === "recording" || s === "processing" || s === "idle") {
        setState(s as LedState);
      }
    });

    onCleanup(() => unlisten());
  });

  const leftPct = () => ((props.zone.cx - props.zone.radius) / props.sourceWidth) * 100;
  const topPct = () => ((props.zone.cy - props.zone.radius) / props.sourceHeight) * 100;
  const widthPct = () => ((props.zone.radius * 2) / props.sourceWidth) * 100;
  const heightPct = () => ((props.zone.radius * 2) / props.sourceHeight) * 100;

  const litColor = () => lighten(props.accentColor, 0.3);

  const background = () => {
    switch (state()) {
      case "recording":
        return hexToRgba(litColor(), 0.9);
      case "processing":
        return "rgba(220, 50, 50, 0.9)";
      default:
        return hexToRgba(props.accentColor, 0.08);
    }
  };

  const boxShadow = () => {
    switch (state()) {
      case "recording":
        return `0 0 10px 4px ${hexToRgba(litColor(), 0.6)}`;
      case "processing":
        return "0 0 10px 4px rgba(220, 50, 50, 0.5)";
      default:
        return "none";
    }
  };

  const animation = () => {
    switch (state()) {
      case "recording":
        return "status-pulse 1.2s ease-in-out infinite";
      case "processing":
        return "none";
      default:
        return "none";
    }
  };

  return (
    <>
      <style>{`
        @keyframes status-pulse {
          0%, 100% { opacity: 0.6; }
          50% { opacity: 1; }
        }
      `}</style>
      <div
        style={{
          position: "absolute",
          left: `${leftPct()}%`,
          top: `${topPct()}%`,
          width: `${widthPct()}%`,
          height: `${heightPct()}%`,
          "border-radius": "50%",
          background: background(),
          "box-shadow": boxShadow(),
          transition: "background 0.2s ease, box-shadow 0.2s ease",
          animation: animation(),
          "pointer-events": "none",
          "z-index": 8,
        }}
      />
    </>
  );
}
