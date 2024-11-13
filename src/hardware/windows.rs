use std::process::Command;

pub fn get_cpu_id() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("wmic")
        .args(&["cpu", "get", "ProcessorId"])
        .output()?;
    let result = String::from_utf8_lossy(&output.stdout);
    let id = result.lines().nth(1).unwrap_or("").trim();
    Ok(id.to_string())
}

pub fn get_motherboard_id() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("wmic")
        .args(&["baseboard", "get", "SerialNumber"])
        .output()?;
    let result = String::from_utf8_lossy(&output.stdout);
    let id = result.lines().nth(1).unwrap_or("").trim();
    Ok(id.to_string())
}
