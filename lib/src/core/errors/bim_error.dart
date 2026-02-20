/// Sealed class hierarchy for structured BIM errors.
sealed class BimError implements Exception {
  /// User-friendly message suitable for display in UI.
  final String userMessage;

  /// Technical message for logging/debugging.
  final String technicalMessage;

  /// Original cause, if any.
  final Object? cause;

  const BimError({
    required this.userMessage,
    required this.technicalMessage,
    this.cause,
  });

  @override
  String toString() => '$runtimeType: $technicalMessage';
}

/// Error from the renderer subsystem (init, render, camera, etc.).
class BimRendererError extends BimError {
  const BimRendererError({
    required super.userMessage,
    required super.technicalMessage,
    super.cause,
  });
}

/// Error from model loading/parsing/management.
class BimModelError extends BimError {
  const BimModelError({
    required super.userMessage,
    required super.technicalMessage,
    super.cause,
  });
}

/// Error from element selection/picking.
class BimSelectionError extends BimError {
  const BimSelectionError({
    required super.userMessage,
    required super.technicalMessage,
    super.cause,
  });
}

/// Error from measurement operations.
class BimMeasurementError extends BimError {
  const BimMeasurementError({
    required super.userMessage,
    required super.technicalMessage,
    super.cause,
  });
}

/// Error from section plane operations.
class BimSectionPlaneError extends BimError {
  const BimSectionPlaneError({
    required super.userMessage,
    required super.technicalMessage,
    super.cause,
  });
}

/// Error from drawing overlay operations.
class BimOverlayError extends BimError {
  const BimOverlayError({
    required super.userMessage,
    required super.technicalMessage,
    super.cause,
  });
}
