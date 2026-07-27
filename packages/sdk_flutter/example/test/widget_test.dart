// Smoke test for the hardware test app: it should build without error and
// show its three transport panels.
import 'package:flutter_test/flutter_test.dart';

import 'package:sdk_flutter_example/main.dart';

void main() {
  testWidgets('Test app renders its three transport panels', (WidgetTester tester) async {
    await tester.pumpWidget(const TestApp());

    expect(find.textContaining('USB (Android USB Host API)'), findsOneWidget);
    expect(find.textContaining('Bluetooth (BLE only)'), findsOneWidget);
    expect(find.textContaining('WiFi/TCP printer'), findsOneWidget);
  });

  test('buildSampleReceipt produces a non-empty ESC/POS byte sequence', () {
    final bytes = buildSampleReceipt();
    expect(bytes, isNotEmpty);
    expect(bytes[0], 0x1B); // ESC
  });
}
