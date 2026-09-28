import { useEffect, useRef, useState } from "react";
import { commands, type Settings, type SoundChoice, type TrayIconStyle } from "../bindings";
import { parsePollIntervalMinutes } from "../lib/pollIntervalInput";
import "../styles/SettingsPage.css";

type SaveStatus = { ok: boolean; message: string } | null;

/** Matches `config::SoundChoice::default()` in `gui-tauri/src-tauri/src/config.rs`. */
const DEFAULT_SOUND: SoundChoice = "Default";

/** Matches `config::TrayIconStyle::default()` in `gui-tauri/src-tauri/src/config.rs`. */
const DEFAULT_TRAY_ICON_STYLE: TrayIconStyle = "Colored";

/** `gui/ui/main.slint`'s `notification-sound` `ComboBox` model, in the same order. */
const SOUND_CHOICES: { value: SoundChoice; label: string }[] = [
  { value: "Default", label: "Default sound" },
  { value: "Silent", label: "Silent" },
  { value: "DialogWarning", label: "Dialog warning" },
  { value: "MessageNewInstant", label: "Message" },
];

/** Bottom-right toast shown for a few seconds after a save succeeds or fails. */
function SaveToast({ status }: { status: SaveStatus }) {
  if (status === null) return null;
  return (
    <div className={`settings-page__toast${status.ok ? "" : " settings-page__toast--error"}`} role="status">
      {status.message}
    </div>
  );
}

