import { useState } from "react";
import { DeviceDetailPage } from "./components/DeviceDetailPage";
import { DeviceListPage } from "./components/DeviceListPage";
import { useDevices } from "./hooks/useDevices";
import "./App.css";

function App() {
  const { devices, loading, error, refresh } = useDevices();
  const [selectedProductId, setSelectedProductId] = useState<number | null>(null);

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
}

export default App;
