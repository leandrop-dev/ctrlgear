import type { DeviceSummary } from "../bindings";
import { batteryText, connectionLabel, displayName, productIdLabel } from "../lib/deviceLabels";
import "../styles/DeviceListPage.css";

function DeviceCard({ device, onSelect }: { device: DeviceSummary; onSelect: () => void }) {
  const name = displayName(device.name);
  const badge = connectionLabel(device.connectionType);

  return (
    <li
      className={`device-card${device.supported ? "" : " device-card--unsupported"}`}
      role="button"
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect();
        }
      }}
    >
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

export function DeviceListPage({
  devices,
  loading,
  error,
  refresh,
  onSelectDevice,
}: {
  devices: DeviceSummary[];
  loading: boolean;
  error: string | null;
  refresh: () => void;
  onSelectDevice: (productId: number) => void;
}) {
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
            <DeviceCard
              key={device.productId}
              device={device}
              onSelect={() => onSelectDevice(device.productId)}
            />
          ))}
        </ul>
      )}
    </section>
  );
}
