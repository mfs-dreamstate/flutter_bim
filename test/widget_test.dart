// BIM Viewer Widget Tests

import 'package:flutter_test/flutter_test.dart';

import 'package:bim_viewer/main.dart';

void main() {
  testWidgets('BIM Viewer app smoke test', (WidgetTester tester) async {
    // Build our app and trigger a frame.
    await tester.pumpWidget(const BimViewerApp());

    // Verify that the app title is present
    expect(find.text('BIM Viewer'), findsWidgets);

    // Note: Rust FFI initialization requires actual native library
    // which is not available in widget tests. Integration tests
    // should be used for FFI functionality testing.
  });
}
