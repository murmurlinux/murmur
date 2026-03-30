import { createSignal, onMount, onCleanup, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { saveSetting, type ModelInfo } from "../lib/settings";
import logoImg from "../assets/logo.png";

const ACCENT = "#14b8a6";

const glass = {
  padding: "20px",
  background: "rgba(255, 255, 255, 0.025)",
  "border-radius": "14px",
  border: "1px solid rgba(255, 255, 255, 0.06)",
};

const btnPrimary = {
  padding: "10px 28px",
  background: ACCENT,
  border: "none",
  "border-radius": "8px",
  color: "#0c1222",
  "font-weight": 600,
  "font-size": "14px",
  cursor: "pointer",
};

const btnSecondary = {
  padding: "10px 28px",
  background: "rgba(255, 255, 255, 0.04)",
  border: "1px solid rgba(255, 255, 255, 0.08)",
  "border-radius": "8px",
  color: "rgba(255, 255, 255, 0.5)",
  "font-size": "14px",
  cursor: "pointer",
};

const selectStyle = {
  padding: "10px 14px",
  background: "rgba(0, 0, 0, 0.3)",
  border: "1px solid rgba(255, 255, 255, 0.08)",
  "border-radius": "8px",
  color: "rgba(255, 255, 255, 0.8)",
  "font-size": "13px",
  cursor: "pointer",
  width: "100%",
  appearance: "none" as any,
};

// Model filename mapping
function getModelFilename(size: string, lang: string): string {
  const suffix = lang === "english" ? ".en" : "";
  return `ggml-${size}${suffix}.bin`;
}

export function OnboardingWizard() {
  const [step, setStep] = createSignal(0);

  // Step 0: Mic
  const [mics, setMics] = createSignal<{ name: string; available: boolean }[]>([]);
  const [selectedMic, setSelectedMic] = createSignal(0);
  const [micLevel, setMicLevel] = createSignal(0);
  const [micConfirmed, setMicConfirmed] = createSignal(false);
  const [micTesting, setMicTesting] = createSignal(false);
  const [showTroubleshoot, setShowTroubleshoot] = createSignal(false);

  // Step 1: Model
  const [modelSize, setModelSize] = createSignal("tiny");
  const [modelLang, setModelLang] = createSignal("english");
  const [models, setModels] = createSignal<ModelInfo[]>([]);
  const [downloading, setDownloading] = createSignal(false);
  const [downloadPercent, setDownloadPercent] = createSignal(0);
  const [downloadDone, setDownloadDone] = createSignal(false);
  const [downloadError, setDownloadError] = createSignal<string | null>(null);

  // Step 2: Hotkey + Mode
  const [hotkey, setHotkey] = createSignal("Ctrl+Shift+Space");
  const [capturingHotkey, setCapturingHotkey] = createSignal(false);
  const [recordMode, setRecordMode] = createSignal<"hold" | "tap">("hold");

  let unlistenAudio: UnlistenFn | undefined;
  let unlistenProgress: UnlistenFn | undefined;

  onMount(async () => {
    // Load microphones
    try {
      const list = await invoke<{ name: string; available: boolean }[]>("list_microphones");
      if (list.length > 0) {
        setMics(list);
      } else {
        setMics([{ name: "No microphone detected", available: false }]);
      }
    } catch {
      setMics([{ name: "Could not detect microphones", available: false }]);
    }

    // Load models
    try {
      const list = await invoke<ModelInfo[]>("list_models");
      setModels(list);
      // Check if default model is already downloaded
      const defaultFile = getModelFilename("tiny", "english");
      const found = list.find((m) => m.filename === defaultFile && m.downloaded);
      if (found) setDownloadDone(true);
    } catch {
      // models command may fail
    }

    // Listen for download progress
    unlistenProgress = await listen<{ model: string; percent: number }>(
      "model-download-progress",
      (event) => {
        setDownloadPercent(Math.round(event.payload.percent));
      }
    );
  });

  onCleanup(() => {
    unlistenAudio?.();
    unlistenProgress?.();
  });

  // --- Mic test ---
  async function startMicTest() {
    setMicTesting(true);
    setMicConfirmed(false);
    setMicLevel(0);

    // Listen for audio levels
    unlistenAudio = await listen<{ rms: number; peak: number }>("audio-level", (event) => {
      const level = Math.min(event.payload.rms * 20, 1); // normalize
      setMicLevel(level);
      if (event.payload.rms > 0.02) {
        setMicConfirmed(true);
        setMicTesting(false);
      }
    });

    try {
      await invoke("start_mic_test");
    } catch {
      setMicTesting(false);
    }
  }

  // --- Model download ---
  const currentModelFile = () => getModelFilename(modelSize(), modelLang());

  const isCurrentModelDownloaded = () => {
    const file = currentModelFile();
    return models().some((m) => m.filename === file && m.downloaded);
  };

  async function downloadSelectedModel() {
    const filename = currentModelFile();
    setDownloading(true);
    setDownloadPercent(0);
    setDownloadError(null);
    setDownloadDone(false);
    try {
      await invoke("download_model", { modelFilename: filename });
      const list = await invoke<ModelInfo[]>("list_models");
      setModels(list);
      await invoke("set_active_model", { modelFilename: filename });
      await saveSetting("model", filename);
      setDownloadDone(true);
    } catch (e) {
      setDownloadError(`Download failed: ${e}`);
    } finally {
      setDownloading(false);
    }
  }

  // Reset download state when selection changes
  function onSizeChange(size: string) {
    setModelSize(size);
    setDownloadDone(isCurrentModelDownloaded());
    setDownloadError(null);
  }
  function onLangChange(lang: string) {
    setModelLang(lang);
    setDownloadDone(isCurrentModelDownloaded());
    setDownloadError(null);
  }

  // --- Hotkey capture ---
  function handleHotkeyKeyDown(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      setCapturingHotkey(false);
      return;
    }
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;

    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.shiftKey) parts.push("Shift");
    if (e.altKey) parts.push("Alt");
    if (e.metaKey) parts.push("Super");
    parts.push(e.key.length === 1 ? e.key.toUpperCase() : e.key);

    const combo = parts.join("+");
    setHotkey(combo);
    setCapturingHotkey(false);
    invoke("change_hotkey", { newHotkey: combo }).catch(() => {});
    saveSetting("hotkey", combo);
  }

  // --- Finish ---
  async function finish() {
    // Save model if downloaded
    const file = currentModelFile();
    if (isCurrentModelDownloaded() || downloadDone()) {
      await invoke("set_active_model", { modelFilename: file }).catch(() => {});
      await saveSetting("model", file);
    }

    // Save recording mode
    await saveSetting("recordMode", recordMode());

    // Save hotkey
    await saveSetting("hotkey", hotkey());

    // Mark onboarding complete
    await saveSetting("onboardingComplete", true);

    // Close wizard, show main window
    const { emit } = await import("@tauri-apps/api/event");
    await emit("onboarding-complete", {});
    await getCurrentWindow().close();
  }

  return (
    <div
      style={{
        background: "#060d18",
        color: "rgba(255, 255, 255, 0.7)",
        width: "100%",
        "min-height": "100vh",
        "font-family": "'Inter', 'Segoe UI', system-ui, sans-serif",
        padding: "32px",
        "box-sizing": "border-box",
        display: "flex",
        "flex-direction": "column",
      }}
    >
      {/* Header */}
      <div style={{ "text-align": "center", "margin-bottom": "20px" }}>
        <img
          src={logoImg}
          width={48}
          height={48}
          alt="Murmur"
          style={{ "border-radius": "12px" }}
        />
        <h1
          style={{
            "font-size": "22px",
            "font-weight": 600,
            color: "rgba(255, 255, 255, 0.9)",
            margin: "12px 0 4px",
          }}
        >
          Welcome to Murmur
        </h1>
        <p style={{ "font-size": "13px", color: "rgba(255, 255, 255, 0.35)", margin: 0 }}>
          Let's get you set up in a moment
        </p>
      </div>

      {/* Step indicator */}
      <div
        style={{
          display: "flex",
          "justify-content": "center",
          gap: "8px",
          "margin-bottom": "20px",
        }}
      >
        <For each={[0, 1, 2]}>
          {(i) => (
            <div
              style={{
                width: step() === i ? "24px" : "8px",
                height: "8px",
                "border-radius": "4px",
                background: step() >= i ? ACCENT : "rgba(255, 255, 255, 0.1)",
                transition: "all 0.3s ease",
              }}
            />
          )}
        </For>
      </div>

      {/* Step content */}
      <div style={{ flex: 1 }}>
        {/* ========== STEP 0: Microphone ========== */}
        <Show when={step() === 0}>
          <div style={glass}>
            <h2 style={{ "font-size": "16px", "font-weight": 600, color: "rgba(255, 255, 255, 0.85)", margin: "0 0 12px" }}>
              Microphone Check
            </h2>

            {/* Device selector */}
            <Show when={mics().length > 1}>
              <select
                style={selectStyle}
                value={selectedMic()}
                onChange={(e) => {
                  setSelectedMic(parseInt(e.currentTarget.value));
                  setMicConfirmed(false);
                  setMicLevel(0);
                }}
              >
                <For each={mics()}>
                  {(mic, i) => <option value={i()}>{mic.name}</option>}
                </For>
              </select>
              <div style={{ height: "10px" }} />
            </Show>

            {/* Single mic display */}
            <Show when={mics().length === 1}>
              <div
                style={{
                  padding: "12px",
                  background: "rgba(0, 0, 0, 0.2)",
                  "border-radius": "8px",
                  border: "1px solid rgba(255, 255, 255, 0.04)",
                  "font-size": "13px",
                  color: "rgba(255, 255, 255, 0.7)",
                  "margin-bottom": "10px",
                }}
              >
                {mics()[0]?.name || "Unknown"}
              </div>
            </Show>

            {/* Level meter + test button */}
            <Show when={mics()[selectedMic()]?.available !== false}>
              <div style={{ display: "flex", "align-items": "center", gap: "12px", "margin-bottom": "12px" }}>
                {/* Level bar */}
                <div
                  style={{
                    flex: 1,
                    height: "8px",
                    background: "rgba(255, 255, 255, 0.06)",
                    "border-radius": "4px",
                    overflow: "hidden",
                  }}
                >
                  <div
                    style={{
                      height: "100%",
                      width: `${micLevel() * 100}%`,
                      background: micConfirmed() ? ACCENT : "rgba(255, 255, 255, 0.3)",
                      "border-radius": "4px",
                      transition: "width 0.1s ease",
                    }}
                  />
                </div>

                {/* Status indicator */}
                <Show when={micConfirmed()}>
                  <div
                    style={{
                      width: "24px",
                      height: "24px",
                      "border-radius": "50%",
                      background: "rgba(20, 184, 166, 0.15)",
                      display: "flex",
                      "align-items": "center",
                      "justify-content": "center",
                      color: ACCENT,
                      "font-size": "14px",
                      "flex-shrink": 0,
                    }}
                  >
                    {"\u2713"}
                  </div>
                </Show>
              </div>

              <Show when={!micConfirmed() && !micTesting()}>
                <button
                  onClick={startMicTest}
                  style={{
                    ...btnSecondary,
                    width: "100%",
                    padding: "8px",
                    "font-size": "12px",
                  }}
                >
                  Test Microphone
                </button>
              </Show>
              <Show when={micTesting() && !micConfirmed()}>
                <p style={{ "font-size": "12px", color: "rgba(255, 255, 255, 0.5)", margin: "4px 0 0", "text-align": "center" }}>
                  Speak now... say "Hey Murmur"
                </p>
              </Show>
              <Show when={micConfirmed()}>
                <p style={{ "font-size": "12px", color: ACCENT, margin: "4px 0 0", "text-align": "center" }}>
                  Microphone confirmed working
                </p>
              </Show>
            </Show>

            <Show when={mics()[selectedMic()]?.available === false}>
              <p style={{ "font-size": "12px", color: "rgba(239, 68, 68, 0.7)", margin: "8px 0 0" }}>
                No microphone detected. You can continue, but recording won't work until one is connected.
              </p>
            </Show>

            {/* Troubleshooting */}
            <div style={{ "margin-top": "12px", "text-align": "center" }}>
              <button
                onClick={() => setShowTroubleshoot(!showTroubleshoot())}
                style={{
                  background: "none",
                  border: "none",
                  color: "rgba(255, 255, 255, 0.25)",
                  "font-size": "11px",
                  cursor: "pointer",
                  "text-decoration": "underline",
                }}
              >
                Having trouble?
              </button>
              <Show when={showTroubleshoot()}>
                <div
                  style={{
                    "margin-top": "8px",
                    padding: "10px",
                    background: "rgba(0, 0, 0, 0.2)",
                    "border-radius": "8px",
                    "font-size": "11px",
                    color: "rgba(255, 255, 255, 0.4)",
                    "text-align": "left",
                  }}
                >
                  <p style={{ margin: "0 0 6px" }}>Check that your microphone is:</p>
                  <ul style={{ margin: 0, "padding-left": "16px" }}>
                    <li>Plugged in and powered on</li>
                    <li>Set as the default input in your system sound settings</li>
                    <li>Not muted in PulseAudio/PipeWire volume control</li>
                    <li>Allowed by your desktop environment's privacy settings</li>
                  </ul>
                </div>
              </Show>
            </div>
          </div>
        </Show>

        {/* ========== STEP 1: Model + Language ========== */}
        <Show when={step() === 1}>
          <div style={glass}>
            <h2 style={{ "font-size": "16px", "font-weight": 600, color: "rgba(255, 255, 255, 0.85)", margin: "0 0 6px" }}>
              Speech Model
            </h2>
            <p style={{ "font-size": "12px", color: "rgba(255, 255, 255, 0.35)", margin: "0 0 16px" }}>
              Choose your model size and language. You can change these later in Settings.
            </p>

            {/* Size selector */}
            <label style={{ "font-size": "11px", color: "rgba(255, 255, 255, 0.4)", "text-transform": "uppercase", "letter-spacing": "0.05em", "margin-bottom": "6px", display: "block" }}>
              Model Size
            </label>
            <div style={{ display: "flex", gap: "6px", "margin-bottom": "14px" }}>
              {(["tiny", "base", "small"] as const).map((size) => (
                <button
                  onClick={() => onSizeChange(size)}
                  style={{
                    flex: 1,
                    padding: "10px 8px",
                    background: modelSize() === size ? `${ACCENT}18` : "rgba(0, 0, 0, 0.3)",
                    border: modelSize() === size ? `1px solid ${ACCENT}44` : "1px solid rgba(255, 255, 255, 0.04)",
                    "border-radius": "8px",
                    color: modelSize() === size ? ACCENT : "rgba(255, 255, 255, 0.5)",
                    cursor: "pointer",
                    "font-size": "12px",
                    "font-weight": modelSize() === size ? "600" : "400",
                    "text-align": "center",
                  }}
                >
                  <div>{size.charAt(0).toUpperCase() + size.slice(1)}</div>
                  <div style={{ "font-size": "9px", "margin-top": "2px", opacity: 0.6 }}>
                    {size === "tiny" ? "Fastest" : size === "base" ? "Balanced" : "Most accurate"}
                  </div>
                </button>
              ))}
            </div>

            {/* Language selector */}
            <label style={{ "font-size": "11px", color: "rgba(255, 255, 255, 0.4)", "text-transform": "uppercase", "letter-spacing": "0.05em", "margin-bottom": "6px", display: "block" }}>
              Language
            </label>
            <div style={{ display: "flex", gap: "6px", "margin-bottom": "16px" }}>
              {(["english", "multilingual"] as const).map((lang) => (
                <button
                  onClick={() => onLangChange(lang)}
                  style={{
                    flex: 1,
                    padding: "10px 8px",
                    background: modelLang() === lang ? `${ACCENT}18` : "rgba(0, 0, 0, 0.3)",
                    border: modelLang() === lang ? `1px solid ${ACCENT}44` : "1px solid rgba(255, 255, 255, 0.04)",
                    "border-radius": "8px",
                    color: modelLang() === lang ? ACCENT : "rgba(255, 255, 255, 0.5)",
                    cursor: "pointer",
                    "font-size": "12px",
                    "font-weight": modelLang() === lang ? "600" : "400",
                  }}
                >
                  {lang === "english" ? "English" : "Multilingual (99+)"}
                </button>
              ))}
            </div>

            {/* Selected model info */}
            <div
              style={{
                padding: "10px 14px",
                background: "rgba(0, 0, 0, 0.2)",
                "border-radius": "8px",
                border: "1px solid rgba(255, 255, 255, 0.04)",
                "font-size": "12px",
                color: "rgba(255, 255, 255, 0.5)",
                display: "flex",
                "justify-content": "space-between",
                "align-items": "center",
              }}
            >
              <span>{currentModelFile()}</span>
              <Show when={isCurrentModelDownloaded() || downloadDone()}>
                <span style={{ color: ACCENT, "font-size": "11px" }}>{"\u2713"} Downloaded</span>
              </Show>
            </div>

            {/* Progress bar */}
            <Show when={downloading()}>
              <div
                style={{
                  "margin-top": "10px",
                  height: "4px",
                  background: "rgba(255, 255, 255, 0.06)",
                  "border-radius": "2px",
                  overflow: "hidden",
                }}
              >
                <div
                  style={{
                    height: "100%",
                    width: `${downloadPercent()}%`,
                    background: ACCENT,
                    "border-radius": "2px",
                    transition: "width 0.3s ease",
                  }}
                />
              </div>
            </Show>

            {/* Error */}
            <Show when={downloadError()}>
              <p style={{ "font-size": "11px", color: "#ef4444", "margin-top": "8px" }}>
                {downloadError()}
              </p>
            </Show>
          </div>
        </Show>

        {/* ========== STEP 2: Hotkey + Recording Mode ========== */}
        <Show when={step() === 2}>
          <div style={glass}>
            <h2 style={{ "font-size": "16px", "font-weight": 600, color: "rgba(255, 255, 255, 0.85)", margin: "0 0 6px" }}>
              Hotkey & Recording Mode
            </h2>
            <p style={{ "font-size": "12px", color: "rgba(255, 255, 255, 0.35)", margin: "0 0 16px" }}>
              Set your key combination and how you want to control recording.
            </p>

            {/* Hotkey display/capture */}
            <label style={{ "font-size": "11px", color: "rgba(255, 255, 255, 0.4)", "text-transform": "uppercase", "letter-spacing": "0.05em", "margin-bottom": "6px", display: "block" }}>
              Hotkey
            </label>
            <div style={{ display: "flex", gap: "8px", "align-items": "center", "margin-bottom": "16px" }}>
              <div
                tabIndex={0}
                onKeyDown={capturingHotkey() ? handleHotkeyKeyDown : undefined}
                style={{
                  flex: 1,
                  padding: "12px 16px",
                  background: capturingHotkey() ? "rgba(20, 184, 166, 0.08)" : "rgba(0, 0, 0, 0.25)",
                  "border-radius": "8px",
                  border: capturingHotkey() ? `1px solid ${ACCENT}66` : "1px solid rgba(255, 255, 255, 0.06)",
                  "font-family": "'JetBrains Mono', monospace",
                  "font-size": "16px",
                  "font-weight": 600,
                  color: ACCENT,
                  "text-align": "center",
                  "letter-spacing": "0.03em",
                  outline: "none",
                }}
              >
                {capturingHotkey() ? "Press a key combo..." : hotkey()}
              </div>
              <button
                onClick={() => setCapturingHotkey(!capturingHotkey())}
                style={{
                  ...btnSecondary,
                  padding: "10px 16px",
                  "font-size": "12px",
                }}
              >
                {capturingHotkey() ? "Cancel" : "Change"}
              </button>
            </div>

            {/* Recording mode */}
            <label style={{ "font-size": "11px", color: "rgba(255, 255, 255, 0.4)", "text-transform": "uppercase", "letter-spacing": "0.05em", "margin-bottom": "6px", display: "block" }}>
              Recording Mode
            </label>
            <div style={{ display: "flex", gap: "6px" }}>
              {(["hold", "tap"] as const).map((mode) => (
                <button
                  onClick={() => setRecordMode(mode)}
                  style={{
                    flex: 1,
                    padding: "10px 12px",
                    background: recordMode() === mode ? `${ACCENT}18` : "rgba(0, 0, 0, 0.3)",
                    border: recordMode() === mode ? `1px solid ${ACCENT}44` : "1px solid rgba(255, 255, 255, 0.04)",
                    "border-radius": "8px",
                    color: recordMode() === mode ? ACCENT : "rgba(255, 255, 255, 0.4)",
                    cursor: "pointer",
                    "font-size": "12px",
                    "font-weight": recordMode() === mode ? "600" : "400",
                    "text-align": "center",
                  }}
                >
                  <div>{mode === "hold" ? "Hold to Record" : "Tap to Toggle"}</div>
                  <div style={{ "font-size": "9px", "margin-top": "3px", opacity: 0.6 }}>
                    {mode === "hold" ? "Press and hold, release to stop" : "Tap once to start, tap again to stop"}
                  </div>
                </button>
              ))}
            </div>
          </div>
        </Show>
      </div>

      {/* Navigation */}
      <div
        style={{
          display: "flex",
          "justify-content": "space-between",
          "align-items": "center",
          "margin-top": "20px",
        }}
      >
        <Show when={step() > 0} fallback={<div />}>
          <button style={btnSecondary} onClick={() => setStep((s) => s - 1)}>
            Back
          </button>
        </Show>

        <div style={{ display: "flex", gap: "10px" }}>
          <Show when={step() < 2}>
            <button style={btnSecondary} onClick={() => finish()}>
              Skip
            </button>

            {/* Step 1: Download/Next button logic */}
            <Show when={step() === 1}>
              <Show when={isCurrentModelDownloaded() || downloadDone()}>
                <button style={btnPrimary} onClick={() => setStep((s) => s + 1)}>
                  Next
                </button>
              </Show>
              <Show when={!isCurrentModelDownloaded() && !downloadDone() && !downloading()}>
                <button style={btnPrimary} onClick={downloadSelectedModel}>
                  Download
                </button>
              </Show>
              <Show when={downloading()}>
                <button style={{ ...btnPrimary, opacity: 0.7, cursor: "wait" }} disabled>
                  {downloadPercent()}%
                </button>
              </Show>
            </Show>

            {/* Step 0: Just Next */}
            <Show when={step() === 0}>
              <button style={btnPrimary} onClick={() => setStep((s) => s + 1)}>
                Next
              </button>
            </Show>
          </Show>

          <Show when={step() === 2}>
            <button style={btnPrimary} onClick={() => finish()}>
              Get Started
            </button>
          </Show>
        </div>
      </div>
    </div>
  );
}
