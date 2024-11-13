use std::process::Command;

pub fn get_cpu_id() -> String {
    let output = Command::new("wmic")
        .args(&["cpu", "get", "ProcessorId"])
        .output()
        .expect("Failed to execute command");
    let result = String::from_utf8_lossy(&output.stdout);
    let id = result.lines().nth(1).unwrap_or("").trim();
    id.to_string()
}

pub fn get_motherboard_id() -> String {
    let output = Command::new("wmic")
        .args(&["baseboard", "get", "SerialNumber"])
        .output()
        .expect("Failed to execute command");
    let result = String::from_utf8_lossy(&output.stdout);
    let id = result.lines().nth(1).unwrap_or("").trim();
    id.to_string()
}
