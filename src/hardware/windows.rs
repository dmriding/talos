use std::error::Error;
use std::process::Command;

/// Try WMIC with normal formatting.
///
/// Returns Some(String) if a non-empty value was found.
fn try_wmic(args: &[&str]) -> Option<String> {
    let output = Command::new("wmic").args(args).output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);

    // WMIC output usually has header in first line, value in second line.
    let value = text
        .lines()
        .nth(1)
        .unwrap_or("")
        .trim()
        .to_string();

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Fallback: parse WMIC in list format (`key=value`).
fn try_wmic_list(class: &str, key: &str) -> Option<String> {
    let output = Command::new("wmic")
        .args(&[class, "get", "/format:list"])
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);

    for line in text.lines() {
        if line.starts_with(&format!("{}=", key)) {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() == 2 {
                let value = parts[1].trim().to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

/// Get Windows CPU ID.
///
/// Attempts:
/// 1. WMIC cpu get ProcessorId
/// 2. WMIC cpu get /format:list
/// 3. fallback deterministic value
pub fn get_cpu_id() -> Result<String, Box<dyn Error>> {
    // WMIC normal format
    if let Some(val) = try_wmic(&["cpu", "get", "ProcessorId"]) {
        return Ok(val);
    }

    // WMIC list fallback
    if let Some(val) = try_wmic_list("cpu", "ProcessorId") {
        return Ok(val);
    }

    Ok("windows_cpu_unknown".to_string())
}

/// Get Windows Motherboard ID.
///
/// Attempts:
/// 1. WMIC baseboard get SerialNumber
/// 2. WMIC baseboard list format
/// 3. fallback deterministic ID
pub fn get_motherboard_id() -> Result<String, Box<dyn Error>> {
    if let Some(val) = try_wmic(&["baseboard", "get", "SerialNumber"]) {
        return Ok(val);
    }

    if let Some(val) = try_wmic_list("baseboard", "SerialNumber") {
        return Ok(val);
    }

    Ok("windows_mb_unknown".to_string())
}
