import '../bridge/api.dart' show MeasurementResult;

/// Abstract interface for measurement operations.
abstract interface class IMeasurementService {
  /// Start a new measurement of the given type.
  void startMeasurement({required String measurementType});

  /// Add a measurement point.
  int addMeasurementPoint({required double x, required double y, required double z});

  /// Get the current measurement result.
  MeasurementResult getMeasurementResult();

  /// Get the number of measurement points.
  int getMeasurementPointCount();

  /// Clear the current measurement.
  void clearMeasurement();
}
