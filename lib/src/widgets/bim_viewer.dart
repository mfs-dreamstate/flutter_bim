import 'dart:async';
import 'dart:typed_data';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import '../core/bridge/api.dart' as rust;
import '../core/bridge/bim/model.dart';
import 'model_manager.dart';
import 'properties_panel.dart';
import 'element_tree.dart';
import 'grid_overlay.dart';
import 'measurement_tools.dart';
import 'section_plane_tools.dart';
import 'settings_screen.dart';
import 'drawing_overlay_manager.dart';

class ViewerScreen extends StatefulWidget {
  const ViewerScreen({super.key});

  @override
  State<ViewerScreen> createState() => _ViewerScreenState();
}

class _ViewerScreenState extends State<ViewerScreen> {
  bool _isInitializing = true;
  String _status = 'Initializing renderer...';
  String? _error;
  ui.Image? _frameImage;
  Timer? _renderTimer;
  bool _modelLoaded = false;

  // Selected element info
  ElementInfo? _selectedElement;

  // Visibility toggles
  Map<String, bool> _visibility = {
    'Wall': true,
    'Slab': true,
    'Column': true,
    'Beam': true,
    'Door': true,
    'Window': true,
    'Roof': true,
  };

  // Lighting settings
  double _lightX = 0.5;
  double _lightY = 0.8;
  double _lightZ = 0.3;
  double _lightIntensity = 1.0;
  Color _lightColor = const Color.fromRGBO(255, 250, 242, 1.0); // warm white
  Color _ambientColor = const Color.fromRGBO(38, 43, 51, 1.0); // soft blue-gray

  // Render mode: 0 = Shaded, 1 = Wireframe
  int _renderMode = 0;
  bool _wireframeSupported = false;

  // Grid visibility
  bool _gridVisible = true;

  // Renderer dimensions (reduced for mobile performance, will be scaled up)
  static const int _width = 480;
  static const int _height = 360;

  // Touch tracking for gestures
  Offset? _lastPanPosition;
  double _lastScale = 1.0;
  Offset? _tapPosition;

  // Frame rendering state
  bool _isRendering = false;

  // Gesture throttling
  DateTime _lastGestureTime = DateTime.now();
  static const _gestureThrottleMs = 16; // ~60Hz max gesture rate

  // Scaffold key for drawer
  final GlobalKey<ScaffoldState> _scaffoldKey = GlobalKey<ScaffoldState>();

  // Model count for display
  int _modelCount = 0;

  @override
  void initState() {
    super.initState();
    _initializeRenderer();
  }

  @override
  void dispose() {
    _renderTimer?.cancel();
    super.dispose();
  }

  Future<void> _initializeRenderer() async {
    try {
      setState(() {
        _isInitializing = true;
        _status = 'Initializing GPU...';
      });

      debugPrint('[RENDERER] Initializing renderer ${_width}x$_height...');
      final result = await rust.initRenderer(width: _width, height: _height);
      debugPrint('[RENDERER] Initialized: $result');

      // Check wireframe support
      final wireframeSupported = rust.isWireframeSupported();
      debugPrint('[RENDERER] Wireframe supported: $wireframeSupported');

      // Refresh model count
      _refreshModelCount();

      setState(() {
        _status = result;
        _isInitializing = false;
        _wireframeSupported = wireframeSupported;
      });

      // Start render loop
      _startRenderLoop();

      // Check if any models are loaded and load them into renderer
      if (rust.isModelLoaded()) {
        _loadModelIntoRenderer();
      }
    } catch (e, stackTrace) {
      debugPrint('[RENDERER ERROR] Failed to initialize: $e');
      debugPrint('[RENDERER ERROR] Stack trace: $stackTrace');
      setState(() {
        _error = e.toString();
        _isInitializing = false;
      });
    }
  }

