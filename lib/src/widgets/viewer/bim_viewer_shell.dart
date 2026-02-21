import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/constants/render_config.dart';
import '../../core/providers/accessibility_state.dart';
import '../../core/providers/model_state.dart';
import '../../core/providers/renderer_state.dart';
import '../../core/providers/selection_state.dart';
import '../../core/providers/service_providers.dart';
import '../element_tree.dart';
import '../model_manager.dart';
import 'viewer_body.dart';
import 'viewer_toolbar.dart';

/// The top-level shell for the BIM viewer.
/// Wraps everything in a ProviderScope and provides the Scaffold.
class BimViewerShell extends ConsumerStatefulWidget {
  const BimViewerShell({super.key});

  @override
  ConsumerState<BimViewerShell> createState() => _BimViewerShellState();
}

class _BimViewerShellState extends ConsumerState<BimViewerShell> {
  final GlobalKey<ScaffoldState> _scaffoldKey = GlobalKey<ScaffoldState>();

  @override
  void initState() {
    super.initState();
    // Initialize the renderer on first build
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _initializeAndLoad();
      // Re-trigger when model count changes (models loaded after viewer mounted)
      ref.listenManual(modelStateProvider.select((s) => s.modelCount), (prev, next) {
        if (next > 0 && (prev == null || prev == 0)) {
          _initializeAndLoad();
        }
      });
    });
  }

  /// Called on initState and whenever modelState changes to ensure
  /// the renderer is initialized and models are loaded.
  Future<void> _initializeAndLoad() async {
    final rendererNotifier = ref.read(rendererStateProvider.notifier);

    // Always attempt init — the notifier guards against double-init internally
    final rendererState = ref.read(rendererStateProvider);
    if (!rendererState.isInitialized) {
      // Calculate render size from screen dimensions
      final mq = MediaQuery.of(context);
      final dpr = mq.devicePixelRatio;
      final screenSize = mq.size;
      // Estimate viewport area (screen minus AppBar ~56dp, status bar, bottom nav ~80dp)
      final viewportHeight = screenSize.height - 136;
      final viewportWidth = screenSize.width;
      final renderWidth = (viewportWidth * dpr * RenderConfig.renderScale).round().clamp(240, 1920);
      final renderHeight = (viewportHeight * dpr * RenderConfig.renderScale).round().clamp(180, 1080);

      debugPrint('[BimViewerShell] Initializing renderer at ${renderWidth}x$renderHeight (scale=${RenderConfig.renderScale}, dpr=$dpr)...');
      await rendererNotifier.initialize(width: renderWidth, height: renderHeight);
    }

    final currentState = ref.read(rendererStateProvider);
    if (!currentState.isInitialized) {
      debugPrint('[BimViewerShell] Renderer init failed: ${currentState.error}');
      return;
    }

    final modelNotifier = ref.read(modelStateProvider.notifier);
    modelNotifier.refreshModelCount();

    // Load models if any are already parsed
    final modelService = ref.read(modelServiceProvider);
    if (modelService.isModelLoaded()) {
      try {
        debugPrint('[BimViewerShell] Loading models into renderer...');
        final result = await modelNotifier.loadAllModelsIntoRenderer();
        // Fit camera to see the loaded geometry
        final rendererService = ref.read(rendererServiceProvider);
        rendererService.fitCameraToAllModels();
        rendererNotifier.setStatus(result);
        debugPrint('[BimViewerShell] Models loaded: $result');
      } catch (e) {
        debugPrint('[BimViewerShell] Error loading models: $e');
      }
    }
  }

  void _onElementSelected(dynamic element) {
    ref.read(selectionStateProvider.notifier).selectElement(element);
    Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final modelState = ref.watch(modelStateProvider);
    final selectedElement = ref.watch(selectionStateProvider);
    final accessibilityState = ref.watch(accessibilityStateProvider);

    // Determine the effective text scaler: either the system value or custom.
    final effectiveTextScaler = accessibilityState.useSystemTextScale
        ? MediaQuery.of(context).textScaler
        : TextScaler.linear(accessibilityState.customTextScaleFactor);

    return MediaQuery(
      data: MediaQuery.of(context).copyWith(
        textScaler: effectiveTextScaler,
      ),
      child: Scaffold(
        key: _scaffoldKey,
        drawer: Semantics(
          label: 'Element tree navigation drawer',
          child: ElementTreeDrawer(
            onElementSelected: _onElementSelected,
            selectedElementId: selectedElement?.id,
          ),
        ),
        endDrawer: Semantics(
          label: 'Model manager drawer',
          child: ModelManagerDrawer(
            onModelsChanged: () =>
                ref.read(modelStateProvider.notifier).onModelsChanged(),
          ),
        ),
        appBar: AppBar(
          title: Semantics(
            header: true,
            child: Text(modelState.modelLoaded
                ? '3D Viewer - ${modelState.modelCount} Model${modelState.modelCount == 1 ? '' : 's'}'
                : '3D Viewer'),
          ),
          actions: [
            ViewerToolbar(scaffoldKey: _scaffoldKey),
          ],
        ),
        body: Semantics(
          label: '3D BIM Viewer viewport',
          hint: 'Drag to orbit, pinch to zoom, tap to select elements',
          child: const ViewerBody(),
        ),
      ),
    );
  }
}
