import { createSignal, onMount, For, JSX } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { loadSettings, saveSetting, type MurmurSettings } from "../lib/settings";
import { hexToHue, hueToHex, hexToRgba } from "../lib/color";
import logoImg from "../assets/logo.png";

// --- Ocean Terminal Theme ---

const glass: JSX.CSSProperties = {
  "margin-bottom": "16px",
  padding: "18px",
  background: "rgba(255, 255, 255, 0.025)",
  "border-radius": "14px",
  border: "1px solid rgba(255, 255, 255, 0.06)",
  transition: "border-color 0.2s ease",
};

const label: JSX.CSSProperties = {
  display: "block",
  "font-size": "10px",
  "font-weight": "600",
  "text-transform": "uppercase",
  "letter-spacing": "0.08em",
  color: "#14b8a6",
  "margin-bottom": "10px",
};

const inputBase: JSX.CSSProperties = {
  width: "100%",
  padding: "8px 12px",
  background: "rgba(0, 0, 0, 0.3)",
  border: "1px solid rgba(255, 255, 255, 0.06)",
  "border-radius": "8px",
  color: "#e0e0e0",
  "font-size": "13px",
  "font-family": "-apple-system, system-ui, sans-serif",
  "box-sizing": "border-box",
  outline: "none",
};

function Toggle(props: { value: boolean; onChange: () => void; accent: string }) {
  return (
    <button
      onClick={props.onChange}
      style={{
        width: "40px",
        height: "22px",
        "border-radius": "11px",
        border: "none",
        cursor: "pointer",
        background: props.value ? hexToRgba(props.accent, 0.55) : "rgba(255, 255, 255, 0.08)",
        position: "relative",
        transition: "background 0.2s ease",
        "flex-shrink": "0",
      }}
    >
      <div
        style={{
          width: "16px",
          height: "16px",
          "border-radius": "50%",
          background: "rgba(255, 255, 255, 0.9)",
          position: "absolute",
          top: "3px",
          left: props.value ? "21px" : "3px",
          transition: "left 0.2s ease",
        }}
      />
    </button>
  );
}

function SettingRow(props: { label: string; children: JSX.Element }) {
  return (
    <div
      style={{
        display: "flex",
        "align-items": "center",
        "justify-content": "space-between",
      }}
    >
      <span style={{ "font-size": "13px", color: "rgba(255, 255, 255, 0.7)" }}>{props.label}</span>
      {props.children}
    </div>
  );
}

// --- Component ---

