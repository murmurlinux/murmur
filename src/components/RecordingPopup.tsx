import { onMount, onCleanup, createSignal, createEffect } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { MurmurLogo } from "./MurmurLogo";

const BAR_COUNT = 16;

export function RecordingPopup() {
  const [bars, setBars] = createSignal<number[]>(new Array(BAR_COUNT).fill(0));
  const [isActive, setIsActive] = createSignal(false);
  let currentBars = new Array(BAR_COUNT).fill(0);
  let targetBars = new Array(BAR_COUNT).fill(0);
  let animFrame: number | undefined;

  const animate = () => {
    let changed = false;
    for (let i = 0; i < BAR_COUNT; i++) {
      const target = isActive() ? targetBars[i] : 0;
      const diff = target - currentBars[i];
      if (Math.abs(diff) > 0.005) {
        currentBars[i] += diff * 0.3;
        changed = true;
      } else {
        currentBars[i] = target;
      }
    }
    setBars([...currentBars]);
    if (changed || isActive()) {
      animFrame = requestAnimationFrame(animate);
    }
  };

  onMount(async () => {
    const unlistenAudio = await listen<{ samples: number[] }>("audio-level", (e) => {
      const src = e.payload.samples;
      const step = Math.floor(src.length / BAR_COUNT);
      for (let i = 0; i < BAR_COUNT; i++) {
        targetBars[i] = src[i * step] ?? 0;
      }
      if (!animFrame) animFrame = requestAnimationFrame(animate);
    });

    const unlistenState = await listen<{ state: string }>("recording-state", (e) => {
      const active = e.payload.state === "recording";
      setIsActive(active);
      if (active && !animFrame) animFrame = requestAnimationFrame(animate);
    });

    onCleanup(() => {
      unlistenAudio();
      unlistenState();
      if (animFrame) cancelAnimationFrame(animFrame);
    });
  });

  return (
    <div
      style={{
        display: "flex",
        "align-items": "center",
        gap: "12px",
        width: "100%",
        height: "100%",
        padding: "0 16px",
        background: "rgba(30, 30, 36, 0.92)",
        "border-radius": "24px",
        border: "1px solid rgba(20, 184, 166, 0.15)",
        "box-shadow": "0 4px 20px rgba(0, 0, 0, 0.4)",
      }}
    >
      <MurmurLogo size={24} color="#14b8a6" />

      <div
        style={{
          display: "flex",
          "align-items": "center",
          gap: "2px",
          flex: 1,
          height: "28px",
        }}
      >
        {bars().map((h) => (
          <div
            style={{
              width: "3px",
              height: `${Math.max(4, h * 24)}px`,
              background: "#14b8a6",
              "border-radius": "1.5px",
              opacity: `${0.3 + h * 0.7}`,
              transition: "height 0.05s ease, opacity 0.05s ease",
            }}
          />
        ))}
      </div>
    </div>
  );
}
