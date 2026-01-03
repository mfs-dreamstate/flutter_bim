import 'dart:math';
import 'package:flutter/material.dart';
import 'package:flutter_map/flutter_map.dart';
import 'package:latlong2/latlong.dart';

/// Map view showing building location on OpenStreetMap
class MapViewScreen extends StatefulWidget {
  const MapViewScreen({super.key});

  @override
  State<MapViewScreen> createState() => _MapViewScreenState();
}

class _MapViewScreenState extends State<MapViewScreen> {
  final MapController _mapController = MapController();

  // Default location (will be updated from model georeferencing)
  LatLng _buildingLocation = const LatLng(55.6761, 12.5683); // Copenhagen as default
  double _buildingRotation = 0.0;
  bool _hasGeoReference = false;
  String? _siteName;

  // Building footprint polygon
  List<LatLng> _buildingFootprint = [];

  @override
  void initState() {
    super.initState();
    _loadGeoReference();
  }

  Future<void> _loadGeoReference() async {
    try {
      // TODO: Enable when FRB bindings are regenerated
      // Try to get georeferencing from the loaded model
      // final geoRef = rust.getGeoReference();
      // For now, use demo mode - georeferencing will work after running:
      // flutter_rust_bridge_codegen generate

      // Simulated georeferencing for demo
      // In production, uncomment the rust.getGeoReference() call above
      final hasGeoData = false; // Set to true to test with demo coordinates

      if (hasGeoData) {
        setState(() {
          // Demo coordinates (Copenhagen)
          _buildingLocation = const LatLng(55.6761, 12.5683);
          _buildingRotation = 0.0;
          _hasGeoReference = true;
          _siteName = 'Demo Building';

          // Generate a simple rectangular footprint around the building
          _buildingFootprint = _generateFootprint(
            _buildingLocation,
            30.0, // width in meters
            20.0, // depth in meters
            _buildingRotation,
          );
        });

        // Move map to building location
        _mapController.move(_buildingLocation, 17.0);
      }
    } catch (e) {
      debugPrint('[MAP] No georeferencing available: $e');
    }
  }

  List<LatLng> _generateFootprint(LatLng center, double width, double depth, double rotation) {
    // Convert meters to approximate degrees (rough approximation)
    final latOffset = depth / 111320.0; // ~111km per degree latitude
    final lngOffset = width / (111320.0 * cos(center.latitude * pi / 180));

    // Simple rectangular footprint
    final corners = [
      LatLng(center.latitude - latOffset / 2, center.longitude - lngOffset / 2),
      LatLng(center.latitude - latOffset / 2, center.longitude + lngOffset / 2),
      LatLng(center.latitude + latOffset / 2, center.longitude + lngOffset / 2),
      LatLng(center.latitude + latOffset / 2, center.longitude - lngOffset / 2),
    ];

    // TODO: Apply rotation if needed
    return corners;
  }

  void _centerOnBuilding() {
    _mapController.move(_buildingLocation, 17.0);
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      body: Stack(
        children: [
          // Map
          FlutterMap(
            mapController: _mapController,
            options: MapOptions(
              initialCenter: _buildingLocation,
              initialZoom: 15.0,
              minZoom: 3.0,
              maxZoom: 19.0,
            ),
            children: [
              // OpenStreetMap tile layer
              TileLayer(
                urlTemplate: 'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
                userAgentPackageName: 'com.example.bim_viewer',
                maxZoom: 19,
              ),

              // Building footprint polygon
              if (_buildingFootprint.isNotEmpty)
                PolygonLayer(
                  polygons: [
                    Polygon(
                      points: _buildingFootprint,
                      color: colorScheme.primary.withValues(alpha: 0.3),
                      borderColor: colorScheme.primary,
                      borderStrokeWidth: 2.0,
                    ),
                  ],
                ),

              // Building marker
              MarkerLayer(
                markers: [
                  Marker(
                    point: _buildingLocation,
                    width: 50,
                    height: 50,
                    child: GestureDetector(
                      onTap: () {
                        _showBuildingInfo(context);
                      },
                      child: Container(
                        decoration: BoxDecoration(
                          color: colorScheme.primary,
                          shape: BoxShape.circle,
                          boxShadow: [
                            BoxShadow(
                              color: Colors.black.withValues(alpha: 0.3),
                              blurRadius: 8,
                              offset: const Offset(0, 2),
                            ),
                          ],
                        ),
                        child: Icon(
                          Icons.apartment,
                          color: colorScheme.onPrimary,
                          size: 28,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ],
          ),

          // Info panel
          Positioned(
            top: MediaQuery.of(context).padding.top + 16,
            left: 16,
            right: 16,
            child: Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.surface.withValues(alpha: 0.95),
                borderRadius: BorderRadius.circular(12),
                boxShadow: [
                  BoxShadow(
                    color: Colors.black.withValues(alpha: 0.1),
                    blurRadius: 8,
                  ),
                ],
              ),
              child: Row(
                children: [
                  Icon(
                    Icons.map,
                    color: colorScheme.primary,
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(
                          _siteName ?? 'Building Location',
                          style: Theme.of(context).textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                        Text(
                          _hasGeoReference
                              ? '${_buildingLocation.latitude.toStringAsFixed(5)}, ${_buildingLocation.longitude.toStringAsFixed(5)}'
                              : 'No georeferencing data',
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                  if (!_hasGeoReference)
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                      decoration: BoxDecoration(
                        color: colorScheme.errorContainer,
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(
                        'Demo',
                        style: TextStyle(
                          color: colorScheme.onErrorContainer,
                          fontSize: 12,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ),

          // Center on building button
          Positioned(
            bottom: 24,
            right: 16,
            child: FloatingActionButton(
              onPressed: _centerOnBuilding,
              tooltip: 'Center on Building',
              child: const Icon(Icons.my_location),
            ),
          ),

          // Map attribution
          Positioned(
            bottom: 8,
            left: 8,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
              decoration: BoxDecoration(
                color: Colors.white.withValues(alpha: 0.8),
                borderRadius: BorderRadius.circular(4),
              ),
              child: const Text(
                '© OpenStreetMap contributors',
                style: TextStyle(fontSize: 10, color: Colors.black54),
              ),
            ),
          ),
        ],
      ),
    );
  }

  void _showBuildingInfo(BuildContext context) {
    showModalBottomSheet(
      context: context,
      builder: (context) => Container(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  Icons.apartment,
                  size: 32,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(width: 16),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        _siteName ?? 'Building',
                        style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      if (_hasGeoReference)
                        Text(
                          'Georeferenced from IFC',
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: Theme.of(context).colorScheme.primary,
                          ),
                        ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 24),
            _InfoRow('Latitude', _buildingLocation.latitude.toStringAsFixed(6)),
            _InfoRow('Longitude', _buildingLocation.longitude.toStringAsFixed(6)),
            if (_buildingRotation != 0)
              _InfoRow('Rotation', '${_buildingRotation.toStringAsFixed(1)}°'),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  final String label;
  final String value;

  const _InfoRow(this.label, this.value);

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(
            label,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
          Text(
            value,
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              fontWeight: FontWeight.w500,
              fontFamily: 'monospace',
            ),
          ),
        ],
      ),
    );
  }
}

/// Georeferencing data from IFC
class GeoReference {
  final double latitude;
  final double longitude;
  final double rotation;
  final double width;
  final double depth;
  final String? siteName;

  GeoReference({
    required this.latitude,
    required this.longitude,
    this.rotation = 0.0,
    this.width = 30.0,
    this.depth = 20.0,
    this.siteName,
  });
}