  Future<void> _loadModelIntoRenderer() async {
    try {
      debugPrint('[RENDERER] Loading all models into renderer...');
      final result = rust.loadAllModelsIntoRenderer();
      debugPrint('[RENDERER] Models loaded: $result');
      _refreshModelCount();
      setState(() {
        _modelLoaded = true;
        _status = result;
      });
    } catch (e, stackTrace) {
      debugPrint('[RENDERER ERROR] Failed to load models: $e');
      debugPrint('[RENDERER ERROR] Stack trace: $stackTrace');
      setState(() {
        _error = 'Failed to load models: $e';
      });
    }
  }

  void _refreshModelCount() {
    try {
      final count = rust.getModelCount().toInt();
      setState(() {
        _modelCount = count;
      });
    } catch (e) {
      debugPrint('[RENDERER] Error getting model count: $e');
    }
  }

  void _fitCameraToModel() {
    try {
      rust.fitCameraToAllModels();
      debugPrint('[RENDERER] Camera fitted to all model bounds');
    } catch (e) {
      debugPrint('[RENDERER ERROR] Failed to fit camera: $e');
    }
  }

  void _onTapUp(TapUpDetails details, BoxConstraints constraints) {
    if (!_modelLoaded) return;

    // Calculate normalized screen coordinates (0-1)
    final screenX = details.localPosition.dx / constraints.maxWidth;
    final screenY = details.localPosition.dy / constraints.maxHeight;

    debugPrint('[RENDERER] Tap at ($screenX, $screenY)');

    try {
      final element = rust.pickElement(screenX: screenX, screenY: screenY);
      if (element != null) {
        debugPrint('[RENDERER] Selected: ${element.elementType} - ${element.name}');
        setState(() {
          _selectedElement = element;
        });
        // Update highlight in renderer
        rust.setSelectedElement(elementId: element.id);
        rust.reloadAllModelsMesh();
      } else {
        debugPrint('[RENDERER] No element at tap position');
        _clearSelection();
      }
    } catch (e) {
      debugPrint('[RENDERER ERROR] Pick failed: $e');
    }
  }

  void _clearSelection() {
    setState(() {
      _selectedElement = null;
    });
    try {
      rust.setSelectedElement(elementId: null);
      rust.reloadAllModelsMesh();
    } catch (e) {
      debugPrint('[RENDERER ERROR] Clear selection failed: $e');
    }
  }

  void _showPropertiesPanel() {
    if (_selectedElement == null) return;
    showPropertiesPanel(
      context,
      element: _selectedElement!,
      onFocusElement: () {
        // Focus camera on selected element
        // TODO: Implement focus on specific element bounds
        _fitCameraToModel();
      },
    );
  }

  void _toggleVisibility(String elementType) {
    setState(() {
      _visibility[elementType] = !(_visibility[elementType] ?? true);
    });
    try {
      rust.setElementTypeVisible(
        elementType: elementType,
        visible: _visibility[elementType] ?? true,
      );
      rust.reloadAllModelsMesh();
    } catch (e) {
      debugPrint('[RENDERER ERROR] Toggle visibility failed: $e');
    }
  }

  void _updateLightDirection() {
    try {
      rust.setLightDirection(x: _lightX, y: _lightY, z: _lightZ);
    } catch (e) {
      debugPrint('[RENDERER ERROR] Set light direction failed: $e');
    }
  }

  void _updateLightColor() {
    try {
      rust.setLightColor(
        r: _lightColor.red / 255.0,
        g: _lightColor.green / 255.0,
        b: _lightColor.blue / 255.0,
      );
    } catch (e) {
      debugPrint('[RENDERER ERROR] Set light color failed: $e');
    }
  }

  void _updateLightIntensity() {
    try {
      rust.setLightIntensity(intensity: _lightIntensity);
    } catch (e) {
      debugPrint('[RENDERER ERROR] Set light intensity failed: $e');
    }
  }

