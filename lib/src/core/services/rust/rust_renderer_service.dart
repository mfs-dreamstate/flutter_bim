import 'package:flutter/foundation.dart';
import '../../bridge/api.dart' as rust;
import '../../errors/bim_error.dart';
import '../renderer_service.dart';

/// Rust/wgpu implementation of [IRendererService].
class RustRendererService implements IRendererService {
  @override
  Future<String> initialize({required int width, required int height}) async {
    try {
      return await rust.initRenderer(width: width, height: height);
    } catch (e) {
      throw BimRendererError(
        userMessage: 'Failed to initialize renderer',
        technicalMessage: 'initRenderer($width x $height) failed: $e',
        cause: e,
      );
    }
  }

  @override
  bool isInitialized() => rust.isRendererInitialized();

  @override
  Uint8List renderFrame() {
    try {
      return rust.renderFrame();
    } catch (e) {
      throw BimRendererError(
        userMessage: 'Render error',
        technicalMessage: 'renderFrame failed: $e',
        cause: e,
      );
    }
  }

  @override
  void orbitCamera({required double deltaX, required double deltaY}) {
    try {
      rust.orbitCamera(deltaX: deltaX, deltaY: deltaY);
    } catch (e) {
      debugPrint('[RustRendererService] orbitCamera failed: $e');
    }
  }

  @override
  void zoomCamera({required double delta}) {
    try {
      rust.zoomCamera(delta: delta);
    } catch (e) {
      debugPrint('[RustRendererService] zoomCamera failed: $e');
    }
  }

  @override
  void panCamera({required double deltaX, required double deltaY}) {
    try {
      rust.panCamera(deltaX: deltaX, deltaY: deltaY);
    } catch (e) {
      debugPrint('[RustRendererService] panCamera failed: $e');
    }
  }

  @override
  void fitCameraToAllModels() {
    try {
      rust.fitCameraToAllModels();
    } catch (e) {
      throw BimRendererError(
        userMessage: 'Failed to fit camera',
        technicalMessage: 'fitCameraToAllModels failed: $e',
        cause: e,
      );
    }
  }

  @override
  void fitCameraToModel() {
    try {
      rust.fitCameraToModel();
    } catch (e) {
      throw BimRendererError(
        userMessage: 'Failed to fit camera',
        technicalMessage: 'fitCameraToModel failed: $e',
        cause: e,
      );
    }
  }

  @override
  void setRenderMode({required int mode}) {
    try {
      rust.setRenderMode(mode: mode);
    } catch (e) {
      throw BimRendererError(
        userMessage: 'Failed to set render mode',
        technicalMessage: 'setRenderMode($mode) failed: $e',
        cause: e,
      );
    }
  }

  @override
  int getRenderMode() => rust.getRenderMode();

  @override
  bool isWireframeSupported() => rust.isWireframeSupported();

  @override
  void setInteractionActive({required bool active}) {
    try {
      rust.setInteractionActive(active: active);
    } catch (e) {
      debugPrint('[RustRendererService] setInteractionActive failed: $e');
    }
  }

  @override
  List<double> getSceneBounds() {
    try {
      final bounds = rust.getSceneBounds();
      return bounds.map((e) => e.toDouble()).toList();
    } catch (e) {
      debugPrint('[RustRendererService] getSceneBounds failed: $e');
      return [-50, -50, -50, 50, 50, 50];
    }
  }

  @override
  void flyCamera({required double forward, required double right, required double up}) {
    try {
      rust.flyCamera(forward: forward, right: right, up: up);
    } catch (e) {
      debugPrint('[RustRendererService] flyCamera failed: $e');
    }
  }

  @override
  void lookCamera({required double deltaX, required double deltaY}) {
    try {
      rust.lookCamera(deltaX: deltaX, deltaY: deltaY);
    } catch (e) {
      debugPrint('[RustRendererService] lookCamera failed: $e');
    }
  }

  @override
  void setWalkthroughMode({required bool enabled}) {
    try {
      rust.setWalkthroughMode(enabled: enabled);
    } catch (e) {
      debugPrint('[RustRendererService] setWalkthroughMode failed: $e');
    }
  }

  @override
  bool setOrbitCenterFromScreen({required double screenX, required double screenY}) {
    try {
      return rust.setOrbitCenterFromScreen(screenX: screenX, screenY: screenY);
    } catch (e) {
      debugPrint('[RustRendererService] setOrbitCenterFromScreen failed: $e');
      return false;
    }
  }
}
