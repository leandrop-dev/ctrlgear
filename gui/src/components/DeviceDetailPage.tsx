import { useEffect, useRef, useState } from "react";
import { commands, type ButtonActionChoice, type ChatMixStatus, type DeviceSummary } from "../bindings";
import { batteryText, connectionLabel, displayName, dpiHelpText, productIdLabel } from "../lib/deviceLabels";
import { parseDpiInput } from "../lib/dpiInput";
import { parseSleepTimerMinutes } from "../lib/sleepTimerInput";
import "../styles/DeviceDetailPage.css";

type ApplyStatus = { ok: boolean; message: string } | null;

/** Default button-mapping keyword for a slot before the user changes it. */
const DEFAULT_BUTTON_ACTION = "default";

function StatusLine({ status }: { status: ApplyStatus }) {
  if (status === null) return null;
  return (
    <p className={`device-detail-page__status${status.ok ? "" : " device-detail-page__status--error"}`}>
      {status.message}
    </p>
  );
}

function DpiSection({ device }: { device: DeviceSummary }) {
  const [input, setInput] = useState("400,800,1600");
  const [applying, setApplying] = useState(false);
  const [status, setStatus] = useState<ApplyStatus>(null);

  useEffect(() => {
    setInput("400,800,1600");
    setStatus(null);
  }, [device.productId]);

  const handleApply = async () => {
    const parsed = parseDpiInput(input);
    if ("error" in parsed) {
      setStatus({ ok: false, message: parsed.error });
      return;
    }

    setApplying(true);
    try {
      const result = await commands.setDpi(device.productId, parsed.values);
      if (result.status === "ok") {
        setStatus({ ok: true, message: `DPI set to ${parsed.values.join(", ")}` });
      } else {
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
    } finally {
      setApplying(false);
    }
  };

  const help = dpiHelpText(device.dpiRange, device.dpiMaxPresets);

  return (
    <section className="device-detail-page__section">
      <h2>DPI presets</h2>
      <div className="device-detail-page__row">
        <input
          type="text"
          aria-label="DPI presets, comma-separated"
          value={input}
          placeholder="400,800,1600"
          onChange={(e) => setInput(e.target.value)}
        />
        <button onClick={handleApply} disabled={applying}>
          {applying ? "Applying…" : "Apply"}
        </button>
      </div>
      {help !== "" && <p className="device-detail-page__help">{help}</p>}
      <StatusLine status={status} />
    </section>
  );
}

function PollingRateSection({ device }: { device: DeviceSummary }) {
  const choices = device.pollingRateChoices;
  const highest = choices.length > 0 ? Math.max(...choices) : undefined;
  const [hz, setHz] = useState<number | undefined>(highest);
  const [applying, setApplying] = useState(false);
  const [status, setStatus] = useState<ApplyStatus>(null);

  useEffect(() => {
    setHz(highest);
    setStatus(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [device.productId]);

  const handleApply = async () => {
    if (hz === undefined) return;
    setApplying(true);
    try {
      const result = await commands.setPollingRate(device.productId, hz);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "Polling rate applied." });
      } else {
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
    } finally {
      setApplying(false);
    }
  };

  return (
    <section className="device-detail-page__section">
      <h2>Polling rate</h2>
      <div className="device-detail-page__row">
        <select aria-label="Polling rate" value={hz ?? ""} onChange={(e) => setHz(Number(e.target.value))}>
          {choices.map((choice) => (
            <option key={choice} value={choice}>
              {choice} Hz
            </option>
          ))}
        </select>
        <button onClick={handleApply} disabled={applying || hz === undefined}>
          {applying ? "Applying…" : "Apply"}
        </button>
      </div>
      <StatusLine status={status} />
    </section>
  );
}

function ButtonMappingSection({ device }: { device: DeviceSummary }) {
  const [choices, setChoices] = useState<ButtonActionChoice[]>([]);
  const [slotLabels, setSlotLabels] = useState<string[]>([]);
  const [selections, setSelections] = useState<string[]>([]);
  const [applying, setApplying] = useState(false);
  const [status, setStatus] = useState<ApplyStatus>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    Promise.all([commands.buttonActionChoices(), commands.buttonSlotLabels()])
      .then(([loadedChoices, loadedLabels]) => {
        if (cancelled) return;
        setChoices(loadedChoices);
        setSlotLabels(loadedLabels);
        setSelections(loadedLabels.map(() => DEFAULT_BUTTON_ACTION));
      })
      .catch((e) => {
        if (!cancelled) setLoadError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    setStatus(null);
    setSelections((prev) => (slotLabels.length > 0 ? slotLabels.map(() => DEFAULT_BUTTON_ACTION) : prev));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [device.productId]);

  const handleApply = async () => {
    setApplying(true);
    try {
      const result = await commands.setButtonMapping(device.productId, selections);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "Button mapping applied." });
      } else {
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
    } finally {
      setApplying(false);
    }
  };

  if (loadError !== null) {
    return (
      <section className="device-detail-page__section">
        <h2>Button mapping</h2>
        <p className="device-detail-page__status device-detail-page__status--error">{loadError}</p>
      </section>
    );
  }

  return (
    <section className="device-detail-page__section">
      <h2>Button mapping</h2>
      <p className="device-detail-page__help">Each row is one physical button on the mouse; pick what it should do.</p>
      {slotLabels.map((label, i) => (
        <div className="device-detail-page__row" key={label}>
          <label className="device-detail-page__row-label" htmlFor={`button-slot-${i}`}>
            {label}
          </label>
          <select
            id={`button-slot-${i}`}
            value={selections[i] ?? DEFAULT_BUTTON_ACTION}
            onChange={(e) => {
              const next = [...selections];
              next[i] = e.target.value;
              setSelections(next);
            }}
          >
            {choices.map((choice) => (
              <option key={choice.keyword} value={choice.keyword}>
                {choice.label}
              </option>
            ))}
          </select>
        </div>
      ))}
      <button onClick={handleApply} disabled={applying || slotLabels.length === 0}>
        {applying ? "Applying…" : "Apply"}
      </button>
      <StatusLine status={status} />
    </section>
  );
}

