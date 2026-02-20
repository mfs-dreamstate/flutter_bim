import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../core/bridge/bim/model.dart';
import '../../core/providers/selection_state.dart';
import '../properties_panel.dart';

/// Bottom bar showing the currently selected element.
class SelectionInfoBar extends ConsumerWidget {
  const SelectionInfoBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selectedElement = ref.watch(selectionStateProvider);
    if (selectedElement == null) return const SizedBox.shrink();

    return Positioned(
      left: 16,
      right: 16,
      bottom: 16,
      child: GestureDetector(
        onTap: () => _showPropertiesPanel(context, ref, selectedElement),
        child: Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: Theme.of(context)
                .colorScheme
                .surface
                .withValues(alpha: 0.95),
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
                padding:
                    const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  selectedElement.elementType,
                  style: TextStyle(
                    fontWeight: FontWeight.bold,
                    color:
                        Theme.of(context).colorScheme.onPrimaryContainer,
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
                      selectedElement.name.isEmpty
                          ? 'Unnamed Element'
                          : selectedElement.name,
                      style: Theme.of(context).textTheme.titleSmall,
                    ),
                    Text(
                      'Tap for details',
                      style:
                          Theme.of(context).textTheme.bodySmall?.copyWith(
                                color: Theme.of(context).colorScheme.primary,
                              ),
                    ),
                  ],
                ),
              ),
              IconButton(
                icon: const Icon(Icons.info_outline, size: 20),
                onPressed: () =>
                    _showPropertiesPanel(context, ref, selectedElement),
                tooltip: 'View properties',
              ),
              IconButton(
                icon: const Icon(Icons.close, size: 20),
                onPressed: () =>
                    ref.read(selectionStateProvider.notifier).clearSelection(),
                tooltip: 'Clear selection',
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _showPropertiesPanel(
    BuildContext context,
    WidgetRef ref,
    ElementInfo element,
  ) {
    showPropertiesPanel(
      context,
      element: element,
      onFocusElement: () {
        // TODO: Implement focus on specific element bounds
      },
    );
  }
}
