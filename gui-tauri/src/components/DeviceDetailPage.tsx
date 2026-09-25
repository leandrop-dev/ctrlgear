import { useEffect, useState } from "react";
import { commands, type ButtonActionChoice, type DeviceSummary } from "../bindings";
import { batteryText, connectionLabel, displayName, dpiHelpText, productIdLabel } from "../lib/deviceLabels";
import { parseDpiInput } from "../lib/dpiInput";
import "../styles/DeviceDetailPage.css";

type ApplyStatus = { ok: boolean; message: string } | null;

/** Default button-mapping keyword for a slot before the user changes it, matching the Slint page's "Default" default. */
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
  const showsNoControls = device.supported && !device.supportsDpi && !device.supportsPollingRate && !device.supportsButtonMapping;

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

      {device.supported && device.supportsDpi && <DpiSection device={device} />}
      {device.supported && device.supportsPollingRate && <PollingRateSection device={device} />}
      {device.supported && device.supportsButtonMapping && <ButtonMappingSection device={device} />}

      {showsNoControls && (
        <div className="device-detail-page__notice">
          <p>No adjustable settings implemented yet for this device.</p>
        </div>
      )}
    </section>
  );
}
