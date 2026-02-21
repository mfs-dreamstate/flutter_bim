use flutter_rust_bridge::frb;

/// Initialize the BIM viewer engine
/// This should be called once when the app starts
#[frb(sync)]
pub fn initialize() -> String {
    let _ = tracing_subscriber::fmt::try_init();
    tracing::info!("BIM Viewer initialized");
    "BIM Viewer initialized successfully".to_string()
}

/// Get the library version
#[frb(sync)]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get system information for debugging
#[frb(sync)]
pub fn get_system_info() -> String {
    format!(
        "Rust Version: {}\nTarget: {}\nOS: {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
        std::env::consts::OS
    )
}

/// Test async functionality
pub async fn test_async() -> String {
    tracing::debug!("Starting async test");
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    tracing::debug!("Async test completed");
    "Async test completed successfully".to_string()
}

/// Test error handling across FFI
#[frb(sync)]
pub fn test_error_handling(should_fail: bool) -> Result<String, String> {
    if should_fail {
        Err("Intentional error for testing".to_string())
    } else {
        Ok("Success!".to_string())
    }
}
