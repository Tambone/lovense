import 'dart:convert';
import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:intiface_central/util/lovense_connect_settings.dart';

void main() {
  test('local phone address normalization and unsupported destinations', () {
    expect(normalizeLovenseConnectAddress(''), '');
    expect(normalizeLovenseConnectAddress(' http://192.168.1.5:20010/ '), 'http://192.168.1.5:20010');
    expect(normalizeLovenseConnectAddress('http://192.168.1.5:80'), 'http://192.168.1.5:80');
    for (final invalid in ['http://example.com:20010', 'https://192.168.1.5:20010', 'http://8.8.8.8:20010', 'http://192.168.1.5', 'http://192.168.1.5:0', 'http://192.168.1.5:70000', 'http://user:pass@192.168.1.5:20010', 'http://192.168.1.5:20010/GetToys', 'http://192.168.1.5:20010/?v=20']) {
      expect(() => normalizeLovenseConnectAddress(invalid), throwsFormatException, reason: invalid);
    }
  });
  test('persists manual address and clears it for discovery without touching device config', () async {
    final dir = await Directory.systemTemp.createTemp('intiface-lovense-');
    try {
      final config = File('${dir.path}/user-device-config.json');
      await config.writeAsString('{"original":true}');
      await writeLovenseConnectSettings(config, 'http://192.168.1.5:20010');
      final settings = File('${dir.path}/lovense-connect.json');
      expect(jsonDecode(await settings.readAsString()), {'address': 'http://192.168.1.5:20010'});
      await writeLovenseConnectSettings(config, '');
      expect(jsonDecode(await settings.readAsString()), {'address': ''});
      expect(await config.readAsString(), '{"original":true}');
    } finally { await dir.delete(recursive: true); }
  });
}
