import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
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
      _initializeRenderer();
    });
  }

  Future<void> _initializeRenderer() async {
    final rendererNotifier = ref.read(rendererStateProvider.notifier);
    await rendererNotifier.initialize();

    final modelNotifier = ref.read(modelStateProvider.notifier);
    modelNotifier.refreshModelCount();

    // Load models if any are already parsed
    final modelService = ref.read(modelServiceProvider);
    if (modelService.isModelLoaded()) {
      try {
        final result = await modelNotifier.loadAllModelsIntoRenderer();
        rendererNotifier.setStatus(result);
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

    return Scaffold(
      key: _scaffoldKey,
      drawer: ElementTreeDrawer(
        onElementSelected: _onElementSelected,
        selectedElementId: selectedElement?.id,
      ),
      endDrawer: ModelManagerDrawer(
        onModelsChanged: () =>
            ref.read(modelStateProvider.notifier).onModelsChanged(),
      ),
      appBar: AppBar(
        title: Text(modelState.modelLoaded
            ? '3D Viewer - ${modelState.modelCount} Model${modelState.modelCount == 1 ? '' : 's'}'
            : '3D Viewer'),
        actions: [
          ViewerToolbar(scaffoldKey: _scaffoldKey),
        ],
      ),
      body: const ViewerBody(),
    );
  }
}
