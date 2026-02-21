import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// State for accessibility settings within the BIM viewer.
@immutable
class AccessibilityState {
  /// Whether to use the system text scale (true) or the custom one (false).
  final bool useSystemTextScale;

  /// Custom text scale factor, used when [useSystemTextScale] is false.
  final double customTextScaleFactor;

  const AccessibilityState({
    this.useSystemTextScale = true,
    this.customTextScaleFactor = 1.0,
  });

  AccessibilityState copyWith({
    bool? useSystemTextScale,
    double? customTextScaleFactor,
  }) {
    return AccessibilityState(
      useSystemTextScale: useSystemTextScale ?? this.useSystemTextScale,
      customTextScaleFactor:
          customTextScaleFactor ?? this.customTextScaleFactor,
    );
  }
}

/// Notifier managing accessibility settings.
class AccessibilityNotifier extends Notifier<AccessibilityState> {
  @override
  AccessibilityState build() => const AccessibilityState();

  void setUseSystemTextScale(bool value) {
    state = state.copyWith(useSystemTextScale: value);
  }

  void setCustomTextScaleFactor(double value) {
    state = state.copyWith(customTextScaleFactor: value);
  }
}

/// Provider for accessibility state.
final accessibilityStateProvider =
    NotifierProvider<AccessibilityNotifier, AccessibilityState>(
        AccessibilityNotifier.new);
