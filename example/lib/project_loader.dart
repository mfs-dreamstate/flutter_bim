import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_bim/src/core/bridge/api.dart' as rust;

/// A discipline layer that can be loaded from bundled assets.
class _Layer {
  final String discipline;
  final String label;
  final IconData icon;
  final Color color;
  final String assetPath;
  bool selected;

  _Layer({
    required this.discipline,
    required this.label,
    required this.icon,
    required this.color,
    required this.assetPath,
    // ignore: unused_element_parameter
    this.selected = true,
  });
}

/// The available NordicLCA building variants.
enum _BuildingVariant {
  housingConcrete('Housing Concrete'),
  housingTimber('Housing Timber'),
  officeConcrete('Office Concrete'),
  officeTimber('Office Timber');

  final String label;
  const _BuildingVariant(this.label);
}

/// Returns the bundled asset layers for a given building variant.
List<_Layer> _layersFor(_BuildingVariant variant) {
  final suffix = switch (variant) {
    _BuildingVariant.housingConcrete => 'Housing_Concrete',
    _BuildingVariant.housingTimber => 'Housing_Timber',
    _BuildingVariant.officeConcrete => 'Office_Concrete',
    _BuildingVariant.officeTimber => 'Office_Timber',
  };

  return [
    _Layer(
      discipline: 'STRUC',
      label: 'Structural',
      icon: Icons.foundation,
      color: Colors.blueGrey,
      assetPath: 'assets/nordic/STRUC_NordicLCA_${suffix}_BuildingPermit.ifc',
    ),
    _Layer(
      discipline: 'ELE',
      label: 'Electrical',
      icon: Icons.electrical_services,
      color: Colors.orange,
      assetPath: 'assets/nordic/ELE_NordicLCA_${suffix}_BuildingPermit.ifc',
    ),
  ];
}

/// Shows the NordicLCA project loader dialog.
///
/// Returns `true` if models were loaded successfully.
Future<bool> showProjectLoader(BuildContext context) async {
  final result = await showDialog<bool>(
    context: context,
    barrierDismissible: false,
    builder: (context) => const _ProjectLoaderDialog(),
  );
  return result ?? false;
}

class _ProjectLoaderDialog extends StatefulWidget {
  const _ProjectLoaderDialog();

  @override
  State<_ProjectLoaderDialog> createState() => _ProjectLoaderDialogState();
}

class _ProjectLoaderDialogState extends State<_ProjectLoaderDialog> {
  _BuildingVariant _variant = _BuildingVariant.housingConcrete;
  late List<_Layer> _layers = _layersFor(_variant);
  bool _isLoading = false;
  int _loadProgress = 0;
  String _loadingLabel = '';
  String? _error;

  void _onVariantChanged(_BuildingVariant variant) {
    setState(() {
      _variant = variant;
      _layers = _layersFor(variant);
    });
  }

  int get _selectedCount => _layers.where((l) => l.selected).length;

