import 'package:flutter/material.dart';

/// Single source of truth for BIM element type visual mappings.
/// Replaces duplicated _getTypeIcon/_getTypeColor in element_tree.dart
/// and _getElementIcon/_getElementColor in properties_panel.dart.
class BimElementVisuals {
  BimElementVisuals._();

  /// Get the icon for a BIM element type.
  static IconData iconFor(String elementType) {
    switch (elementType.toUpperCase()) {
      case 'WALL':
        return Icons.view_agenda;
      case 'SLAB':
        return Icons.layers;
      case 'COLUMN':
        return Icons.view_column;
      case 'BEAM':
        return Icons.horizontal_rule;
      case 'DOOR':
        return Icons.door_front_door;
      case 'WINDOW':
        return Icons.window;
      case 'ROOF':
        return Icons.roofing;
      case 'STAIR':
        return Icons.stairs;
      case 'PIPE':
      case 'PIPESEGMENT':
        return Icons.plumbing;
      case 'DUCT':
      case 'DUCTSEGMENT':
        return Icons.air;
      case 'FLOWTERMINAL':
        return Icons.hvac;
      case 'CABLECARRIERSEGMENT':
        return Icons.electrical_services;
      case 'FOOTING':
        return Icons.foundation;
      case 'BUILDINGELEMENTPROXY':
        return Icons.category;
      default:
        return Icons.architecture;
    }
  }

  /// Get the color for a BIM element type.
  static Color colorFor(String elementType) {
    switch (elementType.toUpperCase()) {
      case 'WALL':
        return Colors.brown;
      case 'SLAB':
        return Colors.grey;
      case 'COLUMN':
        return Colors.blueGrey;
      case 'BEAM':
        return Colors.indigo;
      case 'DOOR':
        return Colors.amber;
      case 'WINDOW':
        return Colors.lightBlue;
      case 'ROOF':
        return Colors.deepOrange;
      case 'STAIR':
        return Colors.purple;
      case 'PIPE':
      case 'PIPESEGMENT':
        return Colors.teal;
      case 'DUCT':
      case 'DUCTSEGMENT':
        return Colors.cyan;
      case 'FLOWTERMINAL':
        return Colors.green;
      case 'CABLECARRIERSEGMENT':
        return Colors.orange;
      case 'FOOTING':
        return Colors.blueGrey;
      default:
        return Colors.grey;
    }
  }

  /// All known element types with default visibility.
  static const Map<String, bool> defaultVisibility = {
    'Wall': true,
    'Slab': true,
    'Column': true,
    'Beam': true,
    'Door': true,
    'Window': true,
    'Roof': true,
  };
}