/** Zone-color placeholder defaults for the RGB section, by zone index. */
const RGB_ZONE_DEFAULT_COLORS = ["#ff0000", "#00ff00", "#0000ff"];
const RGB_ZONE_DEBOUNCE_MS = 150;
/** Explanatory message shown when the rainbow checkbox is unchecked. */
const RGB_RAINBOW_OFF_MESSAGE =
  'Rainbow has no direct "off" command — set a zone color or default lighting to turn it off on the device.';

function RgbSection({ device }: { device: DeviceSummary }) {
  const zoneCount = device.rgbZoneCount;
  const [zoneColors, setZoneColors] = useState<string[]>(() =>
    Array.from({ length: zoneCount }, (_, i) => RGB_ZONE_DEFAULT_COLORS[i] ?? "#ffffff"),
  );
  const [brightness, setBrightness] = useState(100);
  const [brightnessApplying, setBrightnessApplying] = useState(false);
  const [rainbowEnabled, setRainbowEnabled] = useState(false);
  const [defaultLighting, setDefaultLighting] = useState(device.rgbDefaultLightingChoices[0] ?? "");
  const [defaultLightingApplying, setDefaultLightingApplying] = useState(false);
  const [status, setStatus] = useState<ApplyStatus>(null);
  const zoneTimers = useRef<Record<number, ReturnType<typeof setTimeout>>>({});
  const zoneWrites = useRef<Record<number, Promise<void>>>({});

  useEffect(() => {
    setZoneColors(Array.from({ length: zoneCount }, (_, i) => RGB_ZONE_DEFAULT_COLORS[i] ?? "#ffffff"));
    setBrightness(100);
    setRainbowEnabled(false);
    setDefaultLighting(device.rgbDefaultLightingChoices[0] ?? "");
    setStatus(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [device.productId]);

  useEffect(() => {
    const timers = zoneTimers.current;
    return () => {
      for (const timer of Object.values(timers)) {
        clearTimeout(timer);
      }
    };
  }, []);

  const applyZoneColor = async (zone: number, hexColor: string) => {
    try {
      const result = await commands.setRgbZone(device.productId, zone, hexColor);
      if (result.status === "ok") {
        setStatus({ ok: true, message: `Zone ${zone} color applied.` });
      } else {
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
    }
  };

  const handleZoneColorChange = (zoneIndex: number, hexColor: string) => {
    const zone = zoneIndex + 1;
    setZoneColors((prev) => {
      const next = [...prev];
      next[zoneIndex] = hexColor;
      return next;
    });

    const existing = zoneTimers.current[zone];
    if (existing !== undefined) clearTimeout(existing);
    zoneTimers.current[zone] = setTimeout(() => {
      delete zoneTimers.current[zone];
      // Chain onto the zone's previous write so two in-flight writes for the
      // same zone reach the device (and the status line) in dispatch order —
      // HID_BUS_LOCK serializes them but doesn't guarantee FIFO.
      const previous = zoneWrites.current[zone] ?? Promise.resolve();
      zoneWrites.current[zone] = previous.then(() => applyZoneColor(zone, hexColor));
    }, RGB_ZONE_DEBOUNCE_MS);
  };

  const handleApplyBrightness = async () => {
    setBrightnessApplying(true);
    try {
      const result = await commands.setRgbBrightness(device.productId, brightness);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "LED brightness applied." });
      } else {
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
    } finally {
      setBrightnessApplying(false);
    }
  };

  const handleToggleRainbow = async (checked: boolean) => {
    setRainbowEnabled(checked);
    if (!checked) {
      setStatus({ ok: true, message: RGB_RAINBOW_OFF_MESSAGE });
      return;
    }

    try {
      const result = await commands.setRgbRainbow(device.productId);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "Rainbow effect applied." });
      } else {
        setRainbowEnabled(false);
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setRainbowEnabled(false);
      setStatus({ ok: false, message: String(e) });
    }
  };

  const handleApplyDefaultLighting = async () => {
    if (defaultLighting === "") return;
    setDefaultLightingApplying(true);
    try {
      const result = await commands.setRgbDefaultLighting(device.productId, defaultLighting);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "Default lighting applied." });
      } else {
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
    } finally {
      setDefaultLightingApplying(false);
    }
  };

  return (
    <section className="device-detail-page__section">
      <h2>RGB lighting</h2>
      {zoneColors.map((color, i) => {
        const zone = i + 1;
        return (
          <div className="device-detail-page__row" key={zone}>
            <label className="device-detail-page__row-label" htmlFor={`rgb-zone-${zone}`}>
              Zone {zone}
            </label>
            <input
              id={`rgb-zone-${zone}`}
              type="color"
              aria-label={`Zone ${zone} color`}
              value={color}
              onChange={(e) => handleZoneColorChange(i, e.target.value)}
            />
          </div>
        );
      })}

      {device.supportsRgbBrightness && (
        <div className="device-detail-page__row">
          <label className="device-detail-page__row-label" htmlFor="rgb-brightness">
            Brightness
          </label>
          <input
            id="rgb-brightness"
            type="range"
            min={0}
            max={100}
            value={brightness}
            aria-label="LED brightness"
            onChange={(e) => setBrightness(Number(e.target.value))}
          />
          <button onClick={handleApplyBrightness} disabled={brightnessApplying}>
            {brightnessApplying ? "Applying…" : "Apply"}
          </button>
        </div>
      )}

      {device.supportsRgbRainbow && (
        <div className="device-detail-page__row">
          <label className="device-detail-page__row-label" htmlFor="rgb-rainbow">
            Rainbow effect
          </label>
          <input
            id="rgb-rainbow"
            type="checkbox"
            checked={rainbowEnabled}
            onChange={(e) => void handleToggleRainbow(e.target.checked)}
          />
        </div>
      )}

      {device.rgbDefaultLightingChoices.length > 0 && (
        <div className="device-detail-page__row">
          <label className="device-detail-page__row-label" htmlFor="rgb-default-lighting">
            Default lighting
          </label>
          <select
            id="rgb-default-lighting"
            aria-label="Default lighting"
            value={defaultLighting}
            onChange={(e) => setDefaultLighting(e.target.value)}
          >
            {device.rgbDefaultLightingChoices.map((choice) => (
              <option key={choice} value={choice}>
                {choice}
              </option>
            ))}
          </select>
          <button onClick={handleApplyDefaultLighting} disabled={defaultLightingApplying || defaultLighting === ""}>
            {defaultLightingApplying ? "Applying…" : "Apply"}
          </button>
        </div>
      )}

      <StatusLine status={status} />
    </section>
  );
}