  Future<void> _loadSelected() async {
    final selected = _layers.where((l) => l.selected).toList();
    if (selected.isEmpty) return;

    setState(() {
      _isLoading = true;
      _loadProgress = 0;
      _error = null;
    });

    try {
      for (final layer in selected) {
        setState(() {
          _loadingLabel = layer.label;
          _loadProgress++;
        });

        debugPrint('[PROJECT_LOADER] Loading ${layer.assetPath}...');
        final content = await rootBundle.loadString(layer.assetPath);
        debugPrint('[PROJECT_LOADER] Asset loaded, ${content.length} chars');

        await rust.parseIfcContent(content: content);
        debugPrint('[PROJECT_LOADER] Parsed ${layer.discipline}');
      }

      if (mounted) Navigator.of(context).pop(true);
    } catch (e) {
      debugPrint('[PROJECT_LOADER] Error: $e');
      setState(() {
        _error = 'Failed to load $_loadingLabel: $e';
        _isLoading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Dialog(
      insetPadding: const EdgeInsets.all(24),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 420),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            // Header
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(20),
              decoration: BoxDecoration(
                color: colorScheme.primaryContainer,
                borderRadius:
                    const BorderRadius.vertical(top: Radius.circular(28)),
              ),
              child: Row(
                children: [
                  Icon(Icons.apartment, color: colorScheme.onPrimaryContainer),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'NordicLCA Project',
                          style: Theme.of(context)
                              .textTheme
                              .titleLarge
                              ?.copyWith(
                                color: colorScheme.onPrimaryContainer,
                                fontWeight: FontWeight.bold,
                              ),
                        ),
                        Text(
                          'Select building and discipline layers',
                          style:
                              Theme.of(context).textTheme.bodySmall?.copyWith(
                                    color: colorScheme.onPrimaryContainer
                                        .withValues(alpha: 0.8),
                                  ),
                        ),
                      ],
                    ),
                  ),
                  IconButton(
                    icon: Icon(Icons.close,
                        color: colorScheme.onPrimaryContainer),
                    onPressed:
                        _isLoading ? null : () => Navigator.pop(context),
                  ),
                ],
              ),
            ),

            // Body
            Padding(
              padding: const EdgeInsets.all(20),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // Building variant selector
                  Text(
                    'Building',
                    style: Theme.of(context)
                        .textTheme
                        .titleSmall
                        ?.copyWith(fontWeight: FontWeight.bold),
                  ),
                  const SizedBox(height: 8),
                  SegmentedButton<_BuildingVariant>(
                    segments: _BuildingVariant.values
                        .map((v) => ButtonSegment(
                              value: v,
                              label: Text(
                                v.label,
                                style: const TextStyle(fontSize: 11),
                              ),
                            ))
                        .toList(),
                    selected: {_variant},
                    onSelectionChanged: _isLoading
                        ? null
                        : (s) => _onVariantChanged(s.first),
                    showSelectedIcon: false,
                  ),

                  const SizedBox(height: 20),

                  // Layer checkboxes
                  Row(
                    children: [
                      Expanded(
                        child: Text(
                          'Discipline Layers',
                          style: Theme.of(context)
                              .textTheme
                              .titleSmall
                              ?.copyWith(fontWeight: FontWeight.bold),
                        ),
                      ),
                      TextButton(
                        onPressed: _isLoading
                            ? null
                            : () {
                                final allOn =
                                    _layers.every((l) => l.selected);
                                setState(() {
                                  for (final l in _layers) {
                                    l.selected = !allOn;
                                  }
                                });
                              },
                        child: Text(
                          _layers.every((l) => l.selected)
                              ? 'Deselect All'
                              : 'Select All',
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 4),

                  ..._layers.map((layer) => Card(
                        margin: const EdgeInsets.symmetric(vertical: 3),
                        child: CheckboxListTile(
                          value: layer.selected,
                          onChanged: _isLoading
                              ? null
                              : (v) => setState(
                                  () => layer.selected = v ?? false),
                          secondary: CircleAvatar(
                            backgroundColor:
                                layer.color.withValues(alpha: 0.2),
                            child: Icon(layer.icon,
                                color: layer.color, size: 20),
                          ),
                          title: Text(layer.label),
                          subtitle: Text(
                            layer.assetPath.split('/').last,
                            style: const TextStyle(fontSize: 11),
                            overflow: TextOverflow.ellipsis,
                          ),
                          dense: true,
                          controlAffinity: ListTileControlAffinity.trailing,
                        ),
                      )),

                  // Error
                  if (_error != null) ...[
                    const SizedBox(height: 12),
                    Container(
                      padding: const EdgeInsets.all(12),
                      decoration: BoxDecoration(
                        color: colorScheme.errorContainer,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Text(
                        _error!,
                        style:
                            TextStyle(color: colorScheme.onErrorContainer),
                      ),
                    ),
                  ],

                  // Progress
                  if (_isLoading) ...[
                    const SizedBox(height: 16),
                    LinearProgressIndicator(
                      value: _layers.isNotEmpty
                          ? _loadProgress / _selectedCount
                          : null,
                    ),
                    const SizedBox(height: 8),
                    Text(
                      'Loading $_loadingLabel ($_loadProgress/$_selectedCount)...',
                      textAlign: TextAlign.center,
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ],
                ],
              ),
            ),

            // Actions
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 0, 20, 20),
              child: Row(
                children: [
                  Expanded(
                    child: TextButton(
                      onPressed:
                          _isLoading ? null : () => Navigator.pop(context),
                      child: const Text('Cancel'),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    flex: 2,
                    child: FilledButton.icon(
                      onPressed:
                          _isLoading || _selectedCount == 0
                              ? null
                              : _loadSelected,
                      icon: _isLoading
                          ? const SizedBox(
                              width: 18,
                              height: 18,
                              child: CircularProgressIndicator(
                                  strokeWidth: 2, color: Colors.white),
                            )
                          : const Icon(Icons.layers),
                      label: Text(_isLoading
                          ? 'Loading...'
                          : 'Load $_selectedCount Layer(s)'),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
