import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'viewer/bim_viewer_shell.dart';

/// Main BIM viewer widget.
///
/// This is the public API entry point. It wraps the internal viewer
/// in a [ProviderScope] so consumers don't need to depend on Riverpod.
class ViewerScreen extends StatelessWidget {
  const ViewerScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return const ProviderScope(
      child: BimViewerShell(),
    );
  }
}