/** Sidetone and mic-mute-LED choices (identical 4-level scale). */
const LEVEL_0_TO_3_CHOICES = ["Off", "Low", "Medium", "High"];
/** Mic-volume choices (0-7, off to max). */
const MIC_VOLUME_CHOICES = ["0", "1", "2", "3", "4", "5", "6", "7"];
/** Equalizer preset buttons, in `EQ_PRESETS` index order. */
const EQUALIZER_PRESETS = ["Flat", "Bass", "Focus", "Smiley"];
/** Bluetooth call-volume choices. */
const BLUETOOTH_CALL_VOLUME_CHOICES = ["Off", "Lower game volume", "Mute game during calls"];

function SidetoneSection({ device }: { device: DeviceSummary }) {
  const [level, setLevel] = useState(0);
  const [applying, setApplying] = useState(false);
  const [status, setStatus] = useState<ApplyStatus>(null);

  useEffect(() => {
    setLevel(0);
    setStatus(null);
  }, [device.productId]);

  const handleApply = async () => {
    setApplying(true);
    try {
      const result = await commands.setSidetone(device.productId, level);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "Sidetone applied." });
      } else {
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
    } finally {
      setApplying(false);
    }
  };

  return (
    <section className="device-detail-page__section">
      <h2>Sidetone</h2>
      <div className="device-detail-page__row">
        <select aria-label="Sidetone" value={level} onChange={(e) => setLevel(Number(e.target.value))}>
          {LEVEL_0_TO_3_CHOICES.map((label, i) => (
            <option key={label} value={i}>
              {label}
            </option>
          ))}
        </select>
        <button onClick={handleApply} disabled={applying}>
          {applying ? "Applying…" : "Apply"}
        </button>
      </div>
      <StatusLine status={status} />
    </section>
  );
}

