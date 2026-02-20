import '../../bridge/api.dart' as rust;
import '../../errors/bim_error.dart';
import '../overlay_service.dart';

/// Rust implementation of [IOverlayService].
class RustOverlayService implements IOverlayService {
  @override
  Future<void> uploadDrawingOverlay({
    required String id,
    required int width,
    required int height,
    required List<int> rgbaPixels,
  }) async {
    try {
      await rust.uploadDrawingOverlay(
        id: id,
        width: width,
        height: height,
        rgbaPixels: rgbaPixels,
      );
    } catch (e) {
      throw BimOverlayError(
        userMessage: 'Failed to upload overlay',
        technicalMessage: 'uploadDrawingOverlay($id) failed: $e',
        cause: e,
      );
    }
  }

  @override
  void setOverlayTransform({
    required String id,
    required double positionX,
    required double positionY,
    required double positionZ,
    required double scaleX,
    required double scaleY,
    required double rotation,
  }) {
    try {
      rust.setOverlayTransform(
        id: id,
        positionX: positionX,
        positionY: positionY,
        positionZ: positionZ,
        scaleX: scaleX,
        scaleY: scaleY,
        rotation: rotation,
      );
    } catch (e) {
      throw BimOverlayError(
        userMessage: 'Failed to update overlay transform',
        technicalMessage: 'setOverlayTransform($id) failed: $e',
        cause: e,
      );
    }
  }

  @override
  void setOverlayOpacity({required String id, required double opacity}) {
    try {
      rust.setOverlayOpacity(id: id, opacity: opacity);
    } catch (e) {
      throw BimOverlayError(
        userMessage: 'Failed to set overlay opacity',
        technicalMessage: 'setOverlayOpacity($id, $opacity) failed: $e',
        cause: e,
      );
    }
  }

  @override
  void setOverlayVisible({required String id, required bool visible}) {
    try {
      rust.setOverlayVisible(id: id, visible: visible);
    } catch (e) {
      throw BimOverlayError(
        userMessage: 'Failed to toggle overlay visibility',
        technicalMessage: 'setOverlayVisible($id, $visible) failed: $e',
        cause: e,
      );
    }
  }

  @override
  void removeOverlay({required String id}) {
    try {
      rust.removeOverlay(id: id);
    } catch (e) {
      throw BimOverlayError(
        userMessage: 'Failed to remove overlay',
        technicalMessage: 'removeOverlay($id) failed: $e',
        cause: e,
      );
    }
  }

  @override
  void setViewMode({required String mode}) {
    try {
      rust.setViewMode(mode: mode);
    } catch (e) {
      throw BimOverlayError(
        userMessage: 'Failed to set view mode',
        technicalMessage: 'setViewMode($mode) failed: $e',
        cause: e,
      );
    }
  }

  @override
  String getViewMode() => rust.getViewMode();
}
