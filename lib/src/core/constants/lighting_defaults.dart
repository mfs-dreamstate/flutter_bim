import 'package:flutter/material.dart';

/// Default lighting configuration constants.
class LightingDefaults {
  LightingDefaults._();

  /// Default light direction X component (left/right).
  static const double directionX = 0.5;

  /// Default light direction Y component (down/up).
  static const double directionY = 0.8;

  /// Default light direction Z component (back/front).
  static const double directionZ = 0.3;

  /// Default light intensity.
  static const double intensity = 1.0;

  /// Default light color (warm white).
  static const Color lightColor = Color.fromRGBO(255, 250, 242, 1.0);

  /// Default ambient color (soft blue-gray).
  static const Color ambientColor = Color.fromRGBO(38, 43, 51, 1.0);

  /// Preset colors for the lighting color picker.
  static const List<Color> presetColors = [
    Color(0xFFFFFFFF), // White
    Color.fromRGBO(255, 250, 242, 1.0), // Warm white
    Color(0xFFFFE4B5), // Moccasin (warmer)
    Color(0xFFFFA500), // Orange
    Color(0xFFADD8E6), // Light blue
    Color(0xFF87CEEB), // Sky blue
    Color(0xFF90EE90), // Light green
    Color(0xFFFFB6C1), // Light pink
    Color(0xFF1A1A24), // Dark (for ambient)
    Color.fromRGBO(38, 43, 51, 1.0), // Blue-gray (default ambient)
  ];
}
