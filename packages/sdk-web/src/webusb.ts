// Direct WebUSB printing -- the browser talks straight to a USB thermal
// printer, no agent needed. Unverified in this environment: WebUSB requires
// a real device, a user gesture (requestDevice() shows a native picker), and
// an actual Chrome/Edge tab, none of which exist in this sandbox. The
// interface/endpoint-selection logic below is unit-tested against a fake
// USBDevice shape; the actual `transferOut` call is not.

import type { PrintDocument } from "@printers/core-ts";
import { toEscPos } from "@printers/core-ts";
import { UnsupportedTransportError } from "./errors.js";

/** Most ESC/POS USB printers advertise USB printer class 7; narrow further with vendorId/productId if you know your device. */
export async function requestWebUsbPrinter(filters: USBDeviceFilter[] = [{ classCode: 7 }]): Promise<USBDevice> {
  if (!navigator.usb) throw new UnsupportedTransportError("WebUSB is not available in this browser");
  return navigator.usb.requestDevice({ filters });
}

/** Devices the user has already granted permission to in a prior session -- no picker shown. */
export async function listPairedWebUsbPrinters(): Promise<USBDevice[]> {
  if (!navigator.usb) return [];
  return navigator.usb.getDevices();
}

interface ClaimedPrinterInterface {
  interfaceNumber: number;
  endpointOut: number;
}

/** Finds the first interface exposing a bulk OUT endpoint and claims it. */
export async function claimPrinterInterface(device: USBDevice): Promise<ClaimedPrinterInterface> {
  if (!device.opened) await device.open();
  if (device.configuration == null) await device.selectConfiguration(1);

  for (const iface of device.configuration!.interfaces) {
    const alternate = iface.alternates[0];
    const outEndpoint = alternate?.endpoints.find((e) => e.direction === "out" && e.type === "bulk");
    if (outEndpoint) {
      await device.claimInterface(iface.interfaceNumber);
      return { interfaceNumber: iface.interfaceNumber, endpointOut: outEndpoint.endpointNumber };
    }
  }
  throw new UnsupportedTransportError("no bulk OUT endpoint found on this USB device");
}

export async function sendToWebUsbPrinter(device: USBDevice, bytes: Uint8Array): Promise<void> {
  const { endpointOut } = await claimPrinterInterface(device);
  // `new Uint8Array(bytes)` guarantees an ArrayBuffer-backed (not
  // SharedArrayBuffer-backed) view, which is what BufferSource requires.
  const result = await device.transferOut(endpointOut, new Uint8Array(bytes));
  if (result.status !== "ok") {
    throw new Error(`WebUSB transferOut failed with status '${result.status}'`);
  }
}

export async function printViaWebUsb(device: USBDevice, document: PrintDocument): Promise<void> {
  await sendToWebUsbPrinter(device, toEscPos(document));
}