function MicMuteLedSection({ device }: { device: DeviceSummary }) {
  const [level, setLevel] = useState(0);
  const [applying, setApplying] = useState(false);
  const [status, setStatus] = useState<ApplyStatus>(null);

  useEffect(() => {
    setLevel(0);
    setStatus(null);
  }, [device.productId]);

  const handleApply = async () => {
    setApplying(true);
    try {
      const result = await commands.setMicMuteLed(device.productId, level);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "Mic mute LED brightness applied." });
      } else {
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
    } finally {
      setApplying(false);
    }
  };

  return (
    <section className="device-detail-page__section">
      <h2>Mic mute LED brightness</h2>
      <div className="device-detail-page__row">
        <select aria-label="Mic mute LED brightness" value={level} onChange={(e) => setLevel(Number(e.target.value))}>
          {LEVEL_0_TO_3_CHOICES.map((label, i) => (
            <option key={label} value={i}>
              {label}
            </option>
          ))}
        </select>
        <button onClick={handleApply} disabled={applying}>
          {applying ? "Applying…" : "Apply"}
        </button>
      </div>
      <StatusLine status={status} />
    </section>
  );
}

function MicVolumeSection({ device }: { device: DeviceSummary }) {
  const [level, setLevel] = useState(0);
  const [applying, setApplying] = useState(false);
  const [status, setStatus] = useState<ApplyStatus>(null);

  useEffect(() => {
    setLevel(0);
    setStatus(null);
  }, [device.productId]);

  const handleApply = async () => {
    setApplying(true);
    try {
      const result = await commands.setMicVolume(device.productId, level);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "Mic volume applied." });
      } else {
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
    } finally {
      setApplying(false);
    }
  };

  return (
    <section className="device-detail-page__section">
      <h2>Mic volume</h2>
      <div className="device-detail-page__row">
        <select aria-label="Mic volume" value={level} onChange={(e) => setLevel(Number(e.target.value))}>
          {MIC_VOLUME_CHOICES.map((label, i) => (
            <option key={label} value={i}>
              {label}
            </option>
          ))}
        </select>
        <button onClick={handleApply} disabled={applying}>
          {applying ? "Applying…" : "Apply"}
        </button>
      </div>
      <StatusLine status={status} />
    </section>
  );
}

