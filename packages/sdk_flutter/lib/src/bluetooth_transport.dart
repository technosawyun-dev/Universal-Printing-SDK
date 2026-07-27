import 'dart:typed_data';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';

export 'package:flutter_blue_plus/flutter_blue_plus.dart' show BluetoothDevice, BluetoothCharacteristic;

/// Direct Bluetooth (BLE) thermal printer transport -- cross-platform
/// (Android + iOS, see docs/ARCHITECTURE.md). Mirrors sdk-web's
/// web-bluetooth.ts: there is no single universal BLE printer GATT profile,
/// so the Nordic UART Service (a de facto standard used by many BLE-to-
/// serial bridge chips) is the default; override for your specific printer
/// if it uses something else. The scan/connect shape mirrors
/// POS_system_mobile's printer_service.dart, already proven in production.
const nordicUartService = "6e400001-b5a3-f393-e0a9-e50e24dcca9e";
const nordicUartRxCharacteristic = "6e400002-b5a3-f393-e0a9-e50e24dcca9e";

/// Default 20 bytes: the minimum GATT MTU every BLE device must support
/// without a negotiated larger MTU -- see sdk-web/web-bluetooth.ts. Some
/// BLE stacks negotiate a larger MTU (POS_system_mobile's flutter_blue_plus
/// usage chunks at 512 bytes), but 20 is the safe default without assuming
/// negotiation succeeded.
const defaultBleChunkSize = 20;

Future<List<BluetoothDevice>> scanBluetoothPrinters({Duration timeout = const Duration(seconds: 5)}) async {
  final found = <BluetoothDevice>[];
  final sub = FlutterBluePlus.scanResults.listen((results) {
    for (final r in results) {
      if (!found.any((d) => d.remoteId == r.device.remoteId)) found.add(r.device);
    }
  });
  try {
    await FlutterBluePlus.startScan(timeout: timeout);
    await Future.delayed(timeout);
  } finally {
    await sub.cancel();
    await FlutterBluePlus.stopScan();
  }
  return found;
}

/// Connects to `device` and returns the writable characteristic matching
/// `serviceUuid`/`characteristicUuid` (defaults to the Nordic UART Service).
Future<BluetoothCharacteristic> connectBluetoothPrinter(
  BluetoothDevice device, {
  String serviceUuid = nordicUartService,
  String characteristicUuid = nordicUartRxCharacteristic,
}) async {
  await device.connect(autoConnect: false);
  final services = await device.discoverServices();
  for (final service in services) {
    if (service.uuid.str128.toLowerCase() != serviceUuid.toLowerCase()) continue;
    for (final characteristic in service.characteristics) {
      if (characteristic.uuid.str128.toLowerCase() == characteristicUuid.toLowerCase()) {
        return characteristic;
      }
    }
  }
  throw StateError("no matching GATT characteristic found (service $serviceUuid, characteristic $characteristicUuid)");
}

/// Splits `bytes` into GATT-write-sized chunks. Pure logic, no BLE
/// dependency -- see tests.
List<Uint8List> chunkBytes(Uint8List bytes, [int chunkSize = defaultBleChunkSize]) {
  final chunks = <Uint8List>[];
  for (var i = 0; i < bytes.length; i += chunkSize) {
    final end = (i + chunkSize > bytes.length) ? bytes.length : i + chunkSize;
    chunks.add(bytes.sublist(i, end));
  }
  return chunks;
}

Future<void> sendToBluetoothPrinter(
  BluetoothCharacteristic characteristic,
  Uint8List bytes, {
  int chunkSize = defaultBleChunkSize,
}) async {
  for (final chunk in chunkBytes(bytes, chunkSize)) {
    await characteristic.write(chunk, withoutResponse: true);
  }
}
