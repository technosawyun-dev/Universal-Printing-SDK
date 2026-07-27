// Direct Web Bluetooth (BLE-only) printing -- see docs/ARCHITECTURE.md's
// "Browser Bluetooth is BLE-only" note for the platform limits (no classic
// SPP/RFCOMM, no Safari/iOS). Unverified in this environment: requires a
// real BLE printer, a user gesture, and an actual Chrome/Edge tab.
//
// There is no single universal BLE printer GATT profile the way there is for
// ESC/POS bytes over a stream. Many cheap BLE thermal printers expose a
// serial-bridge-style service; the Nordic UART Service (a de facto standard
// used by a large fraction of BLE-to-serial bridge chips) is used as the
// default here, but override serviceUuid/characteristicUuid for your
// specific printer if it doesn't use it.

import type { PrintDocument } from "@printers/core-ts";
import { toEscPos } from "@printers/core-ts";
import { UnsupportedTransportError } from "./errors.js";

export const NORDIC_UART_SERVICE = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
export const NORDIC_UART_RX_CHARACTERISTIC = "6e400002-b5a3-f393-e0a9-e50e24dcca9e";

/** Default 20 bytes: the minimum GATT MTU (23 bytes minus a 3-byte ATT header) every BLE device must support without a negotiated larger MTU. */
const DEFAULT_CHUNK_SIZE = 20;

export interface WebBluetoothPrinterOptions {
  serviceUuid?: BluetoothServiceUUID;
  characteristicUuid?: BluetoothCharacteristicUUID;
  chunkSize?: number;
}

export async function requestWebBluetoothPrinter(options: WebBluetoothPrinterOptions = {}): Promise<BluetoothDevice> {
  if (!navigator.bluetooth) {
    throw new UnsupportedTransportError(
      "Web Bluetooth is not available in this browser (Chrome/Edge only, no iOS Safari -- see docs/ARCHITECTURE.md)",
    );
  }
  const serviceUuid = options.serviceUuid ?? NORDIC_UART_SERVICE;
  return navigator.bluetooth.requestDevice({ filters: [{ services: [serviceUuid] }] });
}

async function getWritableCharacteristic(
  device: BluetoothDevice,
  options: WebBluetoothPrinterOptions,
): Promise<BluetoothRemoteGATTCharacteristic> {
  const serviceUuid = options.serviceUuid ?? NORDIC_UART_SERVICE;
  const characteristicUuid = options.characteristicUuid ?? NORDIC_UART_RX_CHARACTERISTIC;
  if (!device.gatt) throw new UnsupportedTransportError("this Bluetooth device has no GATT server");
  const server = device.gatt.connected ? device.gatt : await device.gatt.connect();
  const service = await server.getPrimaryService(serviceUuid);
  return service.getCharacteristic(characteristicUuid);
}

/** Splits `bytes` into GATT-write-sized chunks. Pure logic, no BLE dependency -- see tests. */
export function chunkBytes(bytes: Uint8Array, chunkSize: number = DEFAULT_CHUNK_SIZE): Uint8Array[] {
  const chunks: Uint8Array[] = [];
  for (let i = 0; i < bytes.length; i += chunkSize) chunks.push(bytes.slice(i, i + chunkSize));
  return chunks;
}

export async function sendToWebBluetoothPrinter(
  device: BluetoothDevice,
  bytes: Uint8Array,
  options: WebBluetoothPrinterOptions = {},
): Promise<void> {
  const characteristic = await getWritableCharacteristic(device, options);
  for (const chunk of chunkBytes(bytes, options.chunkSize ?? DEFAULT_CHUNK_SIZE)) {
    // `new Uint8Array(chunk)` guarantees an ArrayBuffer-backed (not
    // SharedArrayBuffer-backed) view, which is what BufferSource requires.
    await characteristic.writeValueWithoutResponse(new Uint8Array(chunk));
  }
}

export async function printViaWebBluetooth(
  device: BluetoothDevice,
  document: PrintDocument,
  options?: WebBluetoothPrinterOptions,
): Promise<void> {
  await sendToWebBluetoothPrinter(device, toEscPos(document), options);
}
