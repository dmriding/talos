#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;

/// Retrieves a unique hardware ID based on CPU ID and motherboard ID
pub fn get_hardware_id() -> String {
    let cpu_id = get_cpu_id().unwrap_or_else(|_| "unknown".to_string());
    let motherboard_id = get_motherboard_id().unwrap_or_else(|_| "unknown".to_string());
    format!("{}-{}", cpu_id, motherboard_id)
}

/// Get the CPU ID for the current system (platform-specific)
fn get_cpu_id() -> Result<String, Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        windows::get_cpu_id()
    }
    #[cfg(target_os = "macos")]
    {
        macos::get_cpu_id()
    }
    #[cfg(target_os = "linux")]
    {
        linux::get_cpu_id()
    }
}

/// Get the motherboard ID for the current system (platform-specific)
fn get_motherboard_id() -> Result<String, Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        windows::get_motherboard_id()
    }
    #[cfg(target_os = "macos")]
    {
        macos::get_motherboard_id()
    }
    #[cfg(target_os = "linux")]
    {
        linux::get_motherboard_id()
    }
}
