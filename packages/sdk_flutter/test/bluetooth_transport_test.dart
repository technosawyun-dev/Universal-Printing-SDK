import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:sdk_flutter/sdk_flutter.dart';

void main() {
  group('chunkBytes', () {
    test('splits into chunks no larger than chunkSize', () {
      final bytes = Uint8List.fromList(List.generate(45, (i) => i));
      final chunks = chunkBytes(bytes, 20);
      expect(chunks, hasLength(3));
      expect(chunks[0], hasLength(20));
      expect(chunks[1], hasLength(20));
      expect(chunks[2], hasLength(5));
    });

    test('reassembles to the original bytes', () {
      final bytes = Uint8List.fromList(List.generate(45, (i) => i));
      final chunks = chunkBytes(bytes, 20);
      final reassembled = <int>[];
      for (final c in chunks) {
        reassembled.addAll(c);
      }
      expect(reassembled, bytes);
    });

    test('returns a single chunk when bytes fit within chunkSize', () {
      final bytes = Uint8List.fromList([1, 2, 3]);
      expect(chunkBytes(bytes, 20), [bytes]);
    });

    test('returns an empty list for empty input', () {
      expect(chunkBytes(Uint8List(0), 20), isEmpty);
    });

    test('defaults to 20-byte chunks (minimum GATT MTU)', () {
      final bytes = Uint8List.fromList(List.generate(25, (i) => i));
      final chunks = chunkBytes(bytes);
      expect(chunks[0], hasLength(20));
      expect(chunks[1], hasLength(5));
    });
  });
}
