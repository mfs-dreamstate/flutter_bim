import 'dart:typed_data';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_bim/src/core/bridge/bim/model.dart';
import 'package:flutter_bim/src/core/bridge/bim/geometry.dart';
import 'package:flutter_bim/src/core/bridge/lib.dart';
import 'package:flutter_bim/src/core/providers/selection_state.dart';
import 'package:flutter_bim/src/core/providers/service_providers.dart';
import '../../mocks/mock_services.dart';

ElementInfo _makeElement({int id = 1, String name = 'Test Wall', String type = 'Wall'}) {
  return ElementInfo(
    id: id,
    name: name,
    elementType: type,
    globalId: 'abc-$id',
    bounds: BoundingBox(
      min: F32Array3(Float32List.fromList([0, 0, 0])),
      max: F32Array3(Float32List.fromList([1, 1, 1])),
    ),
    triangleStart: 0,
    triangleCount: 2,
  );
}

void main() {
  group('SelectionNotifier', () {
    late ProviderContainer container;
    late MockSelectionService mockSelectionService;

    setUp(() {
      mockSelectionService = MockSelectionService();
      container = ProviderContainer(
        overrides: [
          selectionServiceProvider.overrideWithValue(mockSelectionService),
        ],
      );
    });

    tearDown(() {
      container.dispose();
    });

    test('initial state is null', () {
      expect(container.read(selectionStateProvider), isNull);
    });

    test('selectElement updates state and calls service', () {
      final element = _makeElement();
      container.read(selectionStateProvider.notifier).selectElement(element);

      expect(container.read(selectionStateProvider), equals(element));
      expect(mockSelectionService.selectedId, equals(1));
    });

    test('clearSelection resets to null', () {
      final element = _makeElement();
      container.read(selectionStateProvider.notifier).selectElement(element);
      container.read(selectionStateProvider.notifier).clearSelection();

      expect(container.read(selectionStateProvider), isNull);
      expect(mockSelectionService.selectedId, isNull);
    });

    test('pickElement with hit selects element', () {
      final element = _makeElement();
      mockSelectionService = MockSelectionService(elements: [element]);
      container = ProviderContainer(
        overrides: [
          selectionServiceProvider.overrideWithValue(mockSelectionService),
        ],
      );

      final picked = container
          .read(selectionStateProvider.notifier)
          .pickElement(screenX: 0.5, screenY: 0.5);

      expect(picked, isNotNull);
      expect(container.read(selectionStateProvider), equals(element));
    });

    test('pickElement with miss clears selection', () {
      container = ProviderContainer(
        overrides: [
          selectionServiceProvider
              .overrideWithValue(MockSelectionService(elements: [])),
        ],
      );

      // First select something
      final element = _makeElement();
      container.read(selectionStateProvider.notifier).selectElement(element);
      expect(container.read(selectionStateProvider), isNotNull);

      // Pick miss
      container
          .read(selectionStateProvider.notifier)
          .pickElement(screenX: 0.5, screenY: 0.5);
      expect(container.read(selectionStateProvider), isNull);
    });
  });
}
