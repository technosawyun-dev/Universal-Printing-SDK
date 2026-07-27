import 'dart:convert';
import 'dart:typed_data';

/// Minimal ESC/POS byte-sequence builder, mirroring the constant set in
/// escpos-php's `Printer.php` (MIT licensed reference — see
/// "Opensource Printers printing/escpos-php" alongside this repo) and kept
/// semantically in step with `packages/core-ts/src/escpos.ts` and
/// `crates/core-rs/src/escpos.rs`.
///
/// This exists as a small standalone Dart implementation (rather than always
/// requiring the native library) so `sdk_flutter` has one fully working
/// thermal transport (TCP) even on a target platform/architecture the native
/// build hasn't been produced for yet. The full renderer set (ESC/POS, HTML,
/// SVG, plain text, PDF) is also available via `renderToEscpos()` and
/// friends in `src/rust/api/render.dart`, a flutter_rust_bridge binding onto
/// `crates/core-rs` (see docs/ARCHITECTURE.md's "Why sdk_flutter doesn't use
/// core-ffi") — prefer that once your app has the native build set up, so
/// the byte-sequence logic isn't maintained a third time long-term.
class EscPosBuilder {
  final BytesBuilder _out = BytesBuilder();

  static const int esc = 0x1B;
  static const int gs = 0x1D;

  EscPosBuilder() {
    // ESC @ : initialize
    _out.add([esc, 0x40]);
  }

  EscPosBuilder align(String value) {
    final n = switch (value) {
      'center' => 1,
      'right' => 2,
      _ => 0,
    };
    // ESC a n
    _out.add([esc, 0x61, n]);
    return this;
  }

  EscPosBuilder bold(bool on) {
    // ESC E n
    _out.add([esc, 0x45, on ? 1 : 0]);
    return this;
  }

  EscPosBuilder underline(bool on) {
    // ESC - n
    _out.add([esc, 0x2D, on ? 1 : 0]);
    return this;
  }

  EscPosBuilder invert(bool on) {
    // GS B n
    _out.add([gs, 0x42, on ? 1 : 0]);
    return this;
  }

  EscPosBuilder size(int w, int h) {
    final wClamped = w.clamp(1, 8) - 1;
    final hClamped = h.clamp(1, 8) - 1;
    // GS ! n (width nibble | height nibble)
    _out.add([gs, 0x21, (wClamped << 4) | hClamped]);
    return this;
  }

  EscPosBuilder font(String value) {
    // ESC M n (0 = font A, 1 = font B)
    _out.add([esc, 0x4D, value == 'b' ? 1 : 0]);
    return this;
  }

  EscPosBuilder text(String value) {
    _out.add(_toCp437ish(value));
    return this;
  }

  EscPosBuilder newline() {
    _out.add([0x0A]);
    return this;
  }

  EscPosBuilder feed(int lines) {
    for (var i = 0; i < lines; i++) {
      _out.add([0x0A]);
    }
    return this;
  }

  EscPosBuilder rule(int width, {String char = '-'}) {
    text(char * width);
    newline();
    return this;
  }

  EscPosBuilder cut({String mode = 'partial'}) {
    // GS V m (0 = full, 1 = partial)
    _out.add([gs, 0x56, mode == 'full' ? 0 : 1]);
    return this;
  }

  EscPosBuilder drawer({int pin = 2}) {
    // ESC p m t1 t2 -- m: 0 = pin 2, 1 = pin 5
    _out.add([esc, 0x70, pin == 5 ? 1 : 0, 25, 250]);
    return this;
  }

  static const Map<String, int> _barcodeType = {
    'upca': 65,
    'ean13': 67,
    'code39': 69,
    'code128': 73,
  };

  /// `symbology` must be one of 'code128', 'code39', 'ean13', 'upca' -- see
  /// docs/DOCUMENT_FORMAT.md. Mirrors `writeBarcode` in
  /// packages/core-ts/src/render/escpos.ts and
  /// crates/core-rs/src/render/escpos.rs byte-for-byte.
  EscPosBuilder barcode(
    String symbology,
    String data, {
    int heightPx = 60,
    bool showText = true,
  }) {
    final type = _barcodeType[symbology];
    if (type == null) {
      throw ArgumentError.value(symbology, 'symbology', 'must be one of ${_barcodeType.keys.join(', ')}');
    }
    _out.add([gs, 0x68, heightPx.clamp(1, 255)]); // GS h n -- barcode height
    _out.add([gs, 0x77, 3]); // GS w n -- module width
    _out.add([gs, 0x48, showText ? 2 : 0]); // GS H n -- HRI text below (default) or none
    final content = symbology == 'code128' ? '{B$data' : data;
    final bytes = _toCp437ish(content);
    _out.add([gs, 0x6B, type, bytes.length]);
    _out.add(bytes);
    return this;
  }

  EscPosBuilder qrcode(String data, {int size = 6, String ec = 'M'}) {
    final bytes = utf8.encode(data);
    final storeLen = bytes.length + 3;
    final pL = storeLen & 0xFF;
    final pH = (storeLen >> 8) & 0xFF;
    final ecN = switch (ec) {
      'L' => 48,
      'Q' => 50,
      'H' => 51,
      _ => 49,
    };
    // GS ( k : select model, set module size, set EC level, store data, print
    _out.add([gs, 0x28, 0x6B, 0x04, 0x00, 0x31, 0x41, 0x32, 0x00]);
    _out.add([gs, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x43, size.clamp(1, 16)]);
    _out.add([gs, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x45, ecN]);
    _out.add([gs, 0x28, 0x6B, pL, pH, 0x31, 0x50, 0x30]);
    _out.add(bytes);
    _out.add([gs, 0x28, 0x6B, 0x03, 0x00, 0x31, 0x51, 0x30]);
    return this;
  }

  Uint8List toBytes() => _out.toBytes();

  /// Best-effort passthrough for the common Latin-1/CP437 overlap: code
  /// points above U+00FF become '?' rather than crashing. Mirrors
  /// `textToBytes` in packages/core-ts/src/render/escpos.ts and
  /// crates/core-rs/src/render/escpos.rs exactly. Full codepage switching
  /// (CJK, Burmese, Cyrillic, etc.) is intentionally out of scope for this
  /// minimal Dart builder -- use the flutter_rust_bridge renderer
  /// (src/rust/api/render.dart) once available for that.
  Uint8List _toCp437ish(String value) {
    final bytes = Uint8List(value.runes.length);
    var i = 0;
    for (final rune in value.runes) {
      bytes[i++] = rune <= 0xFF ? rune : 0x3F;
    }
    return bytes;
  }
}
