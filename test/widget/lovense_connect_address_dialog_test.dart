import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intiface_central/widget/lovense_connect_address_dialog.dart';

void main() {
  Future<void> open(WidgetTester tester, ValueChanged<String> saved) async {
    await tester.pumpWidget(MaterialApp(home: Builder(builder: (context) => Scaffold(
      body: TextButton(onPressed: () => showDialog<void>(context: context, builder: (_) => LovenseConnectAddressDialog(initialValue: '', onSaved: saved)), child: const Text('Open')),
    ))));
    await tester.tap(find.text('Open')); await tester.pumpAndSettle();
  }
  testWidgets('invalid address stays open and valid phone address is saved', (tester) async {
    String? saved;
    await open(tester, (v) => saved = v);
    await tester.enterText(find.byKey(const Key('lovense-connect-address')), 'https://example.com');
    await tester.tap(find.byKey(const Key('save-lovense-connect-address'))); await tester.pumpAndSettle();
    expect(saved, isNull); expect(find.text('Lovense Connect phone address'), findsOneWidget);
    await tester.enterText(find.byKey(const Key('lovense-connect-address')), ' http://192.168.1.20:20010/ ');
    await tester.tap(find.byKey(const Key('save-lovense-connect-address'))); await tester.pumpAndSettle();
    expect(saved, 'http://192.168.1.20:20010'); expect(find.text('Lovense Connect phone address'), findsNothing);
  });
  testWidgets('blank selects automatic discovery; Cancel preserves settings', (tester) async {
    String? saved;
    await open(tester, (v) => saved = v);
    await tester.tap(find.byKey(const Key('save-lovense-connect-address'))); await tester.pumpAndSettle();
    expect(saved, '');
    saved = null;
    await open(tester, (v) => saved = v);
    await tester.enterText(find.byKey(const Key('lovense-connect-address')), 'http://192.168.1.20:20010');
    await tester.tap(find.text('Cancel')); await tester.pumpAndSettle();
    expect(saved, isNull);
  });
}
