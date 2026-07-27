// Hardware test app for sdk_flutter's delivery transports (TCP, USB,
// Bluetooth BLE) -- not a demo of the plugin's platform-channel scaffold.
// Builds a sample receipt with EscPosBuilder and sends it through whichever
// transport panel you use.
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:sdk_flutter/sdk_flutter.dart';

void main() {
  runApp(const TestApp());
}

Uint8List buildSampleReceipt() {
  return (EscPosBuilder()
        ..align('center')
        ..bold(true)
        ..size(2, 2)
        ..text('PRINTERS SDK')
        ..newline()
        ..bold(false)
        ..size(1, 1)
        ..text('Flutter/Android hardware test')
        ..newline()
        ..align('left')
        ..rule(32)
        ..text('If this printed, the transport works.')
        ..newline()
        ..feed(2)
        ..cut())
      .toBytes();
}

class TestApp extends StatelessWidget {
  const TestApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Printers SDK - Flutter Test',
      theme: ThemeData.dark(useMaterial3: true),
      home: const TestHomePage(),
    );
  }
}

class TestHomePage extends StatefulWidget {
  const TestHomePage({super.key});

  @override
  State<TestHomePage> createState() => _TestHomePageState();
}

class _TestHomePageState extends State<TestHomePage> {
  final List<String> _log = [];

  final _tcpHostCtrl = TextEditingController(text: '192.168.1.50');
  final _tcpPortCtrl = TextEditingController(text: '9100');

  List<UsbDevice> _usbDevices = [];
  bool _usbBusy = false;

  List<BluetoothDevice> _bleDevices = [];
  bool _bleBusy = false;

  void _log_(String msg) {
    final now = DateTime.now();
    final time = '${now.hour.toString().padLeft(2, '0')}:${now.minute.toString().padLeft(2, '0')}:${now.second.toString().padLeft(2, '0')}';
    setState(() => _log.insert(0, '[$time] $msg'));
  }

  Future<void> _listUsb() async {
    setState(() => _usbBusy = true);
    try {
      final devices = await listUsbPrinters();
      setState(() => _usbDevices = devices);
      _log_('Found ${devices.length} USB device(s)');
    } catch (e) {
      _log_('USB list failed: $e');
    } finally {
      setState(() => _usbBusy = false);
    }
  }

  Future<void> _printUsb(UsbDevice device) async {
    setState(() => _usbBusy = true);
    UsbPort? port;
    try {
      port = await openUsbPrinter(device);
      await sendToUsbPrinter(port, buildSampleReceipt());
      _log_('Sent to USB "${device.productName}" OK');
    } catch (e) {
      _log_('USB print to "${device.productName}" failed: $e');
    } finally {
      await port?.close();
      setState(() => _usbBusy = false);
    }
  }

  Future<void> _scanBle() async {
    setState(() => _bleBusy = true);
    try {
      final statuses = await [
        Permission.bluetoothScan,
        Permission.bluetoothConnect,
        Permission.locationWhenInUse,
      ].request();
      final denied = statuses.entries.where((e) => !e.value.isGranted).toList();
      if (denied.isNotEmpty) {
        _log_('Permission(s) not granted: ${denied.map((e) => e.key).join(", ")} -- scan will likely find nothing');
      }
      final devices = await scanBluetoothPrinters();
      setState(() => _bleDevices = devices);
      _log_('Found ${devices.length} BLE device(s). Classic-Bluetooth-only '
          'printers (e.g. many cheap "Y-series" thermal printers using SPP, '
          'not BLE) will NOT appear here -- this transport is BLE-only.');
    } catch (e) {
      _log_('BLE scan failed: $e');
    } finally {
      setState(() => _bleBusy = false);
    }
  }

  Future<void> _printBle(BluetoothDevice device) async {
    setState(() => _bleBusy = true);
    try {
      final characteristic = await connectBluetoothPrinter(device);
      await sendToBluetoothPrinter(characteristic, buildSampleReceipt());
      _log_('Sent to BLE "${device.platformName}" OK');
    } catch (e) {
      _log_('BLE print to "${device.platformName}" failed: $e');
    } finally {
      await device.disconnect();
      setState(() => _bleBusy = false);
    }
  }

  Future<void> _printTcp() async {
    final host = _tcpHostCtrl.text.trim();
    final port = int.tryParse(_tcpPortCtrl.text.trim()) ?? 9100;
    try {
      await sendToTcpPrinter(host, port, buildSampleReceipt());
      _log_('Sent to tcp://$host:$port OK');
    } catch (e) {
      _log_('TCP send to $host:$port failed: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Printers SDK -- Flutter Test')),
      body: ListView(
        padding: const EdgeInsets.all(12),
        children: [
          Card(
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('1. USB (Android USB Host API)', style: TextStyle(fontWeight: FontWeight.bold)),
                  const SizedBox(height: 8),
                  FilledButton(onPressed: _usbBusy ? null : _listUsb, child: const Text('List USB devices')),
                  for (final d in _usbDevices)
                    ListTile(
                      title: Text(d.productName ?? 'Unknown USB device'),
                      subtitle: Text('vendorId=${d.vid} productId=${d.pid}'),
                      trailing: FilledButton(
                        onPressed: _usbBusy ? null : () => _printUsb(d),
                        child: const Text('Print test'),
                      ),
                    ),
                ],
              ),
            ),
          ),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('2. Bluetooth (BLE only)', style: TextStyle(fontWeight: FontWeight.bold)),
                  const SizedBox(height: 8),
                  FilledButton(onPressed: _bleBusy ? null : _scanBle, child: const Text('Scan 5s')),
                  for (final d in _bleDevices)
                    ListTile(
                      title: Text(d.platformName.isEmpty ? '(unnamed)' : d.platformName),
                      subtitle: Text(d.remoteId.str),
                      trailing: FilledButton(
                        onPressed: _bleBusy ? null : () => _printBle(d),
                        child: const Text('Print test'),
                      ),
                    ),
                ],
              ),
            ),
          ),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('3. WiFi/TCP printer', style: TextStyle(fontWeight: FontWeight.bold)),
                  const SizedBox(height: 8),
                  Row(
                    children: [
                      Expanded(child: TextField(controller: _tcpHostCtrl, decoration: const InputDecoration(labelText: 'Host'))),
                      const SizedBox(width: 8),
                      SizedBox(width: 80, child: TextField(controller: _tcpPortCtrl, decoration: const InputDecoration(labelText: 'Port'))),
                    ],
                  ),
                  const SizedBox(height: 8),
                  FilledButton(onPressed: _printTcp, child: const Text('Send test receipt')),
                ],
              ),
            ),
          ),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('Log', style: TextStyle(fontWeight: FontWeight.bold)),
                  const Divider(),
                  for (final line in _log) Text(line, style: const TextStyle(fontFamily: 'monospace', fontSize: 12)),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