  void _updateAmbientColor() {
    try {
      rust.setAmbientColor(
        r: _ambientColor.red / 255.0,
        g: _ambientColor.green / 255.0,
        b: _ambientColor.blue / 255.0,
      );
    } catch (e) {
      debugPrint('[RENDERER ERROR] Set ambient color failed: $e');
    }
  }

  void _toggleRenderMode() {
    setState(() {
      _renderMode = _renderMode == 0 ? 1 : 0;
    });
    try {
      rust.setRenderMode(mode: _renderMode);
    } catch (e) {
      debugPrint('[RENDERER ERROR] Set render mode failed: $e');
    }
  }

  void _toggleGridVisibility() {
    setState(() {
      _gridVisible = !_gridVisible;
    });
    // TODO: Enable when FRB bindings are regenerated
    // try {
    //   rust.setGridVisible(visible: _gridVisible);
    // } catch (e) {
    //   debugPrint('[RENDERER ERROR] Set grid visibility failed: $e');
    // }
  }

  void _showMeasurementTools() {
    showMeasurementTools(
      context,
      onMeasurementStarted: (type) {
        debugPrint('[MEASUREMENT] Started $type measurement');
        // Could show a snackbar or toast here
      },
      onMeasurementCleared: () {
        debugPrint('[MEASUREMENT] Cleared measurement');
      },
    );
  }

  void _showSectionPlaneTools() {
    showSectionPlaneTools(
      context,
      onPlaneChanged: () {
        debugPrint('[SECTION] Section plane changed');
        // Could trigger a render update here if needed
      },
    );
  }

  void _showDrawingOverlay() {
    showDrawingOverlayManager(context);
  }

  void _openSettings() {
    Navigator.push(
      context,
      MaterialPageRoute(builder: (context) => const SettingsScreen()),
    );
  }

