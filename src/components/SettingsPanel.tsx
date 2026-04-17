import { createSignal, onMount, onCleanup, For, JSX } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { loadSettings, saveSetting, type MurmurSettings, type ModelInfo } from "../lib/settings";
import { initAuth, signIn, signOut, user, profile, isPro, authLoading } from "../lib/auth";
import logoImg from "../assets/logo.png";

// --- Terminal Cream Theme ---

const monoFont = "'JetBrains Mono', ui-monospace, Menlo, Consolas, monospace";

const glass: JSX.CSSProperties = {
  "margin-bottom": "16px",
  padding: "18px",
  background: "#ece4d0",
  "border-radius": "0",
  border: "1px solid #d4c9b5",
  transition: "border-color 0.2s ease",
};

const label: JSX.CSSProperties = {
  display: "block",
  "font-size": "11px",
  "font-weight": "700",
  "text-transform": "uppercase",
  "letter-spacing": "0.08em",
  color: "#c9482b",
  "margin-bottom": "10px",
};

const inputBase: JSX.CSSProperties = {
  width: "100%",
  padding: "8px 12px",
  background: "#f5f0e6",
  border: "1px solid #1a1a1a",
  "border-radius": "0",
  color: "#1a1a1a",
  "font-size": "13px",
  "font-family": monoFont,
  "box-sizing": "border-box",
  outline: "none",
};

