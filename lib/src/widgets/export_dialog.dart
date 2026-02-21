import 'dart:io';
import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import '../core/bridge/api.dart' as rust;
import '../core/bridge/api/export.dart' as export_api;

/// Export dialog — all export formats in one place.
class ExportDialog extends StatefulWidget {
  const ExportDialog({super.key});

  @override
  State<ExportDialog> createState() => _ExportDialogState();
}

class _ExportDialogState extends State<ExportDialog> {
  bool _exporting = false;
  String? _exportingLabel;

  Future<String> _getExportDir() async {
    final dir = await getApplicationDocumentsDirectory();
    return dir.path;
  }

  String _timestamp() =>
      DateTime.now().toIso8601String().replaceAll(':', '-').split('.').first;

  Future<void> _exportScreenshot() async {
    await _doExport('Screenshot', () async {
      final dir = await _getExportDir();
      final path = '$dir/bim_screenshot_${_timestamp()}.png';
      await rust.exportScreenshot(path: path);
      return 'Saved to $path';
    });
  }

  Future<void> _exportGltf() async {
    await _doExport('glTF', () async {
      final json = export_api.exportGltfJson();
      final dir = await _getExportDir();
      final path = '$dir/bim_export_${_timestamp()}.gltf';
      await File(path).writeAsString(json);
      return 'Saved to $path (${(json.length / 1024).round()} KB)';
    });
  }

  Future<void> _exportHtmlReport() async {
    await _doExport('HTML Report', () async {
      final html = export_api.exportHtmlReport();
      final dir = await _getExportDir();
      final path = '$dir/bim_report_${_timestamp()}.html';
      await File(path).writeAsString(html);
      return 'Saved to $path';
    });
  }

  Future<void> _exportPropertiesCsv() async {
    await _doExport('Properties CSV', () async {
      final csv = export_api.exportPropertiesCsv();
      final dir = await _getExportDir();
      final path = '$dir/bim_properties_${_timestamp()}.csv';
      await File(path).writeAsString(csv);
      return 'Saved to $path';
    });
  }

  Future<void> _exportQuantityCsv() async {
    await _doExport('Quantity CSV', () async {
      final csv = export_api.exportQuantityReportCsv();
      final dir = await _getExportDir();
      final path = '$dir/bim_quantities_${_timestamp()}.csv';
      await File(path).writeAsString(csv);
      return 'Saved to $path';
    });
  }

  Future<void> _exportElementSummaryCsv() async {
    await _doExport('Element Summary CSV', () async {
      final csv = export_api.exportElementSummaryCsv();
      final dir = await _getExportDir();
      final path = '$dir/bim_elements_${_timestamp()}.csv';
      await File(path).writeAsString(csv);
      return 'Saved to $path';
    });
  }

  Future<void> _exportModelInfoJson() async {
    await _doExport('Model Info JSON', () async {
      final json = export_api.exportModelInfoJson();
      final dir = await _getExportDir();
      final path = '$dir/bim_model_info_${_timestamp()}.json';
      await File(path).writeAsString(json);
      return 'Saved to $path';
    });
  }

  Future<void> _shareViewpoint() async {
    await _doExport('Viewpoint', () async {
      final json = export_api.getViewpointShareData();
      return 'Viewpoint data copied (${json.length} chars)';
    });
  }

  Future<void> _doExport(
      String label, Future<String> Function() action) async {
    setState(() {
      _exporting = true;
      _exportingLabel = label;
    });
    try {
      final message = await action();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(message), duration: const Duration(seconds: 4)),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Export failed: $e'),
            backgroundColor: Theme.of(context).colorScheme.error,
          ),
        );
      }
    } finally {
      if (mounted) setState(() => _exporting = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('Export',
                  style: theme.textTheme.titleLarge
                      ?.copyWith(fontWeight: FontWeight.bold)),
              IconButton(
                icon: const Icon(Icons.close),
                onPressed: () => Navigator.pop(context),
              ),
            ],
          ),
          if (_exporting)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 16),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  const SizedBox(
                      width: 20,
                      height: 20,
                      child: CircularProgressIndicator(strokeWidth: 2)),
                  const SizedBox(width: 12),
                  Text('Exporting $_exportingLabel...'),
                ],
              ),
            ),
          const SizedBox(height: 8),
          _exportTile(
            Icons.camera_alt,
            'Screenshot (PNG)',
            'Save current view as image',
            _exportScreenshot,
          ),
          _exportTile(
            Icons.view_in_ar,
            'glTF 3D Model',
            'Export as glTF 2.0 (opens in any 3D viewer)',
            _exportGltf,
          ),
          const Divider(height: 16),
          _exportTile(
            Icons.description,
            'HTML Report',
            'Professional model report with tables and quantities',
            _exportHtmlReport,
          ),
          _exportTile(
            Icons.table_chart,
            'Properties CSV',
            'All element properties (ID, name, type, property sets)',
            _exportPropertiesCsv,
          ),
          _exportTile(
            Icons.inventory_2,
            'Quantity Report CSV',
            'Area, volume, length, weight per element type',
            _exportQuantityCsv,
          ),
          _exportTile(
            Icons.list_alt,
            'Element Summary CSV',
            'One row per element with bounds and basic info',
            _exportElementSummaryCsv,
          ),
          _exportTile(
            Icons.data_object,
            'Model Info JSON',
            'Comprehensive model metadata',
            _exportModelInfoJson,
          ),
          const Divider(height: 16),
          _exportTile(
            Icons.share,
            'Share Viewpoint',
            'Get camera + selection state as shareable data',
            _shareViewpoint,
          ),
        ],
      ),
    );
  }

  Widget _exportTile(
      IconData icon, String title, String subtitle, VoidCallback onTap) {
    return ListTile(
      leading: Icon(icon),
      title: Text(title),
      subtitle: Text(subtitle, style: Theme.of(context).textTheme.bodySmall),
      trailing: const Icon(Icons.arrow_forward_ios, size: 14),
      onTap: _exporting ? null : onTap,
      dense: true,
    );
  }
}

/// Show export dialog as a bottom sheet.
void showExportDialog(BuildContext context) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (_) => const ExportDialog(),
  );
}
