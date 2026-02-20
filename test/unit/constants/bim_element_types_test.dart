import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_bim/src/core/constants/bim_element_types.dart';

void main() {
  group('BimElementVisuals', () {
    final knownTypes = [
      'Wall',
      'Slab',
      'Column',
      'Beam',
      'Door',
      'Window',
      'Roof',
      'Stair',
      'Pipe',
      'PipeSegment',
      'Duct',
      'DuctSegment',
      'FlowTerminal',
      'CableCarrierSegment',
      'Footing',
      'BuildingElementProxy',
    ];

    test('every known type has a non-default icon', () {
      for (final type in knownTypes) {
        final icon = BimElementVisuals.iconFor(type);
        expect(icon, isNotNull, reason: '$type should have an icon');
        // All known types should NOT fall through to the default (Icons.architecture)
        expect(icon, isNot(equals(Icons.architecture)),
            reason: '$type should have a specific icon, not the default');
      }
    });

    test('every known type has a non-default color', () {
      for (final type in knownTypes) {
        final color = BimElementVisuals.colorFor(type);
        expect(color, isNotNull, reason: '$type should have a color');
      }
    });

    test('unknown type returns default icon and color', () {
      expect(BimElementVisuals.iconFor('UnknownType'), equals(Icons.architecture));
      expect(BimElementVisuals.colorFor('UnknownType'), equals(Colors.grey));
    });

    test('iconFor is case-insensitive', () {
      expect(BimElementVisuals.iconFor('wall'), equals(BimElementVisuals.iconFor('WALL')));
      expect(BimElementVisuals.iconFor('Wall'), equals(BimElementVisuals.iconFor('WALL')));
    });

    test('colorFor is case-insensitive', () {
      expect(BimElementVisuals.colorFor('wall'), equals(BimElementVisuals.colorFor('WALL')));
      expect(BimElementVisuals.colorFor('Wall'), equals(BimElementVisuals.colorFor('WALL')));
    });

    test('defaultVisibility contains expected element types', () {
      expect(BimElementVisuals.defaultVisibility, containsPair('Wall', true));
      expect(BimElementVisuals.defaultVisibility, containsPair('Slab', true));
      expect(BimElementVisuals.defaultVisibility, containsPair('Column', true));
      expect(BimElementVisuals.defaultVisibility, containsPair('Beam', true));
      expect(BimElementVisuals.defaultVisibility, containsPair('Door', true));
      expect(BimElementVisuals.defaultVisibility, containsPair('Window', true));
      expect(BimElementVisuals.defaultVisibility, containsPair('Roof', true));
      expect(BimElementVisuals.defaultVisibility.length, equals(7));
    });
  });
}