function EqualizerSection({ device }: { device: DeviceSummary }) {
  const [applyingPreset, setApplyingPreset] = useState<number | null>(null);
  const [status, setStatus] = useState<ApplyStatus>(null);

  useEffect(() => {
    setStatus(null);
  }, [device.productId]);

  const handleApply = async (preset: number) => {
    setApplyingPreset(preset);
    try {
      const result = await commands.setEqualizerPreset(device.productId, preset);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "Equalizer preset applied." });
      } else {
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
    } finally {
      setApplyingPreset(null);
    }
  };

  return (
    <section className="device-detail-page__section">
      <h2>Equalizer presets</h2>
      <div className="device-detail-page__row">
        {EQUALIZER_PRESETS.map((label, i) => (
          <button key={label} onClick={() => handleApply(i)} disabled={applyingPreset !== null}>
            {applyingPreset === i ? "Applying…" : label}
          </button>
        ))}
      </div>
      <p className="device-detail-page__help">
        Applies on the headset itself, independent of any PipeWire-side equalizer (the two stack rather than
        conflict).
      </p>
      <StatusLine status={status} />
    </section>
  );
}

function SleepTimerSection({ device }: { device: DeviceSummary }) {
  const [minutesText, setMinutesText] = useState("0");
  const [applying, setApplying] = useState(false);
  const [status, setStatus] = useState<ApplyStatus>(null);

  useEffect(() => {
    setMinutesText("0");
    setStatus(null);
  }, [device.productId]);

  const handleApply = async () => {
    const minutes = parseSleepTimerMinutes(minutesText);
    if (minutes === null) {
      setStatus({ ok: false, message: "Enter a whole number of minutes from 0 to 255 (0 disables the timer)." });
      return;
    }
    setApplying(true);
    try {
      const result = await commands.setSleepTimer(device.productId, minutes);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "Sleep timer applied." });
      } else {
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
    } finally {
      setApplying(false);
    }
  };

  return (
    <section className="device-detail-page__section">
      <h2>Sleep timer</h2>
      <p className="device-detail-page__help">Minutes of inactivity before the headset sleeps.</p>
      <div className="device-detail-page__row">
        <input
          type="number"
          aria-label="Sleep timer minutes"
          min={0}
          max={255}
          step={1}
          value={minutesText}
          onChange={(e) => setMinutesText(e.target.value)}
        />
        <span>minutes</span>
        <button onClick={handleApply} disabled={applying}>
          {applying ? "Applying…" : "Apply"}
        </button>
      </div>
      <StatusLine status={status} />
    </section>
  );
}

