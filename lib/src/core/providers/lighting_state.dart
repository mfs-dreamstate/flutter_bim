import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../constants/lighting_defaults.dart';
import '../services/lighting_service.dart';
import 'service_providers.dart';

/// State for lighting settings.
@immutable
class LightingState {
  final double lightX;
  final double lightY;
  final double lightZ;
  final double intensity;
  final Color lightColor;
  final Color ambientColor;

  const LightingState({
    this.lightX = LightingDefaults.directionX,
    this.lightY = LightingDefaults.directionY,
    this.lightZ = LightingDefaults.directionZ,
    this.intensity = LightingDefaults.intensity,
    this.lightColor = LightingDefaults.lightColor,
    this.ambientColor = LightingDefaults.ambientColor,
  });

  LightingState copyWith({
    double? lightX,
    double? lightY,
    double? lightZ,
    double? intensity,
    Color? lightColor,
    Color? ambientColor,
  }) {
    return LightingState(
      lightX: lightX ?? this.lightX,
      lightY: lightY ?? this.lightY,
      lightZ: lightZ ?? this.lightZ,
      intensity: intensity ?? this.intensity,
      lightColor: lightColor ?? this.lightColor,
      ambientColor: ambientColor ?? this.ambientColor,
    );
  }
}

/// Notifier managing lighting settings.
class LightingNotifier extends Notifier<LightingState> {
  @override
  LightingState build() => const LightingState();

  ILightingService get _service => ref.read(lightingServiceProvider);

  void setDirection({double? x, double? y, double? z}) {
    state = state.copyWith(lightX: x, lightY: y, lightZ: z);
    _service.setLightDirection(
      x: state.lightX,
      y: state.lightY,
      z: state.lightZ,
    );
  }

  void setIntensity(double value) {
    state = state.copyWith(intensity: value);
    _service.setLightIntensity(intensity: value);
  }

  void setLightColor(Color color) {
    state = state.copyWith(lightColor: color);
    _service.setLightColor(
      r: color.r,
      g: color.g,
      b: color.b,
    );
  }

  void setAmbientColor(Color color) {
    state = state.copyWith(ambientColor: color);
    _service.setAmbientColor(
      r: color.r,
      g: color.g,
      b: color.b,
    );
  }

  void resetToDefaults() {
    state = const LightingState();
    _service.setLightDirection(
      x: LightingDefaults.directionX,
      y: LightingDefaults.directionY,
      z: LightingDefaults.directionZ,
    );
    _service.setLightIntensity(intensity: LightingDefaults.intensity);
    _service.setLightColor(
      r: LightingDefaults.lightColor.r,
      g: LightingDefaults.lightColor.g,
      b: LightingDefaults.lightColor.b,
    );
    _service.setAmbientColor(
      r: LightingDefaults.ambientColor.r,
      g: LightingDefaults.ambientColor.g,
      b: LightingDefaults.ambientColor.b,
    );
  }
}

/// Provider for the lighting state.
final lightingStateProvider =
    NotifierProvider<LightingNotifier, LightingState>(LightingNotifier.new);
