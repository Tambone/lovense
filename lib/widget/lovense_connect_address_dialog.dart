import 'package:flutter/material.dart';
import 'package:intiface_central/util/lovense_connect_settings.dart';

class LovenseConnectAddressDialog extends StatefulWidget {
  const LovenseConnectAddressDialog({
    super.key,
    required this.initialValue,
    required this.onSaved,
  });
  final String initialValue;
  final ValueChanged<String> onSaved;

  @override
  State<LovenseConnectAddressDialog> createState() =>
      _LovenseConnectAddressDialogState();
}

class _LovenseConnectAddressDialogState
    extends State<LovenseConnectAddressDialog> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.initialValue,
  );
  String? _error;

  void _save() {
    try {
      final value = normalizeLovenseConnectAddress(_controller.text);
      widget.onSaved(value);
      Navigator.of(context).pop();
    } on FormatException catch (error) {
      setState(() => _error = error.message);
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text('Lovense Connect phone address'),
    content: SizedBox(
      width: 440,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Keep Lovense Connect open on your phone with your toy connected. '
            'Use the same Wi-Fi as this computer. On iPhone, allow Local Network access for Connect. '
            'Enter the local IP address and HTTP port shown by Connect. '
            'Leave blank to use automatic discovery.',
          ),
          const SizedBox(height: 16),
          TextField(
            key: const Key('lovense-connect-address'),
            controller: _controller,
            autofocus: true,
            keyboardType: TextInputType.url,
            autocorrect: false,
            onSubmitted: (_) => _save(),
            decoration: InputDecoration(
              labelText: 'Phone HTTP address',
              hintText: 'http://192.168.1.50:20010',
              errorText: _error,
              errorMaxLines: 4,
            ),
          ),
        ],
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.of(context).pop(),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: const Key('save-lovense-connect-address'),
        onPressed: _save,
        child: const Text('Save'),
      ),
    ],
  );
}
