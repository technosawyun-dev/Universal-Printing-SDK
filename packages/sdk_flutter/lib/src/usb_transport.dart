import 'dart:typed_data';
import 'package:usb_serial/usb_serial.dart';

export 'package:usb_serial/usb_serial.dart' show UsbDevice, UsbPort;

/// Direct USB thermal printer transport via Android's USB Host API --
/// Android-only, see docs/ARCHITECTURE.md ("no raw USB on iOS -- Apple
/// restricted"). Also covers USB-to-serial adapters (CH340/PL2303/FTDI/
/// CP21xx) through the same `UsbPort` abstraction. Proven in production by
/// POS_system_mobile's printer_service.dart, which this mirrors.
Future<List<UsbDevice>> listUsbPrinters() => UsbSerial.listDevices();

/// Opens `device` at `baudRate` (default 9600, the common default for
/// ESC/POS USB-serial adapters). Caller is responsible for closing the
/// returned port when done.
Future<UsbPort> openUsbPrinter(UsbDevice device, {int baudRate = 9600}) async {
  final port = await device.create();
  if (port == null) {
    throw StateError("failed to create a UsbPort for '${device.productName}'");
  }
  if (!await port.open()) {
    throw StateError("failed to open USB device '${device.productName}'");
  }
  await port.setDTR(true);
  await port.setRTS(true);
  await port.setPortParameters(baudRate, UsbPort.DATABITS_8, UsbPort.STOPBITS_1, UsbPort.PARITY_NONE);
  return port;
}

Future<void> sendToUsbPrinter(UsbPort port, Uint8List bytes) => port.write(bytes);
