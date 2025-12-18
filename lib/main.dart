import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:file_picker/file_picker.dart';
import 'package:path_provider/path_provider.dart';
import 'core/bridge/api.dart' as rust;
import 'core/bridge/bim/model.dart';
import 'core/bridge/frb_generated.dart';

void main() {
  runApp(const BimViewerApp());
}

class BimViewerApp extends StatelessWidget {
  const BimViewerApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'BIM Viewer',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: Colors.blue,
          brightness: Brightness.light,
        ),
        useMaterial3: true,
      ),
      darkTheme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: Colors.blue,
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
      ),
      themeMode: ThemeMode.system,
      home: const HomePage(),
    );
  }
}

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  String _status = 'Waiting to initialize...';
  String _version = '';
  String _systemInfo = '';
  bool _isLoading = false;
  ModelInfo? _modelInfo;
  bool _modelLoaded = false;

  @override
  void initState() {
    super.initState();
    _initialize();
  }

  Future<void> _initialize() async {
    // Initialize Rust bridge
    await RustLib.init();

    setState(() {
      _isLoading = true;
      _status = 'Initializing Rust engine...';
    });

    try {
      final initMessage = rust.initialize();
      final version = rust.getVersion();
      final systemInfo = rust.getSystemInfo();

      setState(() {
        _status = initMessage;
        _version = 'v$version';
        _systemInfo = systemInfo;
        _isLoading = false;
      });
    } catch (e) {
      setState(() {
        _status = 'Error: $e';
        _isLoading = false;
      });
    }
  }

  Future<void> _testAsync() async {
    setState(() {
      _isLoading = true;
      _status = 'Testing async...';
    });

    try {
      final result = await rust.testAsync();
      setState(() {
        _status = result;
        _isLoading = false;
      });
    } catch (e) {
      setState(() {
        _status = 'Error: $e';
        _isLoading = false;
      });
    }
  }

  Future<void> _testErrorHandling() async {
    try {
      final result = rust.testErrorHandling(shouldFail: true);
      setState(() => _status = result);
    } catch (e) {
      setState(() => _status = 'Caught error: $e');
    }
  }

  Future<void> _testRenderer() async {
    setState(() {
      _isLoading = true;
      _status = 'Testing wgpu renderer...';
    });

    try {
      final result = await rust.testRendererInit();
      setState(() {
        _status = result;
        _isLoading = false;
      });
    } catch (e) {
      setState(() {
        _status = 'Renderer error: $e';
        _isLoading = false;
      });
    }
  }

  Future<void> _loadSampleIfc() async {
    setState(() {
      _isLoading = true;
      _status = 'Loading sample IFC file...';
    });

    try {
      // Load the sample IFC file from assets
      final content = await rootBundle.loadString('test/sample_building.ifc');

      setState(() {
        _status = 'Parsing IFC file...';
      });

      // Parse the IFC content
      final modelInfo = await rust.parseIfcContent(content: content);

      setState(() {
        _modelInfo = modelInfo;
        _modelLoaded = true;
        _status = 'IFC file loaded successfully!';
        _isLoading = false;
      });
    } catch (e) {
      setState(() {
        _status = 'Error loading IFC: $e';
        _isLoading = false;
        _modelLoaded = false;
      });
    }
  }

  Future<void> _pickAndLoadIfc() async {
    setState(() {
      _isLoading = true;
      _status = 'Picking IFC file...';
    });

    try {
      // Pick IFC file
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['ifc'],
      );

      if (result == null || result.files.isEmpty) {
        setState(() {
          _status = 'No file selected';
          _isLoading = false;
        });
        return;
      }

      final filePath = result.files.first.path;
      if (filePath == null) {
        setState(() {
          _status = 'Invalid file path';
          _isLoading = false;
        });
        return;
      }

      setState(() {
        _status = 'Loading IFC file...';
      });

      // Load the IFC file
      final modelInfo = await rust.loadIfcFile(filePath: filePath);

      setState(() {
        _modelInfo = modelInfo;
        _modelLoaded = true;
        _status = 'IFC file loaded successfully!';
        _isLoading = false;
      });
    } catch (e) {
      setState(() {
        _status = 'Error loading IFC: $e';
        _isLoading = false;
        _modelLoaded = false;
      });
    }
  }

  Future<void> _unloadModel() async {
    try {
      rust.unloadModel();
      setState(() {
        _modelInfo = null;
        _modelLoaded = false;
        _status = 'Model unloaded';
      });
    } catch (e) {
      setState(() {
        _status = 'Error unloading model: $e';
      });
    }
  }

  Widget _buildInfoRow(BuildContext context, String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4.0),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(
            label,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
          ),
          Text(
            value,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  fontWeight: FontWeight.w500,
                ),
          ),
        ],
      ),
    );
  }

  Widget _buildStatRow(BuildContext context, String label, int value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4.0),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: Theme.of(context).textTheme.bodyMedium),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.primaryContainer,
              borderRadius: BorderRadius.circular(12),
            ),
            child: Text(
              value.toString(),
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                    color: Theme.of(context).colorScheme.onPrimaryContainer,
                  ),
            ),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(_modelLoaded ? 'BIM Viewer - Model Loaded' : 'BIM Viewer'),
        elevation: 2,
      ),
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24.0),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            crossAxisAlignment: CrossAxisAlignment.center,
            children: [
              // Logo/Icon placeholder
              Container(
                width: 120,
                height: 120,
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(20),
                ),
                child: Icon(
                  Icons.architecture,
                  size: 64,
                  color: Theme.of(context).colorScheme.primary,
                ),
              ),

              const SizedBox(height: 32),

              // Title
              Text(
                'BIM Viewer',
                style: Theme.of(context).textTheme.headlineLarge?.copyWith(
                      fontWeight: FontWeight.bold,
                    ),
              ),

              const SizedBox(height: 8),

              // Version
              if (_version.isNotEmpty)
                Text(
                  _version,
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        color: Theme.of(context).colorScheme.secondary,
                      ),
                ),

              const SizedBox(height: 32),

              // Status Card
              Card(
                elevation: 4,
                child: Padding(
                  padding: const EdgeInsets.all(20.0),
                  child: Column(
                    children: [
                      if (_isLoading)
                        const CircularProgressIndicator()
                      else
                        Icon(
                          Icons.check_circle_outline,
                          size: 48,
                          color: Theme.of(context).colorScheme.primary,
                        ),
                      const SizedBox(height: 16),
                      SelectableText(
                        _status,
                        textAlign: TextAlign.center,
                        style: Theme.of(context).textTheme.bodyLarge,
                      ),
                    ],
                  ),
                ),
              ),

              const SizedBox(height: 24),

              // System Info
              if (_systemInfo.isNotEmpty)
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16.0),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'System Information',
                          style: Theme.of(context).textTheme.titleMedium?.copyWith(
                                fontWeight: FontWeight.bold,
                              ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          _systemInfo,
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                fontFamily: 'monospace',
                              ),
                        ),
                      ],
                    ),
                  ),
                ),

              const SizedBox(height: 32),

              // Model Info Card (Phase 2)
              if (_modelLoaded && _modelInfo != null)
                Card(
                  elevation: 4,
                  child: Padding(
                    padding: const EdgeInsets.all(20.0),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Icon(
                              Icons.account_balance,
                              color: Theme.of(context).colorScheme.primary,
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                'Model Information',
                                style: Theme.of(context).textTheme.titleLarge?.copyWith(
                                      fontWeight: FontWeight.bold,
                                    ),
                              ),
                            ),
                          ],
                        ),
                        const Divider(height: 24),
                        _buildInfoRow(context, 'Project', _modelInfo!.projectName),
                        _buildInfoRow(context, 'Building', _modelInfo!.buildingName),
                        _buildInfoRow(context, 'Site', _modelInfo!.siteName),
                        const Divider(height: 24),
                        Text(
                          'Element Statistics',
                          style: Theme.of(context).textTheme.titleMedium?.copyWith(
                                fontWeight: FontWeight.bold,
                              ),
                        ),
                        const SizedBox(height: 12),
                        _buildStatRow(context, 'Total Elements', _modelInfo!.stats.totalEntities.toInt()),
                        _buildStatRow(context, 'Walls', _modelInfo!.stats.walls.toInt()),
                        _buildStatRow(context, 'Slabs', _modelInfo!.stats.slabs.toInt()),
                        _buildStatRow(context, 'Columns', _modelInfo!.stats.columns.toInt()),
                        _buildStatRow(context, 'Beams', _modelInfo!.stats.beams.toInt()),
                        _buildStatRow(context, 'Doors', _modelInfo!.stats.doors.toInt()),
                        _buildStatRow(context, 'Windows', _modelInfo!.stats.windows.toInt()),
                        _buildStatRow(context, 'Storeys', _modelInfo!.stats.storeys.toInt()),
                      ],
                    ),
                  ),
                ),

              if (_modelLoaded) const SizedBox(height: 24),

              // Action Buttons
              Wrap(
                spacing: 12,
                runSpacing: 12,
                alignment: WrapAlignment.center,
                children: [
                  // Phase 2: IFC Loading Buttons
                  if (!_modelLoaded) ...[
                    FilledButton.icon(
                      onPressed: _isLoading ? null : _loadSampleIfc,
                      icon: const Icon(Icons.file_open),
                      label: const Text('Load Sample'),
                    ),
                    ElevatedButton.icon(
                      onPressed: _isLoading ? null : _pickAndLoadIfc,
                      icon: const Icon(Icons.folder_open),
                      label: const Text('Pick IFC File'),
                    ),
                  ],
                  if (_modelLoaded)
                    ElevatedButton.icon(
                      onPressed: _isLoading ? null : _unloadModel,
                      icon: const Icon(Icons.close),
                      label: const Text('Unload Model'),
                      style: ElevatedButton.styleFrom(
                        backgroundColor: Theme.of(context).colorScheme.errorContainer,
                        foregroundColor: Theme.of(context).colorScheme.onErrorContainer,
                      ),
                    ),

                  // Phase 1: Test Buttons
                  ElevatedButton.icon(
                    onPressed: _isLoading ? null : _testAsync,
                    icon: const Icon(Icons.play_arrow),
                    label: const Text('Test Async'),
                  ),
                  ElevatedButton.icon(
                    onPressed: _isLoading ? null : _testErrorHandling,
                    icon: const Icon(Icons.error_outline),
                    label: const Text('Test Error'),
                  ),

                  // Phase 3: Renderer Test
                  ElevatedButton.icon(
                    onPressed: _isLoading ? null : _testRenderer,
                    icon: const Icon(Icons.threed_rotation),
                    label: const Text('Test Renderer'),
                  ),
                ],
              ),

              const SizedBox(height: 24),

              // Info Banner
              Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Column(
                  children: [
                    const Icon(Icons.info_outline, size: 32),
                    const SizedBox(height: 8),
                    Text(
                      _modelLoaded
                          ? 'Phase 2: IFC Parsing - Working!'
                          : 'Phase 2: IFC Parser Ready',
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      _modelLoaded
                          ? 'IFC file loaded and parsed successfully!\n'
                              'Element counts and properties extracted.'
                          : 'Load a sample IFC file or pick your own .ifc file\n'
                              'to test the custom Rust IFC parser.',
                      textAlign: TextAlign.center,
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
