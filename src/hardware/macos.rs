use std::error::Error;
use std::process::Command;

/// Get a CPU brand string on macOS.
///
/// Uses `sysctl machdep.cpu.brand_string`.
pub fn get_cpu_id() -> Result<String, Box<dyn Error>> {
    let output = Command::new("sysctl")
        .args(&["-n", "machdep.cpu.brand_string"])
        .output()?;

    let result = String::from_utf8_lossy(&output.stdout);
    let value = result.trim().to_string();

    if value.is_empty() {
        Ok("macos_cpu_unknown".to_string())
    } else {
        Ok(value)
    }
}

/// Get the platform serial number on macOS.
///
/// Uses `ioreg -l` and searches for `IOPlatformSerialNumber`.
pub fn get_motherboard_id() -> Result<String, Box<dyn Error>> {
    let output = Command::new("ioreg")
        .args(&["-l"])
        .output()?;

    let result = String::from_utf8_lossy(&output.stdout);

    if let Some(line) = result.lines().find(|line| line.contains("IOPlatformSerialNumber")) {
        let value = line
            .split('=')
            .nth(1)
            .unwrap_or("")
            .trim()
            .trim_matches('"')
            .to_string();

        if !value.is_empty() {
            return Ok(value);
        }
    }

    Ok("macos_mb_unknown".to_string())
}
