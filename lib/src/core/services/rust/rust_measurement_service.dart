import '../../bridge/api.dart' as rust;
import '../../errors/bim_error.dart';
import '../measurement_service.dart';

/// Rust implementation of [IMeasurementService].
class RustMeasurementService implements IMeasurementService {
  @override
  void startMeasurement({required String measurementType}) {
    try {
      rust.startMeasurement(measurementType: measurementType);
    } catch (e) {
      throw BimMeasurementError(
        userMessage: 'Failed to start measurement',
        technicalMessage: 'startMeasurement($measurementType) failed: $e',
        cause: e,
      );
    }
  }

  @override
  int addMeasurementPoint({required double x, required double y, required double z}) {
    try {
      return rust.addMeasurementPoint(x: x, y: y, z: z);
    } catch (e) {
      throw BimMeasurementError(
        userMessage: 'Failed to add measurement point',
        technicalMessage: 'addMeasurementPoint($x, $y, $z) failed: $e',
        cause: e,
      );
    }
  }

  @override
  rust.MeasurementResult getMeasurementResult() {
    try {
      return rust.getMeasurementResult();
    } catch (e) {
      throw BimMeasurementError(
        userMessage: 'Failed to get measurement result',
        technicalMessage: 'getMeasurementResult failed: $e',
        cause: e,
      );
    }
  }

  @override
  int getMeasurementPointCount() {
    try {
      return rust.getMeasurementPointCount();
    } catch (e) {
      throw BimMeasurementError(
        userMessage: 'Failed to get point count',
        technicalMessage: 'getMeasurementPointCount failed: $e',
        cause: e,
      );
    }
  }

  @override
  void clearMeasurement() {
    try {
      rust.clearMeasurement();
    } catch (e) {
      throw BimMeasurementError(
        userMessage: 'Failed to clear measurement',
        technicalMessage: 'clearMeasurement failed: $e',
        cause: e,
      );
    }
  }
}
