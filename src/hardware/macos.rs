use std::process::Command;

pub fn get_cpu_id() -> String {
    let output = Command::new("sysctl")
        .args(&["-n", "machdep.cpu.brand_string"])
        .output()
        .expect("Failed to execute command");
    let result = String::from_utf8_lossy(&output.stdout);
    result.trim().to_string()
}

pub fn get_motherboard_id() -> String {
    let output = Command::new("ioreg")
        .args(&["-l"])
        .output()
        .expect("Failed to execute command");
    let result = String::from_utf8_lossy(&output.stdout);
    let id = result.lines().find(|line| line.contains("IOPlatformSerialNumber"))
        .unwrap_or("")
        .split('=')
        .nth(1)
        .unwrap_or("")
        .trim()
        .replace("\"", "");
    id
}
