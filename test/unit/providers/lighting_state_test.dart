import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_bim/src/core/constants/lighting_defaults.dart';
import 'package:flutter_bim/src/core/providers/lighting_state.dart';
import 'package:flutter_bim/src/core/providers/service_providers.dart';
import '../../mocks/mock_services.dart';

void main() {
  group('LightingNotifier', () {
    late ProviderContainer container;
    late MockLightingService mockLightingService;

    setUp(() {
      mockLightingService = MockLightingService();
      container = ProviderContainer(
        overrides: [
          lightingServiceProvider.overrideWithValue(mockLightingService),
        ],
      );
    });

    tearDown(() {
      container.dispose();
    });

    test('initial state uses defaults', () {
      final state = container.read(lightingStateProvider);
      expect(state.lightX, equals(LightingDefaults.directionX));
      expect(state.lightY, equals(LightingDefaults.directionY));
      expect(state.lightZ, equals(LightingDefaults.directionZ));
      expect(state.intensity, equals(LightingDefaults.intensity));
      expect(state.lightColor, equals(LightingDefaults.lightColor));
      expect(state.ambientColor, equals(LightingDefaults.ambientColor));
    });

    test('setDirection updates state and calls service', () {
      container
          .read(lightingStateProvider.notifier)
          .setDirection(x: -0.5);

      final state = container.read(lightingStateProvider);
      expect(state.lightX, equals(-0.5));
      expect(state.lightY, equals(LightingDefaults.directionY));
      expect(mockLightingService.lastX, equals(-0.5));
    });

    test('setIntensity updates state and calls service', () {
      container.read(lightingStateProvider.notifier).setIntensity(2.5);

      final state = container.read(lightingStateProvider);
      expect(state.intensity, equals(2.5));
      expect(mockLightingService.lastIntensity, equals(2.5));
    });

    test('setLightColor updates state and calls service', () {
      const newColor = Color(0xFFFF0000);
      container.read(lightingStateProvider.notifier).setLightColor(newColor);

      final state = container.read(lightingStateProvider);
      expect(state.lightColor, equals(newColor));
    });

    test('resetToDefaults restores initial values', () {
      // Change some values
      container
          .read(lightingStateProvider.notifier)
          .setDirection(x: -1.0, y: -1.0, z: -1.0);
      container.read(lightingStateProvider.notifier).setIntensity(3.0);

      // Reset
      container.read(lightingStateProvider.notifier).resetToDefaults();

      final state = container.read(lightingStateProvider);
      expect(state.lightX, equals(LightingDefaults.directionX));
      expect(state.lightY, equals(LightingDefaults.directionY));
      expect(state.lightZ, equals(LightingDefaults.directionZ));
      expect(state.intensity, equals(LightingDefaults.intensity));
      expect(state.lightColor, equals(LightingDefaults.lightColor));
      expect(state.ambientColor, equals(LightingDefaults.ambientColor));
    });
  });
}