function VolumeLimiterSection({ device }: { device: DeviceSummary }) {
  const [enabled, setEnabled] = useState(false);
  const [status, setStatus] = useState<ApplyStatus>(null);

  useEffect(() => {
    setEnabled(false);
    setStatus(null);
  }, [device.productId]);

  const handleToggle = async (checked: boolean) => {
    setEnabled(checked);
    try {
      const result = await commands.setVolumeLimiter(device.productId, checked);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "Volume limiter applied." });
      } else {
        setEnabled(!checked);
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setEnabled(!checked);
      setStatus({ ok: false, message: String(e) });
    }
  };

  return (
    <section className="device-detail-page__section">
      <h2>Volume limiter</h2>
      <div className="device-detail-page__row">
        <label className="device-detail-page__row-label" htmlFor="volume-limiter">
          Enabled
        </label>
        <input
          id="volume-limiter"
          type="checkbox"
          checked={enabled}
          onChange={(e) => void handleToggle(e.target.checked)}
        />
      </div>
      <StatusLine status={status} />
    </section>
  );
}

function BluetoothSection({ device }: { device: DeviceSummary }) {
  const [powerOnEnabled, setPowerOnEnabled] = useState(false);
  const [callVolumeMode, setCallVolumeMode] = useState(0);
  const [callVolumeApplying, setCallVolumeApplying] = useState(false);
  const [status, setStatus] = useState<ApplyStatus>(null);

  useEffect(() => {
    setPowerOnEnabled(false);
    setCallVolumeMode(0);
    setStatus(null);
  }, [device.productId]);

  const handleTogglePowerOn = async (checked: boolean) => {
    setPowerOnEnabled(checked);
    try {
      const result = await commands.setBluetoothPowerOn(device.productId, checked);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "Bluetooth-when-powered-on applied." });
      } else {
        setPowerOnEnabled(!checked);
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setPowerOnEnabled(!checked);
      setStatus({ ok: false, message: String(e) });
    }
  };

  const handleApplyCallVolume = async () => {
    setCallVolumeApplying(true);
    try {
      const result = await commands.setBluetoothCallVolume(device.productId, callVolumeMode);
      if (result.status === "ok") {
        setStatus({ ok: true, message: "Bluetooth call volume applied." });
      } else {
        setStatus({ ok: false, message: result.error });
      }
    } catch (e) {
      setStatus({ ok: false, message: String(e) });
    } finally {
      setCallVolumeApplying(false);
    }
  };

  return (
    <section className="device-detail-page__section">
      <h2>Bluetooth</h2>
      <div className="device-detail-page__row">
        <label className="device-detail-page__row-label" htmlFor="bluetooth-power-on">
          Turn on Bluetooth when the headset powers on
        </label>
        <input
          id="bluetooth-power-on"
          type="checkbox"
          checked={powerOnEnabled}
          onChange={(e) => void handleTogglePowerOn(e.target.checked)}
        />
      </div>
      <h2>Call volume</h2>
      <div className="device-detail-page__row">
        <select
          aria-label="Bluetooth call volume"
          value={callVolumeMode}
          onChange={(e) => setCallVolumeMode(Number(e.target.value))}
        >
          {BLUETOOTH_CALL_VOLUME_CHOICES.map((label, i) => (
            <option key={label} value={i}>
              {label}
            </option>
          ))}
        </select>
        <button onClick={handleApplyCallVolume} disabled={callVolumeApplying}>
          {callVolumeApplying ? "Applying…" : "Apply"}
        </button>
      </div>
      <StatusLine status={status} />
    </section>
  );
}

function ChatMixSection({ chatmix }: { chatmix: ChatMixStatus }) {
  return (
    <section className="device-detail-page__section">
      <h2>ChatMix</h2>
      <p className="device-detail-page__help">
        Game {chatmix.gamePercent}%  ·  Chat {chatmix.chatPercent}%
      </p>
    </section>
  );
}

