import { createSignal, onMount, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { MurmurLogo } from "./MurmurLogo";
import { saveSetting } from "../lib/settings";

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

export function OnboardingWizard() {
  const [step, setStep] = createSignal(0);
  const [micName, setMicName] = createSignal("Checking...");
  const [micAvailable, setMicAvailable] = createSignal(false);
  const [models, setModels] = createSignal<any[]>([]);
  const [downloading, setDownloading] = createSignal<string | null>(null);
  const [downloadPercent, setDownloadPercent] = createSignal(0);
  const [selectedModel, setSelectedModel] = createSignal("ggml-tiny.en.bin");

  onMount(async () => {
    // Check microphone
    try {
      const info = await invoke<{ name: string; available: boolean }>("check_microphone");
      setMicName(info.name);
      setMicAvailable(info.available);
    } catch {
      setMicName("Could not detect microphone");
      setMicAvailable(false);
    }

    // Load models
    try {
      const list = await invoke<any[]>("list_models");
      setModels(list);
      // If any model is already downloaded, pre-select it
      const downloaded = list.find((m: any) => m.downloaded);
      if (downloaded) setSelectedModel(downloaded.filename);
    } catch {
      // models command may fail
    }

    // Listen for download progress
    await listen<{ model: string; percent: number }>("model-download-progress", (event) => {
      setDownloadPercent(Math.round(event.payload.percent));
    });
  });

  async function downloadModel(filename: string) {
    setDownloading(filename);
    setDownloadPercent(0);
    try {
      await invoke("download_model", { modelFilename: filename });
      const list = await invoke<any[]>("list_models");
      setModels(list);
      setSelectedModel(filename);
      await invoke("set_active_model", { modelFilename: filename });
      await saveSetting("model", filename);
    } catch {
      // Error handled silently, user can retry
    } finally {
      setDownloading(null);
    }
  }

  async function finish() {
    // Set active model
    const model = selectedModel();
    const modelList = models();
    const isDownloaded = modelList.find((m: any) => m.filename === model && m.downloaded);
    if (isDownloaded) {
      await invoke("set_active_model", { modelFilename: model }).catch(() => {});
      await saveSetting("model", model);
    }

    // Mark onboarding complete
    await saveSetting("onboardingComplete", true);

    // Close wizard, show main window
    const { emit } = await import("@tauri-apps/api/event");
    await emit("onboarding-complete", {});
    await getCurrentWindow().close();
  }

  const hasDownloadedModel = () => models().some((m: any) => m.downloaded);

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
      <div style={{ "text-align": "center", "margin-bottom": "24px" }}>
        <MurmurLogo size={48} />
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
          "margin-bottom": "24px",
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
        {/* Step 1: Microphone */}
        <Show when={step() === 0}>
          <div style={glass}>
            <h2
              style={{
                "font-size": "16px",
                "font-weight": 600,
                color: "rgba(255, 255, 255, 0.85)",
                margin: "0 0 16px",
              }}
            >
              Microphone Check
            </h2>
            <div
              style={{
                display: "flex",
                "align-items": "center",
                gap: "12px",
                padding: "14px",
                background: "rgba(0, 0, 0, 0.2)",
                "border-radius": "10px",
                border: "1px solid rgba(255, 255, 255, 0.04)",
              }}
            >
              <div
                style={{
                  width: "36px",
                  height: "36px",
                  "border-radius": "50%",
                  background: micAvailable()
                    ? "rgba(20, 184, 166, 0.15)"
                    : "rgba(239, 68, 68, 0.15)",
                  display: "flex",
                  "align-items": "center",
                  "justify-content": "center",
                  "font-size": "18px",
                  "flex-shrink": 0,
                }}
              >
                {micAvailable() ? "\u2713" : "\u2717"}
              </div>
              <div>
                <div
                  style={{
                    "font-size": "14px",
                    color: "rgba(255, 255, 255, 0.8)",
                    "font-weight": 500,
                  }}
                >
                  {micName()}
                </div>
                <div
                  style={{
                    "font-size": "11px",
                    color: micAvailable()
                      ? "rgba(20, 184, 166, 0.7)"
                      : "rgba(239, 68, 68, 0.7)",
                    "margin-top": "2px",
                  }}
                >
                  {micAvailable() ? "Ready to record" : "No microphone found"}
                </div>
              </div>
            </div>
            <Show when={!micAvailable()}>
              <p
                style={{
                  "font-size": "12px",
                  color: "rgba(255, 255, 255, 0.35)",
                  "margin-top": "12px",
                }}
              >
                You can continue without a microphone, but recording won't work
                until one is connected.
              </p>
            </Show>
          </div>
        </Show>

        {/* Step 2: Model Download */}
        <Show when={step() === 1}>
          <div style={glass}>
            <h2
              style={{
                "font-size": "16px",
                "font-weight": 600,
                color: "rgba(255, 255, 255, 0.85)",
                margin: "0 0 6px",
              }}
            >
              Speech Model
            </h2>
            <p
              style={{
                "font-size": "12px",
                color: "rgba(255, 255, 255, 0.35)",
                margin: "0 0 16px",
              }}
            >
              Choose a model for voice recognition. Smaller is faster, larger is
              more accurate.
            </p>
            <div
              style={{
                display: "flex",
                "flex-direction": "column",
                gap: "6px",
              }}
            >
              <For each={models()}>
                {(model) => (
                  <div
                    style={{
                      display: "flex",
                      "align-items": "center",
                      gap: "10px",
                      padding: "12px",
                      background:
                        selectedModel() === model.filename
                          ? "rgba(20, 184, 166, 0.06)"
                          : "rgba(0, 0, 0, 0.2)",
                      border:
                        selectedModel() === model.filename
                          ? `1px solid ${ACCENT}33`
                          : "1px solid rgba(255, 255, 255, 0.04)",
                      "border-radius": "8px",
                      transition: "all 0.2s ease",
                    }}
                  >
                    <div style={{ flex: 1 }}>
                      <div
                        style={{
                          "font-size": "13px",
                          "font-weight": 500,
                          color: "rgba(255, 255, 255, 0.8)",
                        }}
                      >
                        {model.name}
                        <Show when={model.filename === "ggml-tiny.en.bin"}>
                          <span
                            style={{
                              "font-size": "9px",
                              color: ACCENT,
                              "margin-left": "6px",
                              "text-transform": "uppercase",
                              "letter-spacing": "0.05em",
                            }}
                          >
                            Recommended
                          </span>
                        </Show>
                      </div>
                      <div
                        style={{
                          "font-size": "10px",
                          color: "rgba(255, 255, 255, 0.25)",
                          "margin-top": "2px",
                        }}
                      >
                        {model.description} — {model.size_mb}MB
                      </div>
                    </div>
                    {model.downloaded ? (
                      <span
                        style={{
                          "font-size": "10px",
                          color: ACCENT,
                          "font-weight": 600,
                        }}
                      >
                        Downloaded
                      </span>
                    ) : downloading() === model.filename ? (
                      <span
                        style={{
                          "font-size": "10px",
                          color: "rgba(255, 255, 255, 0.4)",
                        }}
                      >
                        {downloadPercent()}%
                      </span>
                    ) : (
                      <button
                        onClick={() => downloadModel(model.filename)}
                        style={{
                          padding: "4px 12px",
                          background: "rgba(255, 255, 255, 0.04)",
                          border: `1px solid ${ACCENT}44`,
                          "border-radius": "6px",
                          color: ACCENT,
                          cursor: "pointer",
                          "font-size": "11px",
                        }}
                      >
                        Download
                      </button>
                    )}
                  </div>
                )}
              </For>
            </div>
            {/* Download progress bar */}
            <Show when={downloading()}>
              <div
                style={{
                  "margin-top": "12px",
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
          </div>
        </Show>

        {/* Step 3: Hotkey */}
        <Show when={step() === 2}>
          <div style={glass}>
            <h2
              style={{
                "font-size": "16px",
                "font-weight": 600,
                color: "rgba(255, 255, 255, 0.85)",
                margin: "0 0 6px",
              }}
            >
              Your Hotkey
            </h2>
            <p
              style={{
                "font-size": "12px",
                color: "rgba(255, 255, 255, 0.35)",
                margin: "0 0 16px",
              }}
            >
              Press and hold this key combination anywhere to dictate. Release to
              stop and inject text at your cursor.
            </p>
            <div
              style={{
                "text-align": "center",
                padding: "24px",
                background: "rgba(0, 0, 0, 0.25)",
                "border-radius": "10px",
                border: "1px solid rgba(255, 255, 255, 0.06)",
              }}
            >
              <div
                style={{
                  "font-size": "24px",
                  "font-weight": 600,
                  color: ACCENT,
                  "font-family": "'JetBrains Mono', monospace",
                  "letter-spacing": "0.05em",
                }}
              >
                Ctrl + Shift + Space
              </div>
              <p
                style={{
                  "font-size": "11px",
                  color: "rgba(255, 255, 255, 0.3)",
                  "margin-top": "10px",
                }}
              >
                You can change this anytime in Settings
              </p>
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
          "margin-top": "24px",
        }}
      >
        <Show when={step() > 0} fallback={<div />}>
          <button style={btnSecondary} onClick={() => setStep((s) => s - 1)}>
            Back
          </button>
        </Show>

        <div style={{ display: "flex", gap: "10px" }}>
          <Show when={step() < 2}>
            <button
              style={btnSecondary}
              onClick={() => finish()}
            >
              Skip
            </button>
            <button
              style={btnPrimary}
              onClick={() => setStep((s) => s + 1)}
              disabled={step() === 1 && !hasDownloadedModel() && !downloading()}
            >
              Next
            </button>
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
