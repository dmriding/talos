use std::error::Error;
use std::fs;
use std::process::Command;

/// Try to read CPU model name from /proc/cpuinfo.
///
/// This is usually more reliable and cheaper than spawning `lscpu`.
pub fn get_cpu_id() -> Result<String, Box<dyn Error>> {
    // Primary path: /proc/cpuinfo
    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        if let Some(line) = cpuinfo.lines().find(|l| l.to_lowercase().starts_with("model name")) {
            let value = line.split(':').nth(1).unwrap_or("").trim().to_string();
            if !value.is_empty() {
                return Ok(value);
            }
        }
    }

    // Fallback: use `lscpu` if available
    let output = Command::new("lscpu").output()?;
    let result = String::from_utf8_lossy(&output.stdout);

    if let Some(line) = result
        .lines()
        .find(|l| l.to_lowercase().contains("model name"))
    {
        let value = line.split(':').nth(1).unwrap_or("").trim().to_string();
        if !value.is_empty() {
            return Ok(value);
        }
    }

    // Final fallback: something deterministic but generic
    Ok("linux_cpu_unknown".to_string())
}

/// Get the motherboard/board serial ID on Linux.
///
/// Reads /sys/devices/virtual/dmi/id/board_serial if present.
/// Returns a generic fallback if not available.
pub fn get_motherboard_id() -> Result<String, Box<dyn Error>> {
    match fs::read_to_string("/sys/devices/virtual/dmi/id/board_serial") {
        Ok(contents) => {
            let value = contents.trim().to_string();
            if !value.is_empty() {
                Ok(value)
            } else {
                Ok("linux_mb_unknown".to_string())
            }
        }
        Err(_) => Ok("linux_mb_unknown".to_string()),
    }
}
