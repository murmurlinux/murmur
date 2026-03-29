import { onMount, onCleanup, createSignal } from "solid-js";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { hexToRgb } from "../lib/color";
import type { RectZone } from "../lib/skin-loader";

interface WaveformProps {
  zone: RectZone;
  sourceWidth: number;
  sourceHeight: number;
  accentColor: string;
}

interface AudioLevel {
  rms: number;
  peak: number;
  samples: number[];
}

export function Waveform(props: WaveformProps) {
  let canvasRef: HTMLCanvasElement | undefined;
  let unlisten: UnlistenFn | undefined;
  let animFrameId: number | undefined;

  const [bars, setBars] = createSignal<number[]>(new Array(48).fill(0));
  const [isActive, setIsActive] = createSignal(false);

  // Smoothing: lerp current bars toward target
  let targetBars: number[] = new Array(48).fill(0);
  let currentBars: number[] = new Array(48).fill(0);

  const leftPct = () => (props.zone.x / props.sourceWidth) * 100;
  const topPct = () => (props.zone.y / props.sourceHeight) * 100;
  const widthPct = () => (props.zone.width / props.sourceWidth) * 100;
  const heightPct = () => (props.zone.height / props.sourceHeight) * 100;

  const draw = () => {
    const canvas = canvasRef;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;

    // Clear with dark background
    ctx.fillStyle = "#080c14";
    ctx.fillRect(0, 0, w, h);

    // Smooth bars toward targets
    const smoothing = 0.3;
    for (let i = 0; i < currentBars.length; i++) {
      const target = i < targetBars.length ? targetBars[i] : 0;
      currentBars[i] += (target - currentBars[i]) * smoothing;
      // Decay toward zero when idle
      if (!isActive()) {
        currentBars[i] *= 0.92;
      }
    }

    const barCount = currentBars.length;
    const barWidth = (w / barCount) * 0.7;
    const gap = (w / barCount) * 0.3;
    const centerY = h / 2;

    // Derive accent RGB for bar colours (lightened version of accent)
    const [ar, ag, ab] = hexToRgb(props.accentColor);
    const lr = Math.round(ar + (255 - ar) * 0.4);
    const lg = Math.round(ag + (255 - ag) * 0.4);
    const lb = Math.round(ab + (255 - ab) * 0.4);

    for (let i = 0; i < barCount; i++) {
      const amplitude = Math.min(currentBars[i], 1); // Already normalised 0-1 from Rust
      const barHeight = Math.max(1, amplitude * (h * 0.8));

      const x = i * (barWidth + gap) + gap / 2;
      const y = centerY - barHeight / 2;

      // Gradient based on amplitude: dim → bright accent
      const alpha = 0.15 + amplitude * 0.75;
      ctx.fillStyle = `rgba(${lr}, ${lg}, ${lb}, ${alpha})`;
      ctx.fillRect(x, y, barWidth, barHeight);

      // Glow effect for louder bars
      if (amplitude > 0.3) {
        ctx.shadowColor = `rgba(${lr}, ${lg}, ${lb}, 0.4)`;
        ctx.shadowBlur = 4;
        ctx.fillRect(x, y, barWidth, barHeight);
        ctx.shadowBlur = 0;
      }
    }

    // Idle state: subtle center line
    if (!isActive()) {
      const maxBar = Math.max(...currentBars);
      if (maxBar < 0.01) {
        ctx.strokeStyle = `rgba(${lr}, ${lg}, ${lb}, 0.15)`;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(w * 0.1, centerY);
        ctx.lineTo(w * 0.9, centerY);
        ctx.stroke();
      }
    }

    // Stop the loop once fully decayed to idle
    const maxBar = Math.max(...currentBars);
    if (!isActive() && maxBar < 0.005) {
      animFrameId = undefined; // Stop looping -- will restart on next audio event
      return;
    }

    animFrameId = requestAnimationFrame(draw);
  };

  const ensureDrawing = () => {
    if (animFrameId === undefined) {
      animFrameId = requestAnimationFrame(draw);
    }
  };

  onMount(async () => {
    // Set canvas resolution
    if (canvasRef) {
      const rect = canvasRef.getBoundingClientRect();
      canvasRef.width = rect.width * 2; // 2x for sharpness
      canvasRef.height = rect.height * 2;
    }

    // Listen for audio level events
    unlisten = await listen<AudioLevel>("audio-level", (event) => {
      targetBars = event.payload.samples;
      setIsActive(true);
      ensureDrawing();
    });

    // Listen for recording state to know when we're idle
    const unlistenState = await listen<{ state: string }>("recording-state", (event) => {
      if (event.payload.state === "idle") {
        setIsActive(false);
        targetBars = new Array(48).fill(0);
        ensureDrawing(); // Run decay animation, then auto-stop
      } else if (event.payload.state === "recording") {
        setIsActive(true);
        ensureDrawing();
      }
    });

    // Draw initial idle frame, then stop
    animFrameId = requestAnimationFrame(draw);

    onCleanup(() => {
      unlisten?.();
      unlistenState?.();
      if (animFrameId) cancelAnimationFrame(animFrameId);
    });
  });

  return (
    <div
      style={{
        position: "absolute",
        left: `${leftPct()}%`,
        top: `${topPct()}%`,
        width: `${widthPct()}%`,
        height: `${heightPct()}%`,
        "border-radius": "12px",
        overflow: "hidden",
        "z-index": 6,
        "pointer-events": "none",
      }}
    >
      <canvas
        ref={canvasRef}
        style={{
          width: "100%",
          height: "100%",
          display: "block",
        }}
      />
    </div>
  );
}
