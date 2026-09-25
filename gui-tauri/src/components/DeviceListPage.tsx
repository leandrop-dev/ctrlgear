import { useCallback, useEffect, useRef, useState } from "react";
import { commands, events, type DeviceSummary } from "../bindings";
import { batteryText, connectionLabel, displayName, productIdLabel } from "../lib/deviceLabels";
import "../styles/DeviceListPage.css";

function DeviceCard({ device }: { device: DeviceSummary }) {
  const name = displayName(device.name);
  const badge = connectionLabel(device.connectionType);

  return (
    <li className={`device-card${device.supported ? "" : " device-card--unsupported"}`}>
      <div className="device-card__avatar">{name.charAt(0).toUpperCase()}</div>
      <div className="device-card__info">
        <div className="device-card__row">
          <span className="device-card__name">{name}</span>
          {badge !== "" && <span className="device-card__badge">{badge}</span>}
        </div>
        <div className="device-card__row">
          <span className="device-card__badge device-card__badge--status">
            {device.supported ? "Supported" : "Not yet supported"}
          </span>
          <span className="device-card__product-id">{productIdLabel(device.productId)}</span>
          {!device.battery && device.note !== null && (
            <span className="device-card__note">{device.note}</span>
          )}
        </div>
      </div>
      {device.battery !== null && (
        <span className="device-card__battery">
          {batteryText(device.battery.level, device.battery.isCharging)}
        </span>
      )}
    </li>
  );
}

export function DeviceListPage() {
  const [devices, setDevices] = useState<DeviceSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  // Bumped by every refresh request and every pushed update; a refresh
  // response only applies if nothing newer arrived while it was in flight.
  const generation = useRef(0);
  // Refresh calls only (not pushed events), so `loading` clears when the
  // latest refresh settles and never sticks when an event lands mid-flight.
  const latestRefreshId = useRef(0);

  const refresh = useCallback(() => {
    const requestGeneration = ++generation.current;
    const refreshId = ++latestRefreshId.current;
    setLoading(true);
    commands
      .listDevices()
      .then((result) => {
        if (requestGeneration !== generation.current) return;
        if (result.status === "ok") {
          setDevices(result.data);
          setError(null);
        } else {
          setError(result.error);
        }
      })
      .catch((e) => {
        if (requestGeneration === generation.current) setError(String(e));
      })
      .finally(() => {
        if (refreshId === latestRefreshId.current) setLoading(false);
      });
  }, []);

  useEffect(() => {
    refresh();
    const unlisten = events.deviceListUpdated.listen((event) => {
      generation.current++;
      setDevices(event.payload);
      setError(null);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [refresh]);

  return (
    <section className="device-list-page">
      <header className="device-list-page__header">
        <h1>Devices</h1>
        <button onClick={refresh} disabled={loading}>
          {loading ? "Refreshing…" : "Refresh"}
        </button>
      </header>

      {error !== null && (
        <p className="device-list-page__error" role="alert">
          Couldn't read devices: {error}
        </p>
      )}

      {devices.length === 0 && !loading && error === null && (
        <div className="device-list-page__empty">
          <p>No SteelSeries devices found.</p>
          <p>Connect a device and press Refresh.</p>
        </div>
      )}

      {devices.length > 0 && (
        <ul className="device-list-page__list">
          {devices.map((device) => (
            <DeviceCard key={device.productId} device={device} />
          ))}
        </ul>
      )}
    </section>
  );
}
