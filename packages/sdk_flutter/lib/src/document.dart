/// Dart mirror of the universal PrintDocument JSON shape.
///
/// See docs/DOCUMENT_FORMAT.md at the repo root for the authoritative
/// schema. This intentionally stays a thin `Map<String, dynamic>` wrapper
/// rather than a fully typed class-per-node model (like `core-ts`/`core-rs`
/// have) — the Dart/Flutter side is the newest of the three SDKs and a
/// typed model can be generated later (e.g. via `json_serializable`) once
/// the schema has proven stable across the other two implementations
/// without duplicating maintenance three times over prematurely.
library;

typedef PrintDocument = Map<String, dynamic>;

/// Small helpers for building a valid PrintDocument by hand without
/// memorizing every field name. Covers every node type except `image`
/// (bitmap dithering needs the full renderer -- build an image node's raw
/// map by hand, or use `src/rust/api/render.dart`, if you need one).
class PrintDocumentBuilder {
  final String paperKind;
  final double? widthMm;
  final int? charsPerLine;
  final List<Map<String, dynamic>> _content = [];

  PrintDocumentBuilder({
    required this.paperKind,
    this.widthMm,
    this.charsPerLine,
  });

  PrintDocumentBuilder text(
    String value, {
    String align = 'left',
    bool bold = false,
    bool underline = false,
    bool invert = false,
    int sizeW = 1,
    int sizeH = 1,
    String font = 'a',
  }) {
    _content.add({
      'type': 'text',
      'value': value,
      'align': align,
      'bold': bold,
      'underline': underline,
      'invert': invert,
      'size': {'w': sizeW, 'h': sizeH},
      'font': font,
    });
    return this;
  }

  PrintDocumentBuilder row(
    List<({String text, num width, String align})> columns, {
    bool bold = false,
  }) {
    _content.add({
      'type': 'row',
      'columns': [
        for (final c in columns)
          {'text': c.text, 'width': c.width, 'align': c.align},
      ],
      'bold': bold,
    });
    return this;
  }

  PrintDocumentBuilder rule({String char = '-'}) {
    _content.add({'type': 'rule', 'char': char});
    return this;
  }

  PrintDocumentBuilder feed({int lines = 1}) {
    _content.add({'type': 'feed', 'lines': lines});
    return this;
  }

  /// `symbology` must be one of 'code128', 'code39', 'ean13', 'upca' -- see
  /// docs/DOCUMENT_FORMAT.md.
  PrintDocumentBuilder barcode(
    String symbology,
    String data, {
    int heightPx = 60,
    bool showText = true,
  }) {
    _content.add({
      'type': 'barcode',
      'symbology': symbology,
      'data': data,
      'heightPx': heightPx,
      'showText': showText,
    });
    return this;
  }

  PrintDocumentBuilder qrcode(String data, {int size = 6, String ec = 'M'}) {
    _content.add({'type': 'qrcode', 'data': data, 'size': size, 'ec': ec});
    return this;
  }

  PrintDocumentBuilder cut({String mode = 'partial'}) {
    _content.add({'type': 'cut', 'mode': mode});
    return this;
  }

  PrintDocumentBuilder drawer({int pin = 2}) {
    _content.add({'type': 'drawer', 'pin': pin});
    return this;
  }

  /// Escape hatch for printer-specific ESC/POS bytes the schema doesn't
  /// model yet -- ignored by HTML/SVG/PDF renderers, see
  /// docs/DOCUMENT_FORMAT.md.
  PrintDocumentBuilder raw(String escposBase64) {
    _content.add({'type': 'raw', 'escposBase64': escposBase64});
    return this;
  }

  PrintDocument build({String? title}) {
    return {
      'version': '1.0',
      'paper': {
        'kind': paperKind,
        if (widthMm != null) 'widthMm': widthMm,
        if (charsPerLine != null) 'charsPerLine': charsPerLine,
      },
      if (title != null) 'meta': {'title': title},
      'content': _content,
    };
  }
}
