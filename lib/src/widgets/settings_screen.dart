import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import '../core/bridge/api.dart' as rust;
import '../core/bridge/api/properties.dart' as properties;
import '../core/bridge/api/section.dart' as section;
import '../core/bridge/api/system.dart' as system;
import '../core/providers/accessibility_state.dart';

/// Settings and tools screen
class SettingsScreen extends ConsumerStatefulWidget {
  const SettingsScreen({super.key});

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen> {
  rust.RenderStats? _stats;
  bool _loadingStats = false;
  bool _exportingScreenshot = false;
  int _unitSystem = 0; // 0=SI, 1=Imperial
  bool _sectionBoxActive = false;

  @override
  void initState() {
    super.initState();
    _loadStats();
    _loadSettings();
  }

  void _loadSettings() {
    try {
      _unitSystem = properties.getUnitSystem();
      _sectionBoxActive = section.isSectionBoxActive();
    } catch (e) {
      debugPrint('[SETTINGS] loadSettings: $e');
    }
  }

  Future<void> _loadStats() async {
    setState(() => _loadingStats = true);
    try {
      final stats = rust.getRenderStats();
      setState(() {
        _stats = stats;
        _loadingStats = false;
      });
    } catch (e) {
      debugPrint('[SETTINGS] Failed to load stats: $e');
      setState(() => _loadingStats = false);
    }
  }

  Future<void> _exportScreenshot() async {
    setState(() => _exportingScreenshot = true);

    try {
      // Get documents directory
      final directory = await getApplicationDocumentsDirectory();
      final timestamp = DateTime.now().toIso8601String().replaceAll(':', '-');
      final path = '${directory.path}/bim_screenshot_$timestamp.png';

      // Export screenshot
      await rust.exportScreenshot(path: path);

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Screenshot saved to:\n$path'),
            duration: const Duration(seconds: 4),
            action: SnackBarAction(
              label: 'OK',
              onPressed: () {},
            ),
          ),
        );
      }
    } catch (e) {
      debugPrint('[SETTINGS] Screenshot export failed: $e');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Failed to export screenshot: $e'),
            backgroundColor: Theme.of(context).colorScheme.error,
          ),
        );
      }
    } finally {
      setState(() => _exportingScreenshot = false);
    }
  }

  Future<void> _colorByType() async {
    try {
      rust.colorByType();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Elements colored by type')),
        );
      }
    } catch (e) {
      debugPrint('[SETTINGS] Color by type failed: $e');
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Failed to color by type: $e'),
            backgroundColor: Theme.of(context).colorScheme.error,
          ),
        );
      }
    }
  }

  Future<void> _resetColors() async {
    try {
      rust.resetElementColors();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Colors reset to defaults')),
        );
      }
    } catch (e) {
      debugPrint('[SETTINGS] Reset colors failed: $e');
    }
  }

  void _setUnitSystem(int system) {
    try {
      properties.setUnitSystem(system: system);
      setState(() => _unitSystem = system);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
              content: Text(system == 0 ? 'Switched to SI (metric)' : 'Switched to Imperial')),
        );
      }
    } catch (e) {
      debugPrint('[SETTINGS] setUnitSystem failed: $e');
    }
  }

  void _toggleSectionBox() {
    try {
      if (_sectionBoxActive) {
        section.clearSectionBox();
      } else {
        section.setSectionBoxFromModel(padding: 0.1);
      }
      setState(() => _sectionBoxActive = !_sectionBoxActive);
    } catch (e) {
      debugPrint('[SETTINGS] sectionBox failed: $e');
    }
  }

  void _compactMemory() {
    try {
      system.compactMemory();
      if (mounted) {
        final summary = system.getMemorySummary();
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Memory compacted. $summary')),
        );
      }
    } catch (e) {
      debugPrint('[SETTINGS] compactMemory failed: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Settings & Tools'),
      ),
      body: ListView(
        children: [
          // Render Statistics Section
          _buildSection(
            title: 'Render Statistics',
            icon: Icons.analytics,
            children: [
              if (_loadingStats)
                const Center(
                  child: Padding(
                    padding: EdgeInsets.all(16.0),
                    child: CircularProgressIndicator(),
                  ),
                )
              else if (_stats != null) ...[
                _buildStatRow('FPS', '${_stats!.fps.toStringAsFixed(1)}'),
                _buildStatRow('Frame Time', '${_stats!.frameTimeMs.toStringAsFixed(2)} ms'),
                _buildStatRow('Triangles', _stats!.triangleCount.toString()),
                _buildStatRow('Elements', _stats!.elementCount.toString()),
                const SizedBox(height: 8),
                Center(
                  child: TextButton.icon(
                    onPressed: _loadStats,
                    icon: const Icon(Icons.refresh),
                    label: const Text('Refresh Stats'),
                  ),
                ),
              ] else
                Padding(
                  padding: const EdgeInsets.all(16.0),
                  child: Center(
                    child: TextButton.icon(
                      onPressed: _loadStats,
                      icon: const Icon(Icons.refresh),
                      label: const Text('Load Stats'),
                    ),
                  ),
                ),
            ],
          ),

          const Divider(),

          // Units Section
          _buildSection(
            title: 'Units',
            icon: Icons.straighten,
            children: [
              RadioListTile<int>(
                title: const Text('SI (Metric)'),
                subtitle: Text(
                    'm, m\u{b2}, m\u{b3}',
                    style: theme.textTheme.bodySmall),
                value: 0,
                groupValue: _unitSystem,
                onChanged: (v) => _setUnitSystem(v!),
              ),
              RadioListTile<int>(
                title: const Text('Imperial'),
                subtitle: Text(
                    'ft, ft\u{b2}, ft\u{b3}',
                    style: theme.textTheme.bodySmall),
                value: 1,
                groupValue: _unitSystem,
                onChanged: (v) => _setUnitSystem(v!),
              ),
            ],
          ),

          const Divider(),

          // Section Box
          _buildSection(
            title: 'Section Box',
            icon: Icons.crop,
            children: [
              SwitchListTile(
                secondary: const Icon(Icons.crop),
                title: const Text('Section Box'),
                subtitle: const Text('Clip model to bounding box (6 planes)'),
                value: _sectionBoxActive,
                onChanged: (_) => _toggleSectionBox(),
              ),
            ],
          ),

          const Divider(),

          // Export Section
          _buildSection(
            title: 'Export',
            icon: Icons.save_alt,
            children: [
              ListTile(
                leading: const Icon(Icons.camera_alt),
                title: const Text('Export Screenshot'),
                subtitle: const Text('Save current view as PNG'),
                trailing: _exportingScreenshot
                    ? const SizedBox(
                        width: 24,
                        height: 24,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.arrow_forward_ios, size: 16),
                onTap: _exportingScreenshot ? null : _exportScreenshot,
              ),
            ],
          ),

          const Divider(),

          // Color Coding Section
          _buildSection(
            title: 'Color Coding',
            icon: Icons.palette,
            children: [
              ListTile(
                leading: const Icon(Icons.category),
                title: const Text('Color by Type'),
                subtitle: const Text('Walls, beams, columns, etc.'),
                trailing: const Icon(Icons.arrow_forward_ios, size: 16),
                onTap: _colorByType,
              ),
              ListTile(
                leading: const Icon(Icons.refresh),
                title: const Text('Reset Colors'),
                subtitle: const Text('Restore default colors'),
                trailing: const Icon(Icons.arrow_forward_ios, size: 16),
                onTap: _resetColors,
              ),
            ],
          ),

          const Divider(),

          // Memory Section
          _buildSection(
            title: 'Memory',
            icon: Icons.memory,
            children: [
              ListTile(
                leading: const Icon(Icons.cleaning_services),
                title: const Text('Compact Memory'),
                subtitle: const Text('Free unused memory'),
                trailing: const Icon(Icons.arrow_forward_ios, size: 16),
                onTap: _compactMemory,
              ),
            ],
          ),

          const Divider(),

          // Accessibility Section
          _buildAccessibilitySection(theme),

          const Divider(),

          // About Section
          _buildSection(
            title: 'About',
            icon: Icons.info_outline,
            children: [
              ListTile(
                leading: const Icon(Icons.apps),
                title: const Text('App Version'),
                subtitle: Text('v${rust.getVersion()}'),
              ),
              ListTile(
                leading: const Icon(Icons.phone_android),
                title: const Text('System Info'),
                subtitle: Text(
                  rust.getSystemInfo(),
                  style: theme.textTheme.bodySmall,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildSection({
    required String title,
    required IconData icon,
    required List<Widget> children,
  }) {
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
          child: Row(
            children: [
              Icon(icon, size: 20, color: theme.colorScheme.primary),
              const SizedBox(width: 8),
              Text(
                title,
                style: theme.textTheme.titleMedium?.copyWith(
                  fontWeight: FontWeight.bold,
                  color: theme.colorScheme.primary,
                ),
              ),
            ],
          ),
        ),
        ...children,
      ],
    );
  }

  Widget _buildStatRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label),
          Text(
            value,
            style: const TextStyle(fontWeight: FontWeight.bold),
          ),
        ],
      ),
    );
  }

  Widget _buildAccessibilitySection(ThemeData theme) {
    final accessibilityState = ref.watch(accessibilityStateProvider);
    final accessibilityNotifier = ref.read(accessibilityStateProvider.notifier);

    return _buildSection(
      title: 'Accessibility',
      icon: Icons.accessibility_new,
      children: [
        SwitchListTile(
          secondary: const Icon(Icons.text_fields),
          title: const Text('Use System Text Size'),
          subtitle: const Text('Follow the device text size setting'),
          value: accessibilityState.useSystemTextScale,
          onChanged: (value) {
            accessibilityNotifier.setUseSystemTextScale(value);
          },
        ),
        if (!accessibilityState.useSystemTextScale) ...[
          ListTile(
            leading: const Icon(Icons.format_size),
            title: const Text('Custom Text Size'),
            subtitle: Slider(
              min: 0.8,
              max: 2.0,
              divisions: 12,
              value: accessibilityState.customTextScaleFactor,
              onChanged: (value) {
                accessibilityNotifier.setCustomTextScaleFactor(value);
              },
              label:
                  '${(accessibilityState.customTextScaleFactor * 100).round()}%',
            ),
            trailing: Text(
              '${(accessibilityState.customTextScaleFactor * 100).round()}%',
              style: theme.textTheme.bodyMedium?.copyWith(
                fontWeight: FontWeight.bold,
              ),
            ),
          ),
        ],
      ],
    );
  }
}
