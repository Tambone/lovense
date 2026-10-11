import 'dart:convert';
import 'dart:io';

/// Empty means automatic discovery; a local HTTP address bypasses discovery.
String normalizeLovenseConnectAddress(String input) {
  final value = input.trim();
  if (value.isEmpty) return '';
  final uri = Uri.tryParse(value);
  final octets = uri?.host.split('.').map(int.tryParse).toList();
  final local =
      octets != null &&
      octets.length == 4 &&
      octets.every((v) => v != null && v >= 0 && v <= 255) &&
      (octets[0] == 10 ||
          octets[0] == 127 ||
          (octets[0] == 192 && octets[1] == 168) ||
          (octets[0] == 172 && octets[1]! >= 16 && octets[1]! <= 31));
  if (uri == null ||
      uri.scheme != 'http' ||
      !local ||
      !RegExp(
        r'^http://[0-9.]+:[0-9]+/?$',
        caseSensitive: false,
      ).hasMatch(value) ||
      uri.port < 1 ||
      uri.port > 65535 ||
      uri.userInfo.isNotEmpty ||
      (uri.path.isNotEmpty && uri.path != '/') ||
      uri.hasQuery ||
      uri.hasFragment) {
    throw const FormatException(
      'Use the local HTTP address and port shown by Lovense Connect, '
      'for example http://192.168.1.50:20010. Leave blank for automatic discovery.',
    );
  }
  return 'http://${uri.host}:${uri.port}';
}

/// This sidecar is read by the Rust runtime before starting the engine.
/// Keeping it beside the existing user-device config preserves the FFI ABI.
Future<void> writeLovenseConnectSettings(
  File userDeviceConfig,
  String address,
) async {
  final normalized = normalizeLovenseConnectAddress(address);
  final file = File(
    '${userDeviceConfig.parent.path}${Platform.pathSeparator}lovense-connect.json',
  );
  await file.parent.create(recursive: true);
  await file.writeAsString(jsonEncode({'address': normalized}), flush: true);
}
