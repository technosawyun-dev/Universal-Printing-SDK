import 'sdk_flutter_platform_interface.dart';

// Standalone Dart ESC/POS builder + document model -- works without the
// native Rust library being built for the target platform yet (see
// src/escpos_builder.dart's own header comment).
export 'src/document.dart';
export 'src/escpos_builder.dart';

// Delivery transports -- generating ESC/POS bytes is only half the job;
// these actually get them to a printer. TCP is cross-platform with no
// native code; USB is Android-only (usb_serial, Android USB Host API);
// Bluetooth is Android+iOS (flutter_blue_plus, BLE). All three mirror
// POS_system_mobile's printer_service.dart, already proven in production.
export 'src/tcp_transport.dart';
export 'src/usb_transport.dart';
export 'src/bluetooth_transport.dart';

// flutter_rust_bridge bindings over crates/core-rs (via rust/src/api/render.rs)
// -- the full renderer set (ESC/POS, HTML, SVG, plain text, PDF), once the
// app has the native library built for its target platform/architecture.
export 'src/rust/api/render.dart';
export 'src/rust/frb_generated.dart' show RustLib;

class SdkFlutter {
  Future<String?> getPlatformVersion() {
    return SdkFlutterPlatform.instance.getPlatformVersion();
  }
}
