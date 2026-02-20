import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_bim/src/core/providers/model_state.dart';
import 'package:flutter_bim/src/core/providers/service_providers.dart';
import '../../mocks/mock_services.dart';

void main() {
  group('ModelNotifier', () {
    late ProviderContainer container;
    late MockModelService mockModelService;

    setUp(() {
      mockModelService = MockModelService();
      container = ProviderContainer(
        overrides: [
          modelServiceProvider.overrideWithValue(mockModelService),
        ],
      );
    });

    tearDown(() {
      container.dispose();
    });

    test('initial state is empty', () {
      final state = container.read(modelStateProvider);
      expect(state.modelLoaded, isFalse);
      expect(state.modelCount, equals(0));
      expect(state.isLoading, isFalse);
    });

    test('refreshModelCount updates count from service', () {
      mockModelService.parseIfcContent(content: 'test');
      container.read(modelStateProvider.notifier).refreshModelCount();
      final state = container.read(modelStateProvider);
      expect(state.modelCount, equals(1));
    });

    test('loadAllModelsIntoRenderer sets modelLoaded', () async {
      mockModelService.parseIfcContent(content: 'test');
      await container
          .read(modelStateProvider.notifier)
          .loadAllModelsIntoRenderer();
      final state = container.read(modelStateProvider);
      expect(state.modelLoaded, isTrue);
    });

    test('setModelLoaded updates state', () {
      container.read(modelStateProvider.notifier).setModelLoaded(true);
      expect(container.read(modelStateProvider).modelLoaded, isTrue);

      container.read(modelStateProvider.notifier).setModelLoaded(false);
      expect(container.read(modelStateProvider).modelLoaded, isFalse);
    });
  });
}