function FirmwareVersionSection({ device }: { device: DeviceSummary }) {
  const [version, setVersion] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setVersion(null);
    setError(null);

    commands
      .getFirmwareVersion(device.productId)
      .then((result) => {
        if (cancelled) return;
        if (result.status === "ok") {
          setVersion(result.data);
        } else {
          setError(result.error);
        }
        setLoading(false);
      })
      .catch((e) => {
        if (cancelled) return;
        setError(String(e));
        setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [device.productId]);

  return (
    <section className="device-detail-page__section">
      <h2>Firmware version</h2>
      {loading && <p className="device-detail-page__help">Reading…</p>}
      {!loading && version !== null && <p className="device-detail-page__help">{version}</p>}
      {!loading && error !== null && (
        <p className="device-detail-page__status device-detail-page__status--error">{error}</p>
      )}
    </section>
  );
}

export function DeviceDetailPage({
  device,
  devicesLoaded,
  error,
  onBack,
}: {
  device: DeviceSummary | undefined;
  devicesLoaded: boolean;
  error: string | null;
  onBack: () => void;
}) {
  if (device === undefined) {
    return (
      <section className="device-detail-page">
        <button className="device-detail-page__back" onClick={onBack}>
          ← Devices
        </button>
        {error !== null ? (
          <p className="device-detail-page__status device-detail-page__status--error" role="alert">
            Couldn't read devices: {error}
          </p>
        ) : devicesLoaded ? (
          <div className="device-detail-page__empty">
            <p>This device is no longer connected.</p>
          </div>
        ) : (
          <div className="device-detail-page__empty">
            <p>Loading…</p>
          </div>
        )}
      </section>
    );
  }

  const name = displayName(device.name);
  const connection = connectionLabel(device.connectionType);
  const showsNoControls =
    device.supported &&
    !device.supportsDpi &&
    !device.supportsPollingRate &&
    !device.supportsButtonMapping &&
    !device.supportsSidetone &&
    !device.supportsMicMuteLed &&
    !device.supportsMicVolume &&
    !device.supportsEqualizer &&
    !device.supportsSleepTimer &&
    !device.supportsVolumeLimiter &&
    !device.supportsBluetoothSettings &&
    !device.supportsRgb;

  return (
    <section className="device-detail-page">
      <button className="device-detail-page__back" onClick={onBack}>
        ← Devices
      </button>

      <header className="device-detail-page__header">
        <div className="device-detail-page__identity">
          <h1>{name}</h1>
          <div className="device-detail-page__row">
            <span className="device-detail-page__product-id">{productIdLabel(device.productId)}</span>
            {connection !== "" && <span className="device-card__badge">{connection}</span>}
          </div>
        </div>
        {device.battery !== null && (
          <span className="device-card__battery">
            {batteryText(device.battery.level, device.battery.isCharging)}
          </span>
        )}
      </header>

      {!device.supported && (
        <div className="device-detail-page__notice">
          <p>This device isn't supported yet.</p>
          {device.note !== null && <p className="device-detail-page__help">{device.note}</p>}
        </div>
      )}

      {device.supported && device.note !== null && (
        <p className="device-detail-page__status device-detail-page__status--error">{device.note}</p>
      )}

      {device.supported && device.supportsFirmware && <FirmwareVersionSection device={device} />}
      {device.supported && device.chatmix !== null && <ChatMixSection chatmix={device.chatmix} />}
      {device.supported && device.supportsDpi && <DpiSection device={device} />}
      {device.supported && device.supportsPollingRate && <PollingRateSection device={device} />}
      {device.supported && device.supportsSidetone && <SidetoneSection device={device} />}
      {device.supported && device.supportsMicMuteLed && <MicMuteLedSection device={device} />}
      {device.supported && device.supportsMicVolume && <MicVolumeSection device={device} />}
      {device.supported && device.supportsEqualizer && <EqualizerSection device={device} />}
      {device.supported && device.supportsSleepTimer && <SleepTimerSection device={device} />}
      {device.supported && device.supportsVolumeLimiter && <VolumeLimiterSection device={device} />}
      {device.supported && device.supportsBluetoothSettings && <BluetoothSection device={device} />}
      {device.supported && device.supportsButtonMapping && <ButtonMappingSection device={device} />}
      {device.supported && device.supportsRgb && <RgbSection device={device} />}

      {showsNoControls && (
        <div className="device-detail-page__notice">
          <p>No adjustable settings implemented yet for this device.</p>
        </div>
      )}
    </section>
  );
}