  void _showLightingSettings() {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (context) => StatefulBuilder(
        builder: (context, setSheetState) {
          return DraggableScrollableSheet(
            initialChildSize: 0.6,
            minChildSize: 0.3,
            maxChildSize: 0.9,
            expand: false,
            builder: (context, scrollController) {
              return Container(
                padding: const EdgeInsets.all(16),
                child: ListView(
                  controller: scrollController,
                  children: [
                    // Header
                    Row(
                      children: [
                        Icon(Icons.light_mode, color: Theme.of(context).colorScheme.primary),
                        const SizedBox(width: 8),
                        Text(
                          'Lighting Settings',
                          style: Theme.of(context).textTheme.titleLarge?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 24),

                    // Light Direction
                    Text('Light Direction', style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 8),
                    _buildSlider(
                      context,
                      'X (Left/Right)',
                      _lightX,
                      -1.0,
                      1.0,
                      (v) {
                        setSheetState(() => _lightX = v);
                        setState(() => _lightX = v);
                        _updateLightDirection();
                      },
                    ),
                    _buildSlider(
                      context,
                      'Y (Down/Up)',
                      _lightY,
                      -1.0,
                      1.0,
                      (v) {
                        setSheetState(() => _lightY = v);
                        setState(() => _lightY = v);
                        _updateLightDirection();
                      },
                    ),
                    _buildSlider(
                      context,
                      'Z (Back/Front)',
                      _lightZ,
                      -1.0,
                      1.0,
                      (v) {
                        setSheetState(() => _lightZ = v);
                        setState(() => _lightZ = v);
                        _updateLightDirection();
                      },
                    ),
                    const SizedBox(height: 16),

                    // Light Intensity
                    Text('Light Intensity', style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 8),
                    _buildSlider(
                      context,
                      'Intensity',
                      _lightIntensity,
                      0.0,
                      3.0,
                      (v) {
                        setSheetState(() => _lightIntensity = v);
                        setState(() => _lightIntensity = v);
                        _updateLightIntensity();
                      },
                    ),
                    const SizedBox(height: 16),

                    // Light Color
                    Text('Light Color', style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 8),
                    _buildColorPicker(
                      context,
                      _lightColor,
                      (color) {
                        setSheetState(() => _lightColor = color);
                        setState(() => _lightColor = color);
                        _updateLightColor();
                      },
                    ),
                    const SizedBox(height: 16),

                    // Ambient Color
                    Text('Ambient Light', style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 8),
                    _buildColorPicker(
                      context,
                      _ambientColor,
                      (color) {
                        setSheetState(() => _ambientColor = color);
                        setState(() => _ambientColor = color);
                        _updateAmbientColor();
                      },
                    ),
                    const SizedBox(height: 24),

                    // Reset button
                    OutlinedButton.icon(
                      onPressed: () {
                        setSheetState(() {
                          _lightX = 0.5;
                          _lightY = 0.8;
                          _lightZ = 0.3;
                          _lightIntensity = 1.0;
                          _lightColor = const Color.fromRGBO(255, 250, 242, 1.0);
                          _ambientColor = const Color.fromRGBO(38, 43, 51, 1.0);
                        });
                        setState(() {
                          _lightX = 0.5;
                          _lightY = 0.8;
                          _lightZ = 0.3;
                          _lightIntensity = 1.0;
                          _lightColor = const Color.fromRGBO(255, 250, 242, 1.0);
                          _ambientColor = const Color.fromRGBO(38, 43, 51, 1.0);
                        });
                        _updateLightDirection();
                        _updateLightIntensity();
                        _updateLightColor();
                        _updateAmbientColor();
                      },
                      icon: const Icon(Icons.refresh),
                      label: const Text('Reset to Defaults'),
                    ),
                  ],
                ),
              );
            },
          );
        },
      ),
    );
  }

  Widget _buildSlider(
    BuildContext context,
    String label,
    double value,
    double min,
    double max,
    ValueChanged<double> onChanged,
  ) {
    return Row(
      children: [
        SizedBox(
          width: 100,
          child: Text(label, style: Theme.of(context).textTheme.bodySmall),
        ),
        Expanded(
          child: Slider(
            value: value,
            min: min,
            max: max,
            onChanged: onChanged,
          ),
        ),
        SizedBox(
          width: 50,
          child: Text(
            value.toStringAsFixed(2),
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ),
      ],
    );
  }

  Widget _buildColorPicker(
    BuildContext context,
    Color currentColor,
    ValueChanged<Color> onColorChanged,
  ) {
    final presetColors = [
      const Color(0xFFFFFFFF), // White
      const Color.fromRGBO(255, 250, 242, 1.0), // Warm white
      const Color(0xFFFFE4B5), // Moccasin (warmer)
      const Color(0xFFFFA500), // Orange
      const Color(0xFFADD8E6), // Light blue
      const Color(0xFF87CEEB), // Sky blue
      const Color(0xFF90EE90), // Light green
      const Color(0xFFFFB6C1), // Light pink
      const Color(0xFF1A1A24), // Dark (for ambient)
      const Color.fromRGBO(38, 43, 51, 1.0), // Blue-gray (default ambient)
    ];

    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: presetColors.map((color) {
        final isSelected = currentColor.value == color.value;
        return GestureDetector(
          onTap: () => onColorChanged(color),
          child: Container(
            width: 40,
            height: 40,
            decoration: BoxDecoration(
              color: color,
              shape: BoxShape.circle,
              border: Border.all(
                color: isSelected
                    ? Theme.of(context).colorScheme.primary
                    : Colors.grey.shade400,
                width: isSelected ? 3 : 1,
              ),
              boxShadow: [
                if (isSelected)
                  BoxShadow(
                    color: Theme.of(context).colorScheme.primary.withValues(alpha: 0.3),
                    blurRadius: 8,
                    spreadRadius: 2,
                  ),
              ],
            ),
          ),
        );
      }).toList(),
    );
  }

  void _startRenderLoop() {
    // Render at ~60 FPS for testing
    _renderTimer = Timer.periodic(const Duration(milliseconds: 16), (_) {
      _renderFrame();
    });
  }

  Future<void> _renderFrame() async {
    // Skip if already rendering (prevent frame overlap)
    if (_isRendering) return;
    if (!rust.isRendererInitialized()) return;

    _isRendering = true;
    try {
      // Get pixel data from Rust
      final Uint8List pixels = rust.renderFrame();

      // Convert to Flutter Image
      final image = await _createImageFromPixels(pixels, _width, _height);

      if (mounted) {
        setState(() {
          _frameImage = image;
        });
      }
    } catch (e) {
      // Silently ignore render errors to keep loop running
      debugPrint('Render error: $e');
    } finally {
      _isRendering = false;
    }
  }

  Future<ui.Image> _createImageFromPixels(Uint8List pixels, int width, int height) async {
    final completer = Completer<ui.Image>();

    ui.decodeImageFromPixels(
      pixels,
      width,
      height,
      ui.PixelFormat.rgba8888,
      (image) => completer.complete(image),
    );

    return completer.future;
  }

  void _onPanStart(DragStartDetails details) {
    _lastPanPosition = details.localPosition;
  }

  void _onPanUpdate(DragUpdateDetails details) {
    if (_lastPanPosition == null) return;

    final delta = details.localPosition - _lastPanPosition!;
    _lastPanPosition = details.localPosition;

    // Orbit camera based on drag
    rust.orbitCamera(
      deltaX: delta.dx * 0.01,
      deltaY: delta.dy * 0.01,
    );
  }

  void _onScaleStart(ScaleStartDetails details) {
    _lastScale = 1.0;
  }

  void _onScaleUpdate(ScaleUpdateDetails details) {
    if (details.pointerCount >= 2) {
      // Pinch zoom - scale > 1 means fingers spreading (zoom in)
      final scaleDelta = details.scale - _lastScale;
      _lastScale = details.scale;
      // Positive delta = zoom in (move camera closer)
      rust.zoomCamera(delta: scaleDelta * 80.0);
    } else {
      // Single finger pan for orbit - increased speed
      // Negate deltaX to fix left/right reversal
      final delta = details.focalPointDelta;
      rust.orbitCamera(
        deltaX: -delta.dx * 1.2,
        deltaY: delta.dy * 1.2,
      );
    }
  }

  void _onPointerSignal(PointerSignalEvent event) {
    if (event is PointerScrollEvent) {
      // Mouse wheel zoom
      rust.zoomCamera(delta: event.scrollDelta.dy * 0.01);
    }
  }

  void _onModelsChanged() {
    _refreshModelCount();
    // Reload all visible models into renderer
    if (_modelLoaded) {
      try {
        rust.loadAllModelsIntoRenderer();
      } catch (e) {
        debugPrint('[RENDERER] Error reloading models: $e');
      }
    }
  }

  void _onElementSelected(ElementInfo element) {
    setState(() {
      _selectedElement = element;
    });
    // Close the drawer
    Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      key: _scaffoldKey,
      drawer: ElementTreeDrawer(
        onElementSelected: _onElementSelected,
        selectedElementId: _selectedElement?.id,
      ),
      endDrawer: ModelManagerDrawer(
        onModelsChanged: _onModelsChanged,
      ),
      appBar: AppBar(
        title: Text(_modelLoaded
            ? '3D Viewer - $_modelCount Model${_modelCount == 1 ? '' : 's'}'
            : '3D Viewer'),
        actions: [
          // Element tree button
          if (_modelLoaded)
            IconButton(
              icon: const Icon(Icons.account_tree),
              onPressed: () {
                _scaffoldKey.currentState?.openDrawer();
              },
              tooltip: 'Element Tree',
            ),
          // Model manager button
          IconButton(
            icon: Badge(
              isLabelVisible: _modelCount > 0,
              label: Text('$_modelCount'),
              child: const Icon(Icons.layers),
            ),
            onPressed: () {
              _scaffoldKey.currentState?.openEndDrawer();
            },
            tooltip: 'Model Manager',
          ),
          if (!_modelLoaded && rust.isModelLoaded())
            IconButton(
              icon: const Icon(Icons.upload),
              onPressed: _loadModelIntoRenderer,
              tooltip: 'Load Models',
            ),
          if (_modelLoaded)
            PopupMenuButton<String>(
              icon: const Icon(Icons.visibility),
              tooltip: 'Element Visibility',
              onSelected: _toggleVisibility,
              itemBuilder: (context) => _visibility.entries.map((e) {
                return PopupMenuItem<String>(
                  value: e.key,
                  child: Row(
                    children: [
                      Icon(
                        e.value ? Icons.visibility : Icons.visibility_off,
                        color: e.value
                            ? Theme.of(context).colorScheme.primary
                            : Theme.of(context).colorScheme.outline,
                        size: 20,
                      ),
                      const SizedBox(width: 12),
                      Text(e.key),
                    ],
                  ),
                );
              }).toList(),
            ),
          if (_modelLoaded)
            IconButton(
              icon: const Icon(Icons.center_focus_strong),
              onPressed: _fitCameraToModel,
              tooltip: 'Fit to Model',
            ),
          if (_modelLoaded)
            IconButton(
              icon: Icon(
                Icons.grid_4x4,
                color: _gridVisible ? null : Theme.of(context).colorScheme.outline,
              ),
              onPressed: _toggleGridVisibility,
              tooltip: _gridVisible ? 'Hide Grid' : 'Show Grid',
            ),
          if (_wireframeSupported)
            IconButton(
              icon: Icon(_renderMode == 0 ? Icons.grid_3x3 : Icons.view_in_ar),
              onPressed: _toggleRenderMode,
              tooltip: _renderMode == 0 ? 'Switch to Wireframe' : 'Switch to Shaded',
            ),
          if (_modelLoaded)
            IconButton(
              icon: const Icon(Icons.straighten),
              onPressed: _showMeasurementTools,
              tooltip: 'Measurement Tools',
            ),
          if (_modelLoaded)
            IconButton(
              icon: const Icon(Icons.cut),
              onPressed: _showSectionPlaneTools,
              tooltip: 'Section Plane',
            ),
          if (_modelLoaded)
            IconButton(
              icon: const Icon(Icons.layers),
              onPressed: _showDrawingOverlay,
              tooltip: 'Drawing Overlay',
            ),
          IconButton(
            icon: const Icon(Icons.light_mode),
            onPressed: _showLightingSettings,
            tooltip: 'Lighting Settings',
          ),
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _initializeRenderer,
            tooltip: 'Restart Renderer',
          ),
          IconButton(
            icon: const Icon(Icons.settings),
            onPressed: _openSettings,
            tooltip: 'Settings',
          ),
        ],
      ),
      body: _buildBody(),
    );
  }

  Widget _buildBody() {
    if (_error != null) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.error_outline,
              size: 64,
              color: Theme.of(context).colorScheme.error,
            ),
            const SizedBox(height: 16),
            Text(
              'Renderer Error',
              style: Theme.of(context).textTheme.headlineSmall,
            ),
            const SizedBox(height: 8),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 32),
              child: SelectableText(
                _error!,
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodyMedium,
              ),
            ),
            const SizedBox(height: 24),
            ElevatedButton.icon(
              onPressed: _initializeRenderer,
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
            ),
          ],
        ),
      );
    }

    if (_isInitializing) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const CircularProgressIndicator(),
            const SizedBox(height: 16),
            Text(_status),
          ],
        ),
      );
    }

    return Column(
      children: [
        // Status bar
        Container(
          padding: const EdgeInsets.all(8),
          color: Theme.of(context).colorScheme.surfaceContainerHighest,
          child: Row(
            children: [
              Icon(
                Icons.check_circle,
                size: 16,
                color: Theme.of(context).colorScheme.primary,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  _status,
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
              Text(
                '${_width}x$_height',
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ),
        ),

        // 3D Viewport
        Expanded(
          child: LayoutBuilder(
            builder: (context, constraints) {
              return Listener(
                onPointerSignal: _onPointerSignal,
                child: GestureDetector(
                  onScaleStart: _onScaleStart,
                  onScaleUpdate: _onScaleUpdate,
                  onTapUp: (details) => _onTapUp(details, constraints),
                  child: Container(
                    color: const Color(0xFF1A1A24), // Dark background matching render clear color
                    child: Stack(
                      children: [
                        // Rendered image
                        Center(
                          child: _frameImage != null
                              ? RawImage(
                                  image: _frameImage,
                                  fit: BoxFit.contain,
                                )
                              : const Text(
                                  'Waiting for frame...',
                                  style: TextStyle(color: Colors.white54),
                                ),
                        ),
                        // Grid overlay
                        if (_modelLoaded)
                          GridOverlay(
                            visible: _gridVisible,
                            color: const Color(0x22FFFFFF),
                            spacing: 60.0,
                          ),
                        // Element info panel (bottom)
                        if (_selectedElement != null)
                          Positioned(
                            left: 16,
                            right: 16,
                            bottom: 16,
                            child: GestureDetector(
                              onTap: _showPropertiesPanel,
                              child: Container(
                                padding: const EdgeInsets.all(12),
                                decoration: BoxDecoration(
                                  color: Theme.of(context).colorScheme.surface.withValues(alpha: 0.95),
                                  borderRadius: BorderRadius.circular(12),
                                  boxShadow: [
                                    BoxShadow(
                                      color: Colors.black.withValues(alpha: 0.3),
                                      blurRadius: 8,
                                    ),
                                  ],
                                ),
                                child: Row(
                                  children: [
                                    Container(
                                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                                      decoration: BoxDecoration(
                                        color: Theme.of(context).colorScheme.primaryContainer,
                                        borderRadius: BorderRadius.circular(4),
                                      ),
                                      child: Text(
                                        _selectedElement!.elementType,
                                        style: TextStyle(
                                          fontWeight: FontWeight.bold,
                                          color: Theme.of(context).colorScheme.onPrimaryContainer,
                                        ),
                                      ),
                                    ),
                                    const SizedBox(width: 12),
                                    Expanded(
                                      child: Column(
                                        crossAxisAlignment: CrossAxisAlignment.start,
                                        mainAxisSize: MainAxisSize.min,
                                        children: [
                                          Text(
                                            _selectedElement!.name.isEmpty
                                                ? 'Unnamed Element'
                                                : _selectedElement!.name,
                                            style: Theme.of(context).textTheme.titleSmall,
                                          ),
                                          Text(
                                            'Tap for details',
                                            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                              color: Theme.of(context).colorScheme.primary,
                                            ),
                                          ),
                                        ],
                                      ),
                                    ),
                                    IconButton(
                                      icon: const Icon(Icons.info_outline, size: 20),
                                      onPressed: _showPropertiesPanel,
                                      tooltip: 'View properties',
                                    ),
                                    IconButton(
                                      icon: const Icon(Icons.close, size: 20),
                                      onPressed: _clearSelection,
                                      tooltip: 'Clear selection',
                                    ),
                                  ],
                                ),
                              ),
                            ),
                          ),
                      ],
                    ),
                  ),
                ),
              );
            },
          ),
        ),

        // Controls hint
        Container(
          padding: const EdgeInsets.all(12),
          color: Theme.of(context).colorScheme.surfaceContainerHighest,
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              _buildControlHint(Icons.touch_app, 'Tap to select'),
              const SizedBox(width: 16),
              _buildControlHint(Icons.swipe, 'Drag to orbit'),
              const SizedBox(width: 16),
              _buildControlHint(Icons.pinch, 'Pinch to zoom'),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildControlHint(IconData icon, String label) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 16, color: Theme.of(context).colorScheme.onSurfaceVariant),
        const SizedBox(width: 4),
        Text(
          label,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
      ],
    );
  }
}
