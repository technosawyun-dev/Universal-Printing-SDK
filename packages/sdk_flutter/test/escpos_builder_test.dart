import 'package:flutter_test/flutter_test.dart';
import 'package:sdk_flutter/sdk_flutter.dart';

void main() {
  group('EscPosBuilder', () {
    test('starts every document with the initialize command', () {
      final bytes = EscPosBuilder().toBytes();
      expect(bytes.sublist(0, 2), [0x1B, 0x40]);
    });

    test('encodes align/bold/underline/size/font before printing text', () {
      final bytes = EscPosBuilder().align('center').bold(true).text('hi').toBytes();
      // ESC @ , ESC a 1 (center), ESC E 1 (bold), 'h' 'i'
      expect(bytes, [0x1B, 0x40, 0x1B, 0x61, 1, 0x1B, 0x45, 1, 0x68, 0x69]);
    });

    test('cut() maps mode to 0=full, 1=partial', () {
      final full = EscPosBuilder().cut(mode: 'full').toBytes();
      final partial = EscPosBuilder().cut().toBytes();
      expect(full.sublist(full.length - 3), [0x1D, 0x56, 0]);
      expect(partial.sublist(partial.length - 3), [0x1D, 0x56, 1]);
    });

    test('drawer() encodes pin 2 vs pin 5 and fixed timing bytes', () {
      final pin2 = EscPosBuilder().drawer().toBytes();
      final pin5 = EscPosBuilder().drawer(pin: 5).toBytes();
      expect(pin2.sublist(pin2.length - 5), [0x1B, 0x70, 0, 25, 250]);
      expect(pin5.sublist(pin5.length - 5), [0x1B, 0x70, 1, 25, 250]);
    });

    test('qrcode() uses the correct EC-level byte for every level (L=48,M=49,Q=50,H=51)', () {
      // Regression test: Q and H were previously off-by-one (51/52 instead
      // of 50/51), verified against the real Epson command spec and
      // cross-checked byte-for-byte against core-ts/escpos.ts and
      // core-rs/escpos.rs, which all agree on 48/49/50/51.
      final expected = {'L': 48, 'M': 49, 'Q': 50, 'H': 51};
      for (final entry in expected.entries) {
        final bytes = EscPosBuilder().qrcode('hi', ec: entry.key).toBytes();
        // Find the "set EC level" sequence: GS ( k 03 00 '1' 'E' <ecByte>
        int? ecByte;
        for (var i = 0; i + 7 <= bytes.length; i++) {
          if (bytes[i] == 0x1D &&
              bytes[i + 1] == 0x28 &&
              bytes[i + 2] == 0x6B &&
              bytes[i + 5] == 0x31 &&
              bytes[i + 6] == 0x45) {
            ecByte = bytes[i + 7];
            break;
          }
        }
        expect(ecByte, entry.value, reason: 'EC level ${entry.key}');
      }
    });

    test('qrcode() stores the UTF-8 data bytes verbatim', () {
      final bytes = EscPosBuilder().qrcode('hi').toBytes();
      expect(String.fromCharCodes(bytes), contains('hi'));
    });

    test('feed(n) emits n line feeds', () {
      final bytes = EscPosBuilder().feed(3).toBytes();
      expect(bytes.sublist(bytes.length - 3), [0x0A, 0x0A, 0x0A]);
    });

    test('text() replaces characters above U+00FF with "?" instead of crashing', () {
      // Regression test: _toCp437ish used to be a no-op that returned its
      // input unchanged, so latin1.encode() threw on any non-Latin-1
      // character (e.g. Burmese, CJK, emoji) instead of degrading
      // gracefully like core-ts/core-rs's textToBytes.
      final bytes = EscPosBuilder().text('héllo 中文').toBytes();
      final text = String.fromCharCodes(bytes.sublist(2)); // skip ESC @
      expect(text, 'héllo ??'); // é (U+00E9) is in range; 中文 becomes ??
    });

    test('barcode() builds a CODE128 barcode with the {B subset prefix and function-B framing', () {
      final bytes = EscPosBuilder().barcode('code128', '12345').toBytes();
      final gsk = () {
        for (var i = 0; i + 3 <= bytes.length; i++) {
          if (bytes[i] == 0x1D && bytes[i + 1] == 0x6B && bytes[i + 2] == 73) return i;
        }
        return -1;
      }();
      expect(gsk, greaterThan(-1));
      expect(bytes[gsk + 3], 7); // len("{B12345")
      expect(String.fromCharCodes(bytes.sublist(gsk + 4, gsk + 4 + 7)), '{B12345');
    });

    test('barcode() maps symbology to the correct type byte', () {
      final expected = {'upca': 65, 'ean13': 67, 'code39': 69, 'code128': 73};
      for (final entry in expected.entries) {
        final bytes = EscPosBuilder().barcode(entry.key, '123').toBytes();
        final typeByte = () {
          for (var i = 0; i + 3 <= bytes.length; i++) {
            if (bytes[i] == 0x1D && bytes[i + 1] == 0x6B) return bytes[i + 2];
          }
          return -1;
        }();
        expect(typeByte, entry.value, reason: entry.key);
      }
    });

    test('barcode() rejects an unknown symbology', () {
      expect(() => EscPosBuilder().barcode('carrier-pigeon', '123'), throwsArgumentError);
    });
  });
}
