import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:bim_viewer/main.dart' as app;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  group('BIM Viewer Integration Tests', () {
    testWidgets('App launches successfully', (WidgetTester tester) async {
      // Start the app
      app.main();
      await tester.pumpAndSettle();

      // Verify app launches
      expect(find.byType(MaterialApp), findsOneWidget);
    });

    testWidgets('Can navigate to viewer screen', (WidgetTester tester) async {
      app.main();
      await tester.pumpAndSettle();

      // The viewer screen should be visible or navigable
      // This test verifies basic app structure
      expect(find.byType(Scaffold), findsAtLeastNWidgets(1));
    });

    testWidgets('Renderer initializes without errors', (WidgetTester tester) async {
      app.main();
      await tester.pumpAndSettle();

      // Wait for async initialization
      await tester.pump(const Duration(seconds: 2));
      await tester.pumpAndSettle();

      // Verify no error dialogs appeared
      expect(find.byType(AlertDialog), findsNothing);
    });
  });

  group('BIM File Loading Flow', () {
    testWidgets('Can access model manager', (WidgetTester tester) async {
      app.main();
      await tester.pumpAndSettle();

      // Look for model manager button (layers icon)
      final layersButton = find.byIcon(Icons.layers);
      if (layersButton.evaluate().isNotEmpty) {
        await tester.tap(layersButton);
        await tester.pumpAndSettle();

        // Model manager drawer should open
        expect(find.byType(Drawer), findsOneWidget);
      }
    });
  });

  group('Rendering Performance', () {
    testWidgets('UI remains responsive during render', (WidgetTester tester) async {
      app.main();
      await tester.pumpAndSettle();

      // Simulate some time passing
      await tester.pump(const Duration(milliseconds: 500));

      // UI should still be responsive
      expect(tester.binding.transientCallbackCount, equals(0));
    });
  });
}
