import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../constants/render_config.dart';
import '../services/renderer_service.dart';
import 'service_providers.dart';

/// State for the renderer.
@immutable
class RendererState {
  final bool isInitializing;
  final bool isInitialized;
  final String status;
  final String? error;
  final bool wireframeSupported;
  final int renderMode; // 0 = Shaded, 1 = Wireframe, 2 = X-Ray

  const RendererState({
    this.isInitializing = true,
    this.isInitialized = false,
    this.status = 'Initializing renderer...',
    this.error,
    this.wireframeSupported = false,
    this.renderMode = 0,
  });

  RendererState copyWith({
    bool? isInitializing,
    bool? isInitialized,
    String? status,
    String? Function()? error,
    bool? wireframeSupported,
    int? renderMode,
  }) {
    return RendererState(
      isInitializing: isInitializing ?? this.isInitializing,
      isInitialized: isInitialized ?? this.isInitialized,
      status: status ?? this.status,
      error: error != null ? error() : this.error,
      wireframeSupported: wireframeSupported ?? this.wireframeSupported,
      renderMode: renderMode ?? this.renderMode,
    );
  }
}

/// Notifier managing renderer initialization and state.
class RendererNotifier extends Notifier<RendererState> {
  @override
  RendererState build() => const RendererState();

  IRendererService get _service => ref.read(rendererServiceProvider);

  Future<void> initialize() async {
    state = state.copyWith(
      isInitializing: true,
      error: () => null,
      status: 'Initializing GPU...',
    );

    try {
      final result = await _service.initialize(
        width: RenderConfig.width,
        height: RenderConfig.height,
      );
      final wireframeSupported = _service.isWireframeSupported();

      state = state.copyWith(
        isInitializing: false,
        isInitialized: true,
        status: result,
        wireframeSupported: wireframeSupported,
      );
    } catch (e) {
      state = state.copyWith(
        isInitializing: false,
        error: () => e.toString(),
      );
    }
  }

  void toggleRenderMode() {
    final newMode = state.renderMode == 0 ? 1 : 0;
    _service.setRenderMode(mode: newMode);
    state = state.copyWith(renderMode: newMode);
  }

  void setRenderMode(int mode) {
    _service.setRenderMode(mode: mode);
    state = state.copyWith(renderMode: mode);
  }

  void setStatus(String status) {
    state = state.copyWith(status: status);
  }

  void setError(String? error) {
    state = state.copyWith(error: () => error);
  }
}

/// Provider for the renderer state.
final rendererStateProvider =
    NotifierProvider<RendererNotifier, RendererState>(RendererNotifier.new);
