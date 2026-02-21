import 'package:flutter/material.dart';
import '../core/bridge/api/clash.dart' as clash;

/// Clash detection panel — run detection, view results, navigate to clashes.
class ClashDetectionPanel extends StatefulWidget {
  const ClashDetectionPanel({super.key});

  @override
  State<ClashDetectionPanel> createState() => _ClashDetectionPanelState();
}

class _ClashDetectionPanelState extends State<ClashDetectionPanel> {
  clash.ClashSummary? _summary;
  List<clash.ClashInfo> _results = [];
  bool _running = false;
  String? _selectedGroup;

  // Tolerance settings
  double _hardTolerance = 0.0;
  double _softTolerance = 0.025; // 25mm
  double _clearanceTolerance = 0.1; // 100mm
  bool _checkSameType = false;

  void _runDetection() {
    setState(() => _running = true);
    try {
      final summary = clash.runClashDetection(
        hardTolerance: _hardTolerance,
        softTolerance: _softTolerance,
        clearanceTolerance: _clearanceTolerance,
        checkSameType: _checkSameType,
      );
      final results = clash.getClashResults();
      setState(() {
        _summary = summary;
        _results = results;
        _selectedGroup = null;
        _running = false;
      });
    } catch (e) {
      setState(() => _running = false);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Clash detection failed: $e')),
        );
      }
    }
  }

  void _filterByGroup(String? group) {
    setState(() => _selectedGroup = group);
    try {
      if (group == null) {
        _results = clash.getClashResults();
      } else {
        _results = clash.getClashesByGroup(groupName: group);
      }
      setState(() {});
    } catch (e) {
      debugPrint('[CLASH] filter failed: $e');
    }
  }

  void _navigateToClash(int id) {
    try {
      clash.navigateToClash(clashId: id);
      if (mounted) Navigator.pop(context);
    } catch (e) {
      debugPrint('[CLASH] navigate failed: $e');
    }
  }

  void _clearResults() {
    try {
      clash.clearClashResults();
      setState(() {
        _summary = null;
        _results = [];
        _selectedGroup = null;
      });
    } catch (_) {}
  }

  void _exportCsv() {
    try {
      final csv = clash.exportClashCsv();
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('CSV exported (${csv.length} chars)')),
        );
      }
    } catch (e) {
      debugPrint('[CLASH] export failed: $e');
    }
  }

  String _clashTypeLabel(int type) {
    switch (type) {
      case 0:
        return 'Hard';
      case 1:
        return 'Soft';
      case 2:
        return 'Clearance';
      default:
        return 'Unknown';
    }
  }

  Color _clashTypeColor(int type) {
    switch (type) {
      case 0:
        return Colors.red;
      case 1:
        return Colors.orange;
      case 2:
        return Colors.amber;
      default:
        return Colors.grey;
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Container(
      constraints: BoxConstraints(
        maxHeight: MediaQuery.of(context).size.height * 0.8,
      ),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: colorScheme.surface,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Header
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('Clash Detection',
                  style: theme.textTheme.titleLarge
                      ?.copyWith(fontWeight: FontWeight.bold)),
              Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (_summary != null)
                    IconButton(
                      icon: const Icon(Icons.download, size: 20),
                      tooltip: 'Export CSV',
                      onPressed: _exportCsv,
                    ),
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: () => Navigator.pop(context),
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 12),

          if (_summary == null) ...[
            // Settings
            _toleranceSlider('Hard tolerance', _hardTolerance, 0.0, 0.1,
                (v) => setState(() => _hardTolerance = v)),
            _toleranceSlider('Soft tolerance', _softTolerance, 0.0, 0.2,
                (v) => setState(() => _softTolerance = v)),
            _toleranceSlider('Clearance tolerance', _clearanceTolerance, 0.0,
                0.5, (v) => setState(() => _clearanceTolerance = v)),
            SwitchListTile(
              title: const Text('Check same element types'),
              value: _checkSameType,
              onChanged: (v) => setState(() => _checkSameType = v),
              dense: true,
            ),
            const SizedBox(height: 12),
            FilledButton.icon(
              onPressed: _running ? null : _runDetection,
              icon: _running
                  ? const SizedBox(
                      width: 18,
                      height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2))
                  : const Icon(Icons.search),
              label: Text(_running ? 'Running...' : 'Run Clash Detection'),
            ),
          ] else ...[
            // Summary
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.primaryContainer,
                borderRadius: BorderRadius.circular(8),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceAround,
                children: [
                  _summaryChip('Total', _summary!.totalClashes, colorScheme.primary),
                  _summaryChip('Hard', _summary!.hardClashes, Colors.red),
                  _summaryChip('Soft', _summary!.softClashes, Colors.orange),
                  _summaryChip('Clear.', _summary!.clearanceClashes, Colors.amber),
                ],
              ),
            ),
            const SizedBox(height: 8),
            // Group filter chips
            if (_summary!.groups.isNotEmpty)
              SizedBox(
                height: 36,
                child: ListView(
                  scrollDirection: Axis.horizontal,
                  children: [
                    _filterChip('All', null),
                    ..._summary!.groups
                        .map((g) => _filterChip('${g.name} (${g.count})', g.name)),
                  ],
                ),
              ),
            const SizedBox(height: 8),
            // Actions
            Row(
              children: [
                TextButton.icon(
                  onPressed: _clearResults,
                  icon: const Icon(Icons.refresh, size: 18),
                  label: const Text('New Run'),
                ),
              ],
            ),
            const Divider(),
            // Results list
            Flexible(
              child: _results.isEmpty
                  ? const Padding(
                      padding: EdgeInsets.all(24),
                      child: Text('No clashes found',
                          textAlign: TextAlign.center),
                    )
                  : ListView.builder(
                      shrinkWrap: true,
                      itemCount: _results.length,
                      itemBuilder: (context, index) {
                        final c = _results[index];
                        return ListTile(
                          dense: true,
                          leading: CircleAvatar(
                            radius: 14,
                            backgroundColor: _clashTypeColor(c.clashType),
                            child: Text('${c.id}',
                                style: const TextStyle(
                                    fontSize: 10, color: Colors.white)),
                          ),
                          title: Text(
                            '${c.elementAName} vs ${c.elementBName}',
                            style: theme.textTheme.bodySmall,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          subtitle: Text(
                            '${_clashTypeLabel(c.clashType)} | ${(c.distance * 1000).toStringAsFixed(1)}mm',
                            style: theme.textTheme.labelSmall,
                          ),
                          trailing: const Icon(Icons.navigate_next, size: 18),
                          onTap: () => _navigateToClash(c.id),
                        );
                      },
                    ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _toleranceSlider(
      String label, double value, double min, double max, ValueChanged<double> onChanged) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          SizedBox(
              width: 140,
              child: Text(label, style: Theme.of(context).textTheme.bodySmall)),
          Expanded(
            child: Slider(
              value: value,
              min: min,
              max: max,
              divisions: 20,
              label: '${(value * 1000).round()}mm',
              onChanged: onChanged,
            ),
          ),
          SizedBox(
            width: 50,
            child: Text('${(value * 1000).round()}mm',
                style: Theme.of(context).textTheme.labelSmall),
          ),
        ],
      ),
    );
  }

  Widget _summaryChip(String label, int count, Color color) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('$count',
            style: TextStyle(
                fontSize: 20, fontWeight: FontWeight.bold, color: color)),
        Text(label, style: Theme.of(context).textTheme.labelSmall),
      ],
    );
  }

  Widget _filterChip(String label, String? group) {
    final selected = _selectedGroup == group;
    return Padding(
      padding: const EdgeInsets.only(right: 6),
      child: FilterChip(
        label: Text(label, style: const TextStyle(fontSize: 12)),
        selected: selected,
        onSelected: (_) => _filterByGroup(group),
        visualDensity: VisualDensity.compact,
      ),
    );
  }
}

/// Show clash detection panel as a bottom sheet.
void showClashDetectionPanel(BuildContext context) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (_) => const ClashDetectionPanel(),
  );
}
