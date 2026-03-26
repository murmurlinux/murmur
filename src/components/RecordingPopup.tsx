import { onMount, onCleanup, createSignal } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import { loadSettings } from "../lib/settings";
import { hexToRgba, hexToHue } from "../lib/color";
import logoImg from "../assets/logo.png";

const BAR_COUNT = 16;

export function RecordingPopup() {
  const [bars, setBars] = createSignal<number[]>(new Array(BAR_COUNT).fill(0));
  const [isActive, setIsActive] = createSignal(false);
  const [accent, setAccent] = createSignal("#10b981");
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
    const settings = await loadSettings();
    setAccent(settings.accentColor);

    const unlistenSettings = await listen<{ key: string; value: any }>(
      "settings-changed",
      (e) => {
        if (e.payload.key === "accentColor") setAccent(e.payload.value);
      },
    );

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
      unlistenSettings();
      unlistenAudio();
      unlistenState();
      if (animFrame) cancelAnimationFrame(animFrame);
    });
  });

  return (
    // Transparent container fills the whole window — only the pill is visible
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        "align-items": "flex-end",
        "justify-content": "center",
        padding: "0 0 90px 0",
      }}
    >
      {/* The actual pill */}
      <div
        style={{
          display: "inline-flex",
          "align-items": "center",
          gap: "10px",
          padding: "8px 14px",
          background: "rgba(30, 30, 36, 0.92)",
          "border-radius": "24px",
          border: `1px solid ${hexToRgba(accent(), 0.2)}`,
          "box-shadow": "0 4px 20px rgba(0, 0, 0, 0.4)",
        }}
      >
        <img
          src={logoImg}
          alt="M"
          width={24}
          height={24}
          style={{
            "border-radius": "4px",
            filter: `hue-rotate(${hexToHue(accent()) - 168}deg)`,
          }}
        />

        <div
          style={{
            display: "flex",
            "align-items": "center",
            gap: "2px",
            height: "24px",
          }}
        >
          {bars().map((h) => (
            <div
              style={{
                width: "3px",
                height: `${Math.max(4, h * 20)}px`,
                background: accent(),
                "border-radius": "1.5px",
                opacity: `${0.3 + h * 0.7}`,
                transition: "height 0.05s ease, opacity 0.05s ease",
              }}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
