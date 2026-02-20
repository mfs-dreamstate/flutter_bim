import 'package:flutter/material.dart';
import '../core/errors/bim_error.dart';

/// A widget that catches errors in its child and displays a retry UI.
class ErrorBoundary extends StatefulWidget {
  final Widget child;
  final VoidCallback? onRetry;

  const ErrorBoundary({
    super.key,
    required this.child,
    this.onRetry,
  });

  @override
  State<ErrorBoundary> createState() => _ErrorBoundaryState();
}

class _ErrorBoundaryState extends State<ErrorBoundary> {
  String? _error;

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      return Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.error_outline,
              size: 64,
              color: Theme.of(context).colorScheme.error,
            ),
            const SizedBox(height: 16),
            Text(
              'Something went wrong',
              style: Theme.of(context).textTheme.headlineSmall,
            ),
            const SizedBox(height: 8),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 32),
              child: SelectableText(
                _error!,
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodyMedium,
              ),
            ),
            const SizedBox(height: 24),
            if (widget.onRetry != null)
              ElevatedButton.icon(
                onPressed: () {
                  setState(() => _error = null);
                  widget.onRetry!();
                },
                icon: const Icon(Icons.refresh),
                label: const Text('Retry'),
              ),
          ],
        ),
      );
    }

    return widget.child;
  }

  /// Call this to display an error state.
  void showError(String message) {
    setState(() => _error = message);
  }

  /// Clear the error state.
  void clearError() {
    setState(() => _error = null);
  }
}

/// Show a BimError as a snackbar.
void showBimError(BuildContext context, BimError error) {
  ScaffoldMessenger.of(context).showSnackBar(
    SnackBar(
      content: Text(error.userMessage),
      backgroundColor: Theme.of(context).colorScheme.error,
      duration: const Duration(seconds: 4),
      action: SnackBarAction(
        label: 'Details',
        textColor: Theme.of(context).colorScheme.onError,
        onPressed: () {
          showDialog(
            context: context,
            builder: (context) => AlertDialog(
              title: const Text('Error Details'),
              content: SelectableText(error.technicalMessage),
              actions: [
                TextButton(
                  onPressed: () => Navigator.pop(context),
                  child: const Text('OK'),
                ),
              ],
            ),
          );
        },
      ),
    ),
  );
}

/// Show a generic error string as a snackbar.
void showErrorSnackbar(BuildContext context, String message) {
  ScaffoldMessenger.of(context).showSnackBar(
    SnackBar(
      content: Text(message),
      backgroundColor: Theme.of(context).colorScheme.error,
      duration: const Duration(seconds: 4),
    ),
  );
}
