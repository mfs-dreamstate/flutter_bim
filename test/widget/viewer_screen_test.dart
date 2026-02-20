import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_bim/src/widgets/viewer/bim_viewer_shell.dart';
import '../helpers/test_app.dart';

void main() {
  group('BimViewerShell Widget Tests', () {
    testWidgets('renders without crashing', (WidgetTester tester) async {
      await tester.pumpWidget(
        createFullyMockedTestApp(
          child: const BimViewerShell(),
        ),
      );

      expect(find.byType(BimViewerShell), findsOneWidget);
    });

    testWidgets('has app bar with title', (WidgetTester tester) async {
      await tester.pumpWidget(
        createFullyMockedTestApp(
          child: const BimViewerShell(),
        ),
      );

      expect(find.byType(AppBar), findsOneWidget);
      expect(find.text('3D Viewer'), findsOneWidget);
    });

    testWidgets('shows loading state initially', (WidgetTester tester) async {
      await tester.pumpWidget(
        createFullyMockedTestApp(
          child: const BimViewerShell(),
        ),
      );

      // Initially shows loading indicator before renderer initializes
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
    });

    testWidgets('shows viewport after initialization', (WidgetTester tester) async {
      await tester.pumpWidget(
        createFullyMockedTestApp(
          child: const BimViewerShell(),
        ),
      );

      await tester.pumpAndSettle();

      // After init, should show control hints
      expect(find.text('Tap to select'), findsOneWidget);
      expect(find.text('Drag to orbit'), findsOneWidget);
      expect(find.text('Pinch to zoom'), findsOneWidget);
    });

    testWidgets('Model manager button is always visible', (WidgetTester tester) async {
      await tester.pumpWidget(
        createFullyMockedTestApp(
          child: const BimViewerShell(),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.byTooltip('Model Manager'), findsOneWidget);
    });

    testWidgets('Settings button is visible', (WidgetTester tester) async {
      await tester.pumpWidget(
        createFullyMockedTestApp(
          child: const BimViewerShell(),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.byTooltip('Settings'), findsOneWidget);
    });

    testWidgets('Lighting button is visible', (WidgetTester tester) async {
      await tester.pumpWidget(
        createFullyMockedTestApp(
          child: const BimViewerShell(),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.byTooltip('Lighting Settings'), findsOneWidget);
    });

    testWidgets('Restart renderer button is visible', (WidgetTester tester) async {
      await tester.pumpWidget(
        createFullyMockedTestApp(
          child: const BimViewerShell(),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.byTooltip('Restart Renderer'), findsOneWidget);
    });
  });
}
