import type { ConnectionType } from "../bindings";

const VENDOR_PREFIX = "SteelSeries ";

/** Strips the redundant "SteelSeries" prefix from a device name for display. */
export function displayName(name: string): string {
  return name.startsWith(VENDOR_PREFIX) ? name.slice(VENDOR_PREFIX.length) : name;
}

/** Short label shown next to a device's name when reachable over more than one transport. */
export function connectionLabel(kind: ConnectionType): string {
  switch (kind) {
    case "usbWired":
      return "USB";
    case "dongle24Ghz":
      return "2.4GHz";
    case "bluetooth":
      return "Bluetooth";
    case "unknown":
      return "";
  }
}

/** Formats a USB product ID as "ID 0x1830" (lowercase, zero-padded to 4 digits). */
export function productIdLabel(productId: number): string {
  return `ID 0x${productId.toString(16).padStart(4, "0")}`;
}

/** Formats a battery reading as "Charging ⚡ NN%" (or plain "NN%" when not charging). */
export function batteryText(percent: number, charging: boolean): string {
  return charging ? `Charging ⚡ ${percent}%` : `${percent}%`;
}

/** Formats a device's valid DPI range and preset limit as help text for the detail page. */
export function dpiHelpText(dpiRange: [number, number, number] | null, dpiMaxPresets: number | null): string {
  if (dpiRange === null) return "";
  const [min, max, step] = dpiRange;
  const presets = dpiMaxPresets !== null ? `, up to ${dpiMaxPresets} presets` : "";
  return `Supports ${min}–${max} DPI in steps of ${step}${presets}.`;
}
