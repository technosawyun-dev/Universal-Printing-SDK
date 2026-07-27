import 'dart:convert';

import 'package:integration_test/integration_test.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:sdk_flutter/sdk_flutter.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  test('renders a PrintDocument to ESC/POS through the native Rust library', () {
    final doc = PrintDocumentBuilder(paperKind: 'thermal-80').text('hello').cut().build();
    final bytes = renderToEscpos(documentJson: jsonEncode(doc));
    expect(bytes.take(2), [0x1B, 0x40]); // ESC @ initialize
  });

  test('validateDocument throws on an invalid document', () {
    expect(
      () => validateDocument(documentJson: '{"version":"1.0","paper":{"kind":"custom"},"content":[]}'),
      throwsA(anything),
    );
  });
}
