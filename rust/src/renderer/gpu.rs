//! GPU Context Management
//!
//! Handles wgpu instance, adapter, device, and queue initialization.

/// GPU context wrapping wgpu resources
pub struct GpuContext {
    pub instance: Option<wgpu::Instance>,
    pub adapter: Option<wgpu::Adapter>,
    pub device: Option<wgpu::Device>,
    pub queue: Option<wgpu::Queue>,
}

impl GpuContext {
    /// Create a new uninitialized GPU context
    pub fn new() -> Self {
        Self {
            instance: None,
            adapter: None,
            device: None,
            queue: None,
        }
    }

    /// Initialize wgpu (headless for now, surface will be added later)
    pub async fn initialize(&mut self) -> Result<(), String> {
        tracing::info!("Initializing wgpu");

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find suitable GPU adapter")?;

        tracing::info!(
            "Selected adapter: {:?}",
            adapter.get_info()
        );

        // Check which optional features are supported
        let adapter_features = adapter.features();
        let mut required_features = wgpu::Features::empty();

        // Request POLYGON_MODE_LINE if available (for wireframe rendering)
        if adapter_features.contains(wgpu::Features::POLYGON_MODE_LINE) {
            required_features |= wgpu::Features::POLYGON_MODE_LINE;
            tracing::info!("POLYGON_MODE_LINE feature enabled (wireframe supported)");
        } else {
            tracing::warn!("POLYGON_MODE_LINE not supported - wireframe mode unavailable");
        }

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("BIM Viewer Device"),
                    required_features,
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .map_err(|e| format!("Failed to create device: {}", e))?;

        tracing::info!("GPU device and queue created successfully");

        self.instance = Some(instance);
        self.adapter = Some(adapter);
        self.device = Some(device);
        self.queue = Some(queue);

        Ok(())
    }

    /// Check if GPU is initialized
    pub fn is_initialized(&self) -> bool {
        self.device.is_some() && self.queue.is_some()
    }

    /// Get device reference
    pub fn device(&self) -> Option<&wgpu::Device> {
        self.device.as_ref()
    }

    /// Get queue reference
    pub fn queue(&self) -> Option<&wgpu::Queue> {
        self.queue.as_ref()
    }

    /// Check if wireframe rendering is supported
    pub fn wireframe_supported(&self) -> bool {
        self.device
            .as_ref()
            .map(|d| d.features().contains(wgpu::Features::POLYGON_MODE_LINE))
            .unwrap_or(false)
    }
}
