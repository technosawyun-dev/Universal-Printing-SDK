// Direct Web Serial printing -- for USB-serial thermal printers. Unverified
// in this environment: requires a real serial device, a user gesture
// (requestPort() shows a native picker), and an actual Chrome/Edge tab.

import type { PrintDocument } from "@printers/core-ts";
import { toEscPos } from "@printers/core-ts";
import { UnsupportedTransportError } from "./errors.js";

const DEFAULT_BAUD_RATE = 9600;

export async function requestWebSerialPort(): Promise<SerialPort> {
  if (!navigator.serial) throw new UnsupportedTransportError("Web Serial is not available in this browser");
  return navigator.serial.requestPort();
}

/** Ports the user has already granted permission to in a prior session -- no picker shown. */
export async function listPairedWebSerialPorts(): Promise<SerialPort[]> {
  if (!navigator.serial) return [];
  return navigator.serial.getPorts();
}

export async function sendToWebSerialPort(port: SerialPort, bytes: Uint8Array, baudRate: number = DEFAULT_BAUD_RATE): Promise<void> {
  if (port.readable == null && port.writable == null) {
    await port.open({ baudRate });
  }
  if (port.writable == null) {
    throw new UnsupportedTransportError("serial port has no writable stream after open()");
  }
  const writer = port.writable.getWriter();
  try {
    await writer.write(bytes);
  } finally {
    writer.releaseLock();
  }
}

export async function printViaWebSerial(port: SerialPort, document: PrintDocument, baudRate: number = DEFAULT_BAUD_RATE): Promise<void> {
  await sendToWebSerialPort(port, toEscPos(document), baudRate);
}
