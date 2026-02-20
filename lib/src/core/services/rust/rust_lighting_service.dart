import '../../bridge/api.dart' as rust;
import '../../errors/bim_error.dart';
import '../lighting_service.dart';

/// Rust implementation of [ILightingService].
class RustLightingService implements ILightingService {
  @override
  void setLightDirection({required double x, required double y, required double z}) {
    try {
      rust.setLightDirection(x: x, y: y, z: z);
    } catch (e) {
      throw BimRendererError(
        userMessage: 'Failed to set light direction',
        technicalMessage: 'setLightDirection($x, $y, $z) failed: $e',
        cause: e,
      );
    }
  }

  @override
  void setLightColor({required double r, required double g, required double b}) {
    try {
      rust.setLightColor(r: r, g: g, b: b);
    } catch (e) {
      throw BimRendererError(
        userMessage: 'Failed to set light color',
        technicalMessage: 'setLightColor($r, $g, $b) failed: $e',
        cause: e,
      );
    }
  }

  @override
  void setLightIntensity({required double intensity}) {
    try {
      rust.setLightIntensity(intensity: intensity);
    } catch (e) {
      throw BimRendererError(
        userMessage: 'Failed to set light intensity',
        technicalMessage: 'setLightIntensity($intensity) failed: $e',
        cause: e,
      );
    }
  }

  @override
  void setAmbientColor({required double r, required double g, required double b}) {
    try {
      rust.setAmbientColor(r: r, g: g, b: b);
    } catch (e) {
      throw BimRendererError(
        userMessage: 'Failed to set ambient color',
        technicalMessage: 'setAmbientColor($r, $g, $b) failed: $e',
        cause: e,
      );
    }
  }
}
