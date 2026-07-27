import 'dart:io';
import 'dart:typed_data';

/// Direct WiFi/LAN thermal printer transport via a raw TCP socket (port 9100
/// is the common "JetDirect"/RAW convention -- see docs/PROTOCOL.md).
/// Cross-platform with zero native platform code, unlike USB/Bluetooth --
/// proven in production by POS_system_mobile's printer_service.dart, which
/// this mirrors.
Future<void> sendToTcpPrinter(
  String host,
  int port,
  Uint8List bytes, {
  Duration timeout = const Duration(seconds: 5),
}) async {
  final socket = await Socket.connect(host, port, timeout: timeout);
  try {
    socket.add(bytes);
    await socket.flush();
  } finally {
    await socket.close();
  }
}
