import '../../bridge/api.dart' as rust;
import '../../errors/bim_error.dart';
import '../section_plane_service.dart';

/// Rust implementation of [ISectionPlaneService].
class RustSectionPlaneService implements ISectionPlaneService {
  @override
  void setSectionPlane({
    required double originX,
    required double originY,
    required double originZ,
    required double normalX,
    required double normalY,
    required double normalZ,
  }) {
    try {
      rust.setSectionPlane(
        originX: originX,
        originY: originY,
        originZ: originZ,
        normalX: normalX,
        normalY: normalY,
        normalZ: normalZ,
      );
    } catch (e) {
      throw BimSectionPlaneError(
        userMessage: 'Failed to set section plane',
        technicalMessage: 'setSectionPlane failed: $e',
        cause: e,
      );
    }
  }

  @override
  void setSectionPlaneEnabled({required bool enabled}) {
    try {
      rust.setSectionPlaneEnabled(enabled: enabled);
    } catch (e) {
      throw BimSectionPlaneError(
        userMessage: 'Failed to toggle section plane',
        technicalMessage: 'setSectionPlaneEnabled($enabled) failed: $e',
        cause: e,
      );
    }
  }

  @override
  void clearSectionPlane() {
    try {
      rust.clearSectionPlane();
    } catch (e) {
      throw BimSectionPlaneError(
        userMessage: 'Failed to clear section plane',
        technicalMessage: 'clearSectionPlane failed: $e',
        cause: e,
      );
    }
  }

  @override
  bool isSectionPlaneActive() => rust.isSectionPlaneActive();

  @override
  void setSectionPlaneFromAxis({required int axis, required double position}) {
    try {
      rust.setSectionPlaneFromAxis(axis: axis, position: position);
    } catch (e) {
      throw BimSectionPlaneError(
        userMessage: 'Failed to set section plane',
        technicalMessage: 'setSectionPlaneFromAxis($axis, $position) failed: $e',
        cause: e,
      );
    }
  }
}