export function SettingsPanel() {
  const [settings, setSettings] = createSignal<MurmurSettings | null>(null);
  const [hue, setHue] = createSignal(160);
  const [capturingHotkey, setCapturingHotkey] = createSignal(false);
  const [models, setModels] = createSignal<any[]>([]);
  const [downloadingModel, setDownloadingModel] = createSignal<string | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [version, setVersion] = createSignal("...");

  const accent = () => hueToHex(hue());

  const showError = (msg: string) => {
    setError(msg);
    setTimeout(() => setError(null), 5000);
  };

  onMount(async () => {
    // Read version from Tauri config (not hardcoded)
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      setVersion(await getVersion());
    } catch { /* fallback stays as "..." */ }

    const s = await loadSettings();
    setSettings(s);
    setHue(hexToHue(s.accentColor));

    try {
      const list = await invoke<any[]>("list_models");
      setModels(list);
    } catch {
      // Models command may not exist yet
    }
  });

  const updateSetting = async <K extends keyof MurmurSettings>(
    key: K,
    value: MurmurSettings[K],
  ) => {
    await saveSetting(key, value);
    setSettings((prev) => (prev ? { ...prev, [key]: value } : prev));
    await emit("settings-changed", { key, value });
  };

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
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Super");

    let key = e.key;
    if (key === " ") key = "Space";
    else if (key.length === 1) key = key.toUpperCase();

    parts.push(key);
    const combo = parts.join("+");

    setCapturingHotkey(false);
    updateSetting("hotkey", combo);
    invoke("change_hotkey", { newHotkey: combo }).catch((err) =>
      showError(`Failed to set hotkey "${combo}": ${err}`),
    );
  }

  async function downloadModel(filename: string) {
    setDownloadingModel(filename);
    try {
      await invoke("download_model", { modelFilename: filename });
      const list = await invoke<any[]>("list_models");
      setModels(list);
    } catch (e) {
      showError(`Download failed: ${e}`);
    } finally {
      setDownloadingModel(null);
    }
  }

  async function selectModel(filename: string) {
    await invoke("set_active_model", { modelFilename: filename }).catch(() => {});
    updateSetting("model", filename);
  }

  return (
    <div
      style={{
        background: "#060d18",
        color: "rgba(255, 255, 255, 0.6)",
        width: "100%",
        "min-height": "100vh",
        padding: "20px",
        "box-sizing": "border-box",
        "font-family": "-apple-system, system-ui, sans-serif",
        position: "relative",
      }}
    >
      {/* Subtle gradient overlay */}
      <div
        style={{
          position: "fixed",
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          background: "radial-gradient(ellipse at 50% 0%, rgba(20, 184, 166, 0.04) 0%, transparent 60%)",
          "pointer-events": "none",
          "z-index": 0,
        }}
      />

      <div style={{ position: "relative", "z-index": 1 }}>
        {/* Header */}
        <div
          style={{
            display: "flex",
            "align-items": "center",
            gap: "10px",
            "margin-bottom": "20px",
          }}
        >
          <img src={logoImg} alt="Murmur" width={28} height={28} style={{ "border-radius": "6px" }} />
          <div style={{ flex: 1 }}>
            <div style={{ "font-size": "16px", "font-weight": 600, color: "rgba(255, 255, 255, 0.9)" }}>
              Murmur
            </div>
          </div>
          <span style={{ "font-size": "10px", color: "rgba(255, 255, 255, 0.2)", "font-family": "monospace" }}>
            v{version()}
          </span>
        </div>

        {error() && (
          <div
            style={{
              padding: "10px 14px",
              background: "rgba(220, 50, 50, 0.1)",
              border: "1px solid rgba(220, 50, 50, 0.2)",
              "border-radius": "10px",
              color: "#ff8888",
              "font-size": "12px",
              "margin-bottom": "16px",
              cursor: "pointer",
            }}
            onClick={() => setError(null)}
          >
            {error()}
          </div>
        )}

        {settings() && (
          <>
            {/* Skin */}
            <div style={glass}>
              <label style={label}>Skin</label>
              <div
                style={{
                  padding: "8px 12px",
                  background: "rgba(0, 0, 0, 0.3)",
                  "border-radius": "8px",
                  border: "1px solid rgba(255, 255, 255, 0.06)",
                  color: "rgba(255, 255, 255, 0.7)",
                  "font-size": "13px",
                }}
              >
                Comm Badge
                <span style={{ color: "rgba(255, 255, 255, 0.2)", "margin-left": "8px", "font-size": "11px" }}>
                  default
                </span>
              </div>
            </div>

            {/* Accent Colour */}
            <div style={glass}>
              <label style={label}>Accent Colour</label>
              <div style={{ display: "flex", "align-items": "center", gap: "12px" }}>
                <input
                  type="range"
                  min="0"
                  max="360"
                  value={hue()}
                  onInput={(e) => {
                    const h = parseInt(e.currentTarget.value);
                    setHue(h);
                    updateSetting("accentColor", hueToHex(h));
                  }}
                  style={{
                    flex: 1,
                    height: "4px",
                    "border-radius": "2px",
                    appearance: "auto",
                    cursor: "pointer",
                    background: "linear-gradient(to right, #ff0000, #ffff00, #00ff00, #00ffff, #0000ff, #ff00ff, #ff0000)",
                  }}
                />
                <div
                  style={{
                    width: "28px",
                    height: "28px",
                    "border-radius": "50%",
                    background: hexToRgba(accent(), 0.6),
                    border: "2px solid rgba(255, 255, 255, 0.08)",
                    "flex-shrink": "0",
                    "box-shadow": `0 0 8px ${accent()}22`,
                  }}
                />
              </div>
            </div>

            {/* Hotkey */}
            <div style={glass}>
              <label style={label}>Global Hotkey</label>
              <div style={{ display: "flex", gap: "8px" }}>
                <div
                  tabIndex={0}
                  onFocus={() => setCapturingHotkey(true)}
                  onBlur={() => setCapturingHotkey(false)}
                  onKeyDown={(e) => capturingHotkey() && handleHotkeyKeyDown(e)}
                  style={{
                    ...inputBase,
                    flex: "1",
                    cursor: "pointer",
                    "border-color": capturingHotkey() ? accent() : "rgba(255, 255, 255, 0.06)",
                    "text-align": "center",
                    "user-select": "none",
                    "font-family": "monospace",
                  }}
                >
                  {capturingHotkey() ? (
                    <span style={{ color: "rgba(255, 255, 255, 0.3)", "font-style": "italic", "font-family": "-apple-system, system-ui, sans-serif" }}>
                      Press a key combo...
                    </span>
                  ) : (
                    settings()!.hotkey
                  )}
                </div>
                <button
                  onClick={() => {
                    updateSetting("hotkey", "Ctrl+Shift+Space");
                    invoke("change_hotkey", { newHotkey: "Ctrl+Shift+Space" }).catch(() => {});
                  }}
                  style={{
                    padding: "8px 12px",
                    background: "rgba(255, 255, 255, 0.04)",
                    border: "1px solid rgba(255, 255, 255, 0.06)",
                    "border-radius": "8px",
                    color: "rgba(255, 255, 255, 0.4)",
                    cursor: "pointer",
                    "font-size": "11px",
                    "white-space": "nowrap",
                    transition: "background 0.2s ease",
                  }}
                >
                  Reset
                </button>
              </div>
            </div>

            {/* Model */}
            <div style={glass}>
              <label style={label}>Whisper Model</label>
              <div style={{ display: "flex", "flex-direction": "column", gap: "6px" }}>
                {models().length > 0 ? (
                  <For each={models()}>
                    {(model) => (
                      <div
                        style={{
                          display: "flex",
                          "align-items": "center",
                          gap: "10px",
                          padding: "10px 12px",
                          background: settings()!.model === model.filename
                            ? "rgba(20, 184, 166, 0.06)"
                            : "rgba(0, 0, 0, 0.2)",
                          border: settings()!.model === model.filename
                            ? `1px solid ${accent()}33`
                            : "1px solid rgba(255, 255, 255, 0.04)",
                          "border-radius": "8px",
                          transition: "all 0.2s ease",
                        }}
                      >
                        <div style={{ flex: 1 }}>
                          <div style={{ "font-size": "13px", "font-weight": 500, color: "rgba(255, 255, 255, 0.8)" }}>
                            {model.name}
                          </div>
                          <div style={{ "font-size": "10px", color: "rgba(255, 255, 255, 0.25)", "margin-top": "2px" }}>
                            {model.description} — {model.size_mb}MB
                          </div>
                        </div>
                        {model.downloaded ? (
                          settings()!.model === model.filename ? (
                            <span style={{ "font-size": "10px", color: accent(), "font-weight": 600, "text-transform": "uppercase", "letter-spacing": "0.05em" }}>
                              Active
                            </span>
                          ) : (
                            <button
                              onClick={() => selectModel(model.filename)}
                              style={{
                                padding: "4px 10px",
                                background: "rgba(255, 255, 255, 0.04)",
                                border: "1px solid rgba(255, 255, 255, 0.06)",
                                "border-radius": "6px",
                                color: "rgba(255, 255, 255, 0.5)",
                                cursor: "pointer",
                                "font-size": "10px",
                              }}
                            >
                              Select
                            </button>
                          )
                        ) : (
                          <button
                            onClick={() => downloadModel(model.filename)}
                            disabled={downloadingModel() === model.filename}
                            style={{
                              padding: "4px 10px",
                              background: downloadingModel() === model.filename
                                ? "rgba(0, 0, 0, 0.2)"
                                : "rgba(255, 255, 255, 0.04)",
                              border: "1px solid rgba(255, 255, 255, 0.06)",
                              "border-radius": "6px",
                              color: downloadingModel() === model.filename
                                ? "rgba(255, 255, 255, 0.2)"
                                : accent(),
                              cursor: downloadingModel() === model.filename ? "wait" : "pointer",
                              "font-size": "10px",
                            }}
                          >
                            {downloadingModel() === model.filename ? "Downloading..." : "Download"}
                          </button>
                        )}
                      </div>
                    )}
                  </For>
                ) : (
                  <div style={{ color: "rgba(255, 255, 255, 0.25)", "font-size": "12px" }}>
                    Model: {settings()!.model}
                  </div>
                )}
              </div>
            </div>

            {/* Recording Mode */}
            <div style={glass}>
              <label style={label}>Recording Mode</label>
              <div style={{ display: "flex", gap: "6px" }}>
                {(["hold", "tap"] as const).map((mode) => (
                  <button
                    onClick={() => updateSetting("recordMode", mode)}
                    style={{
                      flex: 1,
                      padding: "8px 12px",
                      background: settings()!.recordMode === mode
                        ? `${accent()}18`
                        : "rgba(0, 0, 0, 0.3)",
                      border: settings()!.recordMode === mode
                        ? `1px solid ${accent()}44`
                        : "1px solid rgba(255, 255, 255, 0.04)",
                      "border-radius": "8px",
                      color: settings()!.recordMode === mode
                        ? accent()
                        : "rgba(255, 255, 255, 0.4)",
                      cursor: "pointer",
                      "font-size": "12px",
                      "font-weight": settings()!.recordMode === mode ? "600" : "400",
                      transition: "all 0.2s ease",
                    }}
                  >
                    {mode === "hold" ? "Hold to Record" : "Tap to Toggle"}
                  </button>
                ))}
              </div>
              {/* Auto-stop on silence (only meaningful in tap mode) */}
              <div style={{ "margin-top": "10px" }}>
                <SettingRow label="Auto-stop on Silence">
                  <Toggle
                    value={settings()!.autoStopSilence}
                    onChange={() => updateSetting("autoStopSilence", !settings()!.autoStopSilence)}
                    accent={accent()}
                  />
                </SettingRow>
                <div style={{ "font-size": "10px", color: "rgba(255,255,255,0.2)", "margin-top": "4px" }}>
                  Stops recording after ~2s of silence in tap mode
                </div>
              </div>
            </div>

            {/* General */}
            <div style={glass}>
              <label style={label}>General</label>
              <div style={{ display: "flex", "flex-direction": "column", gap: "12px" }}>
                <SettingRow label="Show Skin on Startup">
                  <Toggle
                    value={settings()!.showSkin}
                    onChange={() => updateSetting("showSkin", !settings()!.showSkin)}
                    accent={accent()}
                  />
                </SettingRow>
                <SettingRow label="Always on Top">
                  <Toggle
                    value={settings()!.alwaysOnTop}
                    onChange={() => updateSetting("alwaysOnTop", !settings()!.alwaysOnTop)}
                    accent={accent()}
                  />
                </SettingRow>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