function Toggle(props: { value: boolean; onChange: () => void; disabled?: boolean }) {
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

function SettingRow(props: { label: string; children: JSX.Element }) {
  return (
    <div
      style={{
        display: "flex",
        "align-items": "center",
        "justify-content": "space-between",
      }}
    >
      <span style={{ "font-size": "13px", color: "#6b655a" }}>{props.label}</span>
      {props.children}
    </div>
  );
}

const LANGUAGES = [
  { value: "en", label: "English" },
  { value: "auto", label: "Auto-detect" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "de", label: "German" },
  { value: "it", label: "Italian" },
  { value: "pt", label: "Portuguese" },
  { value: "ru", label: "Russian" },
  { value: "ja", label: "Japanese" },
  { value: "zh", label: "Chinese" },
  { value: "ko", label: "Korean" },
  { value: "ar", label: "Arabic" },
  { value: "hi", label: "Hindi" },
  { value: "nl", label: "Dutch" },
  { value: "pl", label: "Polish" },
  { value: "tr", label: "Turkish" },
  { value: "sv", label: "Swedish" },
  { value: "id", label: "Indonesian" },
  { value: "uk", label: "Ukrainian" },
];

function CustomSelect(props: { value: string; onChange: (v: string) => void; options: { value: string; label: string }[] }) {
  const [open, setOpen] = createSignal(false);
  let containerRef: HTMLDivElement | undefined;

  const selected = () => props.options.find((o) => o.value === props.value)?.label || props.value;

  const handleClickOutside = (e: MouseEvent) => {
    if (containerRef && !containerRef.contains(e.target as Node)) {
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
        <span style={{ color: "#6b655a", "font-size": "10px" }}>{open() ? "\u25B2" : "\u25BC"}</span>
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
          {props.options.map((opt) => (
            <div
              onClick={() => { props.onChange(opt.value); setOpen(false); }}
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
                if (opt.value !== props.value) e.currentTarget.style.background = "#ece4d0";
              }}
              onMouseLeave={(e) => {
                if (opt.value !== props.value) e.currentTarget.style.background = "transparent";
              }}
            >
              {opt.label}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function AccountSignIn() {
  const [email, setEmail] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [error, setError] = createSignal("");
  const [loading, setLoading] = createSignal(false);

  async function handleSignIn(e: Event) {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      const result = await signIn(email(), password());
      if (result.error) setError(result.error);
    } catch {
      setError("Connection failed. Check your internet and try again.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <form onSubmit={handleSignIn}>
      <input
        type="email"
        placeholder="Email"
        value={email()}
        onInput={(e) => setEmail(e.currentTarget.value)}
        required
        style={{ ...inputBase, "margin-bottom": "8px" }}
        onFocus={(e) => {
          e.currentTarget.style.borderColor = "#c9482b";
          e.currentTarget.style.boxShadow = "3px 3px 0 #c9482b";
        }}
        onBlur={(e) => {
          e.currentTarget.style.borderColor = "#1a1a1a";
          e.currentTarget.style.boxShadow = "none";
        }}
      />
      <input
        type="password"
        placeholder="Password"
        value={password()}
        onInput={(e) => setPassword(e.currentTarget.value)}
        required
        style={{ ...inputBase, "margin-bottom": "8px" }}
        onFocus={(e) => {
          e.currentTarget.style.borderColor = "#c9482b";
          e.currentTarget.style.boxShadow = "3px 3px 0 #c9482b";
        }}
        onBlur={(e) => {
          e.currentTarget.style.borderColor = "#1a1a1a";
          e.currentTarget.style.boxShadow = "none";
        }}
      />
      {error() && (
        <p style={{ color: "#a33a2a", "font-size": "12px", "margin-bottom": "8px" }}>
          {error()}
        </p>
      )}
      <button
        type="submit"
        disabled={loading()}
        style={{
          width: "100%",
          padding: "8px",
          background: "#c9482b",
          border: "1px solid #c9482b",
          "border-radius": "0",
          color: "#fff8ed",
          "font-size": "13px",
          "font-weight": "500",
          "font-family": monoFont,
          cursor: loading() ? "wait" : "pointer",
          opacity: loading() ? "0.5" : "1",
          transition: "all 0.15s ease",
        }}
        onMouseEnter={(e) => {
          if (!loading()) {
            e.currentTarget.style.background = "#1a1a1a";
            e.currentTarget.style.borderColor = "#1a1a1a";
            e.currentTarget.style.transform = "translate(-2px, -2px)";
            e.currentTarget.style.boxShadow = "4px 4px 0 #c9482b";
          }
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.background = "#c9482b";
          e.currentTarget.style.borderColor = "#c9482b";
          e.currentTarget.style.transform = "none";
          e.currentTarget.style.boxShadow = "none";
        }}
      >
        {loading() ? "Signing in..." : "Sign in"}
      </button>
      <p style={{ color: "#6b655a", "font-size": "11px", "margin-top": "8px", "text-align": "center" }}>
        Create an account at murmurlinux.com
      </p>
    </form>
  );
}

// --- Component ---

export function SettingsPanel() {
  const [settings, setSettings] = createSignal<MurmurSettings | null>(null);
  const [capturingHotkey, setCapturingHotkey] = createSignal(false);
  const [models, setModels] = createSignal<ModelInfo[]>([]);
  const [downloadingModel, setDownloadingModel] = createSignal<string | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [version, setVersion] = createSignal("...");

  const showError = (msg: string) => {
    setError(msg);
    setTimeout(() => setError(null), 5000);
  };

  onMount(async () => {
    await initAuth();

    // Read version from Tauri config (not hardcoded)
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      setVersion(await getVersion());
    } catch { /* fallback stays as "..." */ }

    const s = await loadSettings();
    setSettings(s);

    try {
      const list = await invoke<ModelInfo[]>("list_models");
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
      const list = await invoke<ModelInfo[]>("list_models");
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
        background: "#f5f0e6",
        "background-image": "radial-gradient(circle at 1px 1px, rgba(26,26,26,0.06) 1px, transparent 0)",
        "background-size": "14px 14px",
        color: "#1a1a1a",
        width: "100%",
        "min-height": "100vh",
        padding: "20px",
        "box-sizing": "border-box",
        "font-family": monoFont,
        position: "relative",
      }}
    >
      <div style={{ position: "relative", "z-index": 1 }}>
        {/* Header */}
        <div
          style={{
            display: "flex",
            "align-items": "center",
            "justify-content": "space-between",
            "margin-bottom": "20px",
          }}
        >
          <div style={{ display: "flex", "align-items": "center", gap: "6px" }}>
            <img src={logoImg} alt="Murmur" width={48} height={48} style={{ "border-radius": "0" }} />
            <pre
              style={{
                color: "#c9482b",
                "font-size": "10px",
                "line-height": "1.0",
                margin: "-10px 0 0 0",
                "white-space": "pre",
                "font-weight": "700",
                "font-family": monoFont,
              }}
            >{` __  __\n|  \\/  |_   _ _ __ _ __ ___  _   _ _ __\n| |\\/| | | | | '__| '_ \` _ \\| | | | '__|\n| |  | | |_| | |  | | | | | | |_| | |\n|_|  |_|\\__,_|_|  |_| |_| |_|\\__,_|_|`}</pre>
            <span style={{ "font-size": "10px", color: "#6b655a", "font-family": monoFont, "align-self": "flex-end", "margin-bottom": "2px" }}>
              v{version()}
            </span>
          </div>
        </div>

        {error() && (
          <div
            style={{
              padding: "10px 14px",
              background: "#ece4d0",
              border: "1px solid #a33a2a",
              "border-radius": "0",
              color: "#a33a2a",
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
            {/* Account */}
            <div style={glass}>
              <span style={label}>Account</span>
              {authLoading() ? (
                <p style={{ color: "#6b655a", "font-size": "13px" }}>Loading...</p>
              ) : user() ? (
                <div>
                  <p style={{ color: "#1a1a1a", "font-size": "13px", "margin-bottom": "8px" }}>
                    {profile()?.email ?? user()?.email}
                  </p>
                  <p style={{
                    color: isPro() ? "#5a7a3a" : "#6b655a",
                    "font-size": "11px",
                    "font-weight": "600",
                    "text-transform": "uppercase",
                    "letter-spacing": "0.05em",
                    "margin-bottom": "12px",
                  }}>
                    {isPro() ? "Pro" : "Free"}
                  </p>
                  <button
                    onClick={() => signOut()}
                    style={{
                      padding: "6px 16px",
                      background: "#ece4d0",
                      border: "1px solid #1a1a1a",
                      "border-radius": "0",
                      color: "#1a1a1a",
                      "font-size": "12px",
                      "font-family": monoFont,
                      cursor: "pointer",
                      transition: "all 0.15s ease",
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.background = "#1a1a1a";
                      e.currentTarget.style.color = "#f5f0e6";
                      e.currentTarget.style.transform = "translate(-2px, -2px)";
                      e.currentTarget.style.boxShadow = "4px 4px 0 #c9482b";
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.background = "#ece4d0";
                      e.currentTarget.style.color = "#1a1a1a";
                      e.currentTarget.style.transform = "none";
                      e.currentTarget.style.boxShadow = "none";
                    }}
                  >
                    Sign out
                  </button>
                </div>
              ) : (
                <AccountSignIn />
              )}
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
                    "border-color": capturingHotkey() ? "#c9482b" : "#1a1a1a",
                    "box-shadow": capturingHotkey() ? "3px 3px 0 #c9482b" : "none",
                    "text-align": "center",
                    "user-select": "none",
                  }}
                >
                  {capturingHotkey() ? (
                    <span style={{ color: "#6b655a", "font-style": "italic", "font-family": monoFont }}>
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
                    background: "#ece4d0",
                    border: "1px solid #1a1a1a",
                    "border-radius": "0",
                    color: "#1a1a1a",
                    cursor: "pointer",
                    "font-size": "11px",
                    "font-family": monoFont,
                    "white-space": "nowrap",
                    transition: "all 0.15s ease",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = "#1a1a1a";
                    e.currentTarget.style.color = "#f5f0e6";
                    e.currentTarget.style.transform = "translate(-2px, -2px)";
                    e.currentTarget.style.boxShadow = "4px 4px 0 #c9482b";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = "#ece4d0";
                    e.currentTarget.style.color = "#1a1a1a";
                    e.currentTarget.style.transform = "none";
                    e.currentTarget.style.boxShadow = "none";
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
                            ? "#f5f0e6"
                            : "#ece4d0",
                          border: settings()!.model === model.filename
                            ? "1px solid #c9482b"
                            : "1px solid #d4c9b5",
                          "border-radius": "0",
                          transition: "all 0.2s ease",
                        }}
                      >
                        <div style={{ flex: 1 }}>
                          <div style={{ "font-size": "13px", "font-weight": 500, color: "#1a1a1a" }}>
                            {model.name}
                          </div>
                          <div style={{ "font-size": "10px", color: "#6b655a", "margin-top": "2px" }}>
                            {model.description} -- {model.size_mb}MB
                          </div>
                        </div>
                        {model.downloaded ? (
                          settings()!.model === model.filename ? (
                            <span style={{ "font-size": "10px", color: "#c9482b", "font-weight": 700, "text-transform": "uppercase", "letter-spacing": "0.05em", "width": "100px", "text-align": "center", display: "inline-block" }}>
                              active
                            </span>
                          ) : (
                            <button
                              onClick={() => selectModel(model.filename)}
                              style={{
                                padding: "4px 10px",
                                "width": "100px",
                                "text-align": "center",
                                background: "#ece4d0",
                                border: "1px solid #1a1a1a",
                                "border-radius": "0",
                                color: "#1a1a1a",
                                cursor: "pointer",
                                "font-size": "10px",
                                "font-family": monoFont,
                                transition: "all 0.15s ease",
                              }}
                              onMouseEnter={(e) => {
                                e.currentTarget.style.background = "#1a1a1a";
                                e.currentTarget.style.color = "#f5f0e6";
                                e.currentTarget.style.transform = "translate(-2px, -2px)";
                                e.currentTarget.style.boxShadow = "4px 4px 0 #c9482b";
                              }}
                              onMouseLeave={(e) => {
                                e.currentTarget.style.background = "#ece4d0";
                                e.currentTarget.style.color = "#1a1a1a";
                                e.currentTarget.style.transform = "none";
                                e.currentTarget.style.boxShadow = "none";
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
                              "width": "100px",
                              "text-align": "center",
                              background: downloadingModel() === model.filename
                                ? "#d4c9b5"
                                : "#ece4d0",
                              border: "1px solid #1a1a1a",
                              "border-radius": "0",
                              color: downloadingModel() === model.filename
                                ? "#6b655a"
                                : "#c9482b",
                              cursor: downloadingModel() === model.filename ? "wait" : "pointer",
                              "font-size": "10px",
                              "font-family": monoFont,
                              transition: "all 0.15s ease",
                            }}
                            onMouseEnter={(e) => {
                              if (downloadingModel() !== model.filename) {
                                e.currentTarget.style.background = "#1a1a1a";
                                e.currentTarget.style.color = "#f5f0e6";
                                e.currentTarget.style.transform = "translate(-2px, -2px)";
                                e.currentTarget.style.boxShadow = "4px 4px 0 #c9482b";
                              }
                            }}
                            onMouseLeave={(e) => {
                              if (downloadingModel() !== model.filename) {
                                e.currentTarget.style.background = "#ece4d0";
                                e.currentTarget.style.color = "#c9482b";
                                e.currentTarget.style.transform = "none";
                                e.currentTarget.style.boxShadow = "none";
                              }
                            }}
                          >
                            {downloadingModel() === model.filename ? "Downloading..." : "Download"}
                          </button>
                        )}
                      </div>
                    )}
                  </For>
                ) : (
                  <div style={{ color: "#6b655a", "font-size": "12px" }}>
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
                        ? "#f5f0e6"
                        : "#ece4d0",
                      border: settings()!.recordMode === mode
                        ? "1px solid #c9482b"
                        : "1px solid #d4c9b5",
                      "border-radius": "0",
                      color: settings()!.recordMode === mode
                        ? "#c9482b"
                        : "#6b655a",
                      cursor: "pointer",
                      "font-size": "12px",
                      "font-family": monoFont,
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
                    disabled={settings()!.recordMode === "hold"}
                  />
                </SettingRow>
                <div style={{ "font-size": "10px", color: "#6b655a", "margin-top": "4px" }}>
                  {settings()!.recordMode === "hold"
                    ? "Not applicable in hold mode (release to stop)"
                    : "Stops recording after ~2s of silence in tap mode"}
                </div>
              </div>
            </div>

            {/* Language */}
            <div style={glass}>
              <label style={label}>Language</label>
              <div style={{ display: "flex", "flex-direction": "column", gap: "10px" }}>
                <CustomSelect
                  value={settings()!.language}
                  onChange={(v) => updateSetting("language", v)}
                  options={LANGUAGES}
                />
                {settings()!.language !== "en" && (
                  <SettingRow label="Translate to English">
                    <Toggle
                      value={settings()!.translateToEnglish}
                      onChange={() => updateSetting("translateToEnglish", !settings()!.translateToEnglish)}
                    />
                  </SettingRow>
                )}
                {settings()!.language !== "en" && settings()!.model.includes(".en.") && (
                  <div style={{
                    "font-size": "11px",
                    color: "#c9482b",
                    padding: "8px 10px",
                    background: "#f5f0e6",
                    "border-radius": "0",
                    border: "1px solid #c9482b",
                  }}>
                    Your current model is English-only. Download a multilingual model above for best results.
                  </div>
                )}
              </div>
            </div>

            {/* General */}
            <div style={glass}>
              <label style={label}>General</label>
              <div style={{ display: "flex", "flex-direction": "column", gap: "12px" }}>
                <SettingRow label="Start on Login">
                  <Toggle
                    value={settings()!.startOnLogin}
                    onChange={async () => {
                      const next = !settings()!.startOnLogin;
                      await invoke("set_start_on_login", { enabled: next });
                      updateSetting("startOnLogin", next);
                    }}
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
