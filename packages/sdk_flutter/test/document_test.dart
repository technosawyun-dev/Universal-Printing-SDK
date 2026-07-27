import 'package:flutter_test/flutter_test.dart';
import 'package:sdk_flutter/sdk_flutter.dart';

void main() {
  group('PrintDocumentBuilder', () {
    test('builds a minimal valid PrintDocument matching docs/DOCUMENT_FORMAT.md', () {
      final doc = PrintDocumentBuilder(paperKind: 'thermal-80').text('hello').build();
      expect(doc['version'], '1.0');
      expect(doc['paper'], {'kind': 'thermal-80'});
      expect(doc['content'], [
        {
          'type': 'text',
          'value': 'hello',
          'align': 'left',
          'bold': false,
          'underline': false,
          'invert': false,
          'size': {'w': 1, 'h': 1},
          'font': 'a',
        },
      ]);
    });

    test('includes optional paper fields only when provided', () {
      final withExtras = PrintDocumentBuilder(paperKind: 'custom', widthMm: 120, charsPerLine: 40).build();
      expect(withExtras['paper'], {'kind': 'custom', 'widthMm': 120, 'charsPerLine': 40});

      final withoutExtras = PrintDocumentBuilder(paperKind: 'thermal-58').build();
      expect(withoutExtras['paper'], {'kind': 'thermal-58'});
    });

    test('row() lays out fractional-width columns', () {
      final doc = PrintDocumentBuilder(paperKind: 'thermal-80')
          .row([(text: 'Item', width: 3, align: 'left'), (text: '1.00', width: 1, align: 'right')])
          .build();
      expect(doc['content'], [
        {
          'type': 'row',
          'columns': [
            {'text': 'Item', 'width': 3, 'align': 'left'},
            {'text': '1.00', 'width': 1, 'align': 'right'},
          ],
          'bold': false,
        },
      ]);
    });

    test('chains multiple node types in call order', () {
      final doc = PrintDocumentBuilder(paperKind: 'thermal-58')
          .text('Receipt')
          .rule()
          .feed(lines: 2)
          .barcode('code128', '12345')
          .qrcode('https://example.com')
          .cut()
          .drawer()
          .raw('AA==')
          .build();
      final types = (doc['content'] as List).map((n) => (n as Map)['type']).toList();
      expect(types, ['text', 'rule', 'feed', 'barcode', 'qrcode', 'cut', 'drawer', 'raw']);
    });

    test('barcode() builds a node matching docs/DOCUMENT_FORMAT.md', () {
      final doc = PrintDocumentBuilder(paperKind: 'thermal-80').barcode('ean13', '0123456789012', heightPx: 80, showText: false).build();
      expect(doc['content'], [
        {'type': 'barcode', 'symbology': 'ean13', 'data': '0123456789012', 'heightPx': 80, 'showText': false},
      ]);
    });

    test('raw() builds an escape-hatch node from base64 ESC/POS bytes', () {
      final doc = PrintDocumentBuilder(paperKind: 'thermal-80').raw('AA==').build();
      expect(doc['content'], [
        {'type': 'raw', 'escposBase64': 'AA=='},
      ]);
    });

    test('build(title: ...) adds a meta.title field', () {
      final doc = PrintDocumentBuilder(paperKind: 'a4').build(title: 'Invoice #1');
      expect(doc['meta'], {'title': 'Invoice #1'});
    });

    test('build() omits meta entirely when no title is given', () {
      final doc = PrintDocumentBuilder(paperKind: 'a4').build();
      expect(doc.containsKey('meta'), isFalse);
    });
  });
}
