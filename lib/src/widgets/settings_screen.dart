import 'dart:io';
import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import '../core/bridge/api.dart' as rust;

/// Settings and tools screen
class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key});

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  rust.RenderStats? _stats;
  bool _loadingStats = false;
  bool _exportingScreenshot = false;

  @override
  void initState() {
    super.initState();
    _loadStats();
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

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

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
}
