import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:sdk_flutter/sdk_flutter.dart';

void main() {
  group('sendToTcpPrinter', () {
    test('delivers bytes to a real listening TCP socket', () async {
      final server = await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
      final receivedCompleter = Completer<List<int>>();
      server.listen((socket) {
        final buffer = <int>[];
        socket.listen(
          buffer.addAll,
          onDone: () => receivedCompleter.complete(buffer),
        );
      });

      await sendToTcpPrinter(
        InternetAddress.loopbackIPv4.address,
        server.port,
        Uint8List.fromList([0x1B, 0x40, 0x68, 0x69]),
      );

      final received = await receivedCompleter.future.timeout(const Duration(seconds: 2));
      expect(received, [0x1B, 0x40, 0x68, 0x69]);
      await server.close();
    });

    test('throws when nothing is listening on the port', () async {
      // Port 1 is reserved; nothing should be listening there.
      await expectLater(
        sendToTcpPrinter('127.0.0.1', 1, Uint8List.fromList([1]), timeout: const Duration(milliseconds: 500)),
        throwsA(anything),
      );
    });
  });
}
