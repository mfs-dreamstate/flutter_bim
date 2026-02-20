import '../../bridge/api.dart' as rust;
import '../../errors/bim_error.dart';
import '../visibility_service.dart';

/// Rust implementation of [IVisibilityService].
class RustVisibilityService implements IVisibilityService {
  @override
  void setElementTypeVisible({required String elementType, required bool visible}) {
    try {
      rust.setElementTypeVisible(elementType: elementType, visible: visible);
    } catch (e) {
      throw BimRendererError(
        userMessage: 'Failed to toggle element visibility',
        technicalMessage: 'setElementTypeVisible($elementType, $visible) failed: $e',
        cause: e,
      );
    }
  }

  @override
  bool isElementTypeVisible({required String elementType}) {
    return rust.isElementTypeVisible(elementType: elementType);
  }

  @override
  List<String> getHiddenElementTypes() => rust.getHiddenElementTypes();

  @override
  void setGridVisible({required bool visible}) {
    try {
      rust.setGridVisible(visible: visible);
    } catch (e) {
      throw BimRendererError(
        userMessage: 'Failed to set grid visibility',
        technicalMessage: 'setGridVisible($visible) failed: $e',
        cause: e,
      );
    }
  }

  @override
  bool isGridVisible() => rust.isGridVisible();
}
