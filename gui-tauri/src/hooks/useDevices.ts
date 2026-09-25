import { useCallback, useEffect, useRef, useState } from "react";
import { commands, events, type DeviceSummary } from "../bindings";

/** Live device list, refreshed on demand and pushed by the poll loop's `device-list-updated` event. */
export function useDevices() {
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

  return { devices, loading, error, refresh };
}