export function SettingsPage() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [pollIntervalText, setPollIntervalText] = useState("5");
  const [savingPollInterval, setSavingPollInterval] = useState(false);
  const [savingTrayIcon, setSavingTrayIcon] = useState(false);
  const [savingPersistent, setSavingPersistent] = useState(false);
  const [savingSound, setSavingSound] = useState(false);
  const [previewing, setPreviewing] = useState(false);
  const [status, setStatus] = useState<SaveStatus>(null);
  const toastTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    let cancelled = false;
    commands
      .getSettings()
      .then((loaded) => {
        if (cancelled) return;
        setSettings(loaded);
        setPollIntervalText(String(loaded.poll_interval_minutes));
      })
      .catch((e) => {
        if (!cancelled) setLoadError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const timer = toastTimer;
    return () => {
      if (timer.current !== null) clearTimeout(timer.current);
    };
  }, []);

  const showToast = (next: SaveStatus) => {
    setStatus(next);
    if (toastTimer.current !== null) clearTimeout(toastTimer.current);
    toastTimer.current = setTimeout(() => setStatus(null), 5000);
  };

  const applySaveResult = (result: { status: "ok"; data: Settings } | { status: "error"; error: string }) => {
    if (result.status === "ok") {
      setSettings(result.data);
      showToast({ ok: true, message: "Saved." });
    } else {
      showToast({ ok: false, message: `Failed to save: ${result.error}` });
    }
  };

  const handleApplyPollInterval = async () => {
    const minutes = parsePollIntervalMinutes(pollIntervalText);
    if (minutes === null) {
      showToast({ ok: false, message: "Enter a whole number of minutes from 1 to 120." });
      return;
    }
    setSavingPollInterval(true);
    try {
      const result = await commands.savePollInterval(minutes);
      if (result.status === "ok") setPollIntervalText(String(result.data.poll_interval_minutes));
      applySaveResult(result);
    } catch (e) {
      showToast({ ok: false, message: `Failed to save: ${String(e)}` });
    } finally {
      setSavingPollInterval(false);
    }
  };

  const handleSetTrayIconStyle = async (monochrome: boolean) => {
    setSavingTrayIcon(true);
    try {
      applySaveResult(await commands.saveTrayIconStyle(monochrome));
    } catch (e) {
      showToast({ ok: false, message: `Failed to save: ${String(e)}` });
    } finally {
      setSavingTrayIcon(false);
    }
  };

  const handleTogglePersistent = async (checked: boolean) => {
    setSavingPersistent(true);
    try {
      applySaveResult(await commands.saveNotificationPersistent(checked));
    } catch (e) {
      showToast({ ok: false, message: `Failed to save: ${String(e)}` });
    } finally {
      setSavingPersistent(false);
    }
  };

  const handleChangeSound = async (sound: SoundChoice) => {
    setSavingSound(true);
    try {
      applySaveResult(await commands.saveNotificationSound(sound));
    } catch (e) {
      showToast({ ok: false, message: `Failed to save: ${String(e)}` });
    } finally {
      setSavingSound(false);
    }
  };

  const handlePreview = async () => {
    if (settings === null) return;
    setPreviewing(true);
    try {
      const result = await commands.previewNotificationSound(settings.notification_sound ?? DEFAULT_SOUND);
      if (result.status === "error") showToast({ ok: false, message: result.error });
    } catch (e) {
      showToast({ ok: false, message: String(e) });
    } finally {
      setPreviewing(false);
    }
  };

  if (loadError !== null) {
    return (
      <section className="settings-page">
        <h1>Settings</h1>
        <p className="settings-page__status settings-page__status--error" role="alert">
          Couldn't load settings: {loadError}
        </p>
      </section>
    );
  }

  if (settings === null) {
    return (
      <section className="settings-page">
        <h1>Settings</h1>
        <p className="settings-page__help">Loading…</p>
      </section>
    );
  }

  return (
    <section className="settings-page">
      <h1>Settings</h1>

      <div className="settings-page__section">
        <h2>Battery check interval</h2>
        <p className="settings-page__help">
          How often the system tray re-checks connected devices' battery levels.
        </p>
        <div className="settings-page__row">
          <input
            type="text"
            inputMode="numeric"
            aria-label="Battery check interval, minutes"
            value={pollIntervalText}
            onChange={(e) => setPollIntervalText(e.target.value)}
          />
          <span>minutes</span>
          <button onClick={() => void handleApplyPollInterval()} disabled={savingPollInterval}>
            {savingPollInterval ? "Applying…" : "Apply"}
          </button>
        </div>
      </div>

      <div className="settings-page__section">
        <h2>Tray icon</h2>
        <div className="settings-page__row">
          <button
            className={`settings-page__choice${
              (settings.tray_icon_style ?? DEFAULT_TRAY_ICON_STYLE) === "Colored" ? " settings-page__choice--active" : ""
            }`}
            disabled={savingTrayIcon}
            onClick={() => void handleSetTrayIconStyle(false)}
          >
            Colored
          </button>
          <button
            className={`settings-page__choice${
              (settings.tray_icon_style ?? DEFAULT_TRAY_ICON_STYLE) === "Monochrome" ? " settings-page__choice--active" : ""
            }`}
            disabled={savingTrayIcon}
            onClick={() => void handleSetTrayIconStyle(true)}
          >
            Monochrome
          </button>
        </div>
      </div>

      <div className="settings-page__section">
        <h2>Low-battery notification</h2>
        <div className="settings-page__row">
          <label>
            <input
              type="checkbox"
              checked={settings.notification_persistent}
              disabled={savingPersistent}
              onChange={(e) => void handleTogglePersistent(e.target.checked)}
            />
            Keep until manually dismissed
          </label>
        </div>
        <div className="settings-page__row">
          <select
            aria-label="Notification sound"
            value={settings.notification_sound ?? DEFAULT_SOUND}
            disabled={savingSound}
            onChange={(e) => void handleChangeSound(e.target.value as SoundChoice)}
          >
            {SOUND_CHOICES.map((choice) => (
              <option key={choice.value} value={choice.value}>
                {choice.label}
              </option>
            ))}
          </select>
          <button onClick={() => void handlePreview()} disabled={previewing}>
            {previewing ? "Playing…" : "▶ Preview"}
          </button>
        </div>
      </div>

      <SaveToast status={status} />
    </section>
  );
}
