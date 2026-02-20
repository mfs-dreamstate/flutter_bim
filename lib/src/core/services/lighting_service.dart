/// Abstract interface for lighting operations.
abstract interface class ILightingService {
  /// Set the directional light direction.
  void setLightDirection({required double x, required double y, required double z});

  /// Set the directional light color (RGB, 0.0-1.0).
  void setLightColor({required double r, required double g, required double b});

  /// Set the directional light intensity.
  void setLightIntensity({required double intensity});

  /// Set the ambient light color (RGB, 0.0-1.0).
  void setAmbientColor({required double r, required double g, required double b});
}
