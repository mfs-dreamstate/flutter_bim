import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_bim/src/widgets/viewer/bim_viewer_shell.dart';
import 'helpers/test_app.dart';

void main() {
  group('ViewerScreen Widget Tests', () {
    testWidgets('BimViewerShell renders without crashing', (WidgetTester tester) async {
      await tester.pumpWidget(
        createFullyMockedTestApp(
          child: const BimViewerShell(),
        ),
      );

      expect(find.byType(BimViewerShell), findsOneWidget);
    });

    testWidgets('BimViewerShell shows status text initially', (WidgetTester tester) async {
      await tester.pumpWidget(
        createFullyMockedTestApp(
          child: const BimViewerShell(),
        ),
      );

      expect(find.byType(Text), findsWidgets);
    });

    testWidgets('BimViewerShell has app bar', (WidgetTester tester) async {
      await tester.pumpWidget(
        createFullyMockedTestApp(
          child: const BimViewerShell(),
        ),
      );

      expect(find.byType(AppBar), findsOneWidget);
    });

    testWidgets('Camera controls are present after init', (WidgetTester tester) async {
      await tester.pumpWidget(
        createFullyMockedTestApp(
          child: const BimViewerShell(),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.byType(GestureDetector), findsWidgets);
    });
  });
}
