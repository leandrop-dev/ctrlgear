import { useState } from "react";
import { DeviceDetailPage } from "./components/DeviceDetailPage";
import { DeviceListPage } from "./components/DeviceListPage";
import { SettingsPage } from "./components/SettingsPage";
import { useDevices } from "./hooks/useDevices";
import "./App.css";

type View = "devices" | "settings";

function App() {
  const { devices, loading, error, refresh } = useDevices();
  const [selectedProductId, setSelectedProductId] = useState<number | null>(null);
  const [view, setView] = useState<View>("devices");

  const renderPage = () => {
    if (view === "settings") {
      return <SettingsPage />;
    }
    if (selectedProductId !== null) {
      const device = devices.find((d) => d.productId === selectedProductId);
      return (
        <DeviceDetailPage
          device={device}
          devicesLoaded={devices.length > 0 || (!loading && error === null)}
          error={error}
          onBack={() => setSelectedProductId(null)}
        />
      );
    }
    return (
      <DeviceListPage
        devices={devices}
        loading={loading}
        error={error}
        refresh={refresh}
        onSelectDevice={setSelectedProductId}
      />
    );
  };

  return (
    <div className="app-shell">
      <nav className="app-nav">
        <button
          className={`app-nav__item${view === "devices" ? " app-nav__item--active" : ""}`}
          onClick={() => setView("devices")}
        >
          Devices
        </button>
        <button
          className={`app-nav__item${view === "settings" ? " app-nav__item--active" : ""}`}
          onClick={() => setView("settings")}
        >
          Settings
        </button>
      </nav>
      <div className="app-shell__content">{renderPage()}</div>
    </div>
  );
}

export default App;
