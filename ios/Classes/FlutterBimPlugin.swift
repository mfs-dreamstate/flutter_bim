import Flutter
import UIKit

public class FlutterBimPlugin: NSObject, FlutterPlugin {
  public static func register(with registrar: FlutterPluginRegistrar) {
    // FFI plugin - no method channel needed
    // The Rust library is loaded via FFI from Dart
  }
}
