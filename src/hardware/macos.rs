use std::process::Command;

pub fn get_cpu_id() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("sysctl")
        .args(&["-n", "machdep.cpu.brand_string"])
        .output()?;
    let result = String::from_utf8_lossy(&output.stdout);
    Ok(result.trim().to_string())
}

pub fn get_motherboard_id() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("ioreg")
        .args(&["-l"])
        .output()?;
    let result = String::from_utf8_lossy(&output.stdout);
    let id = result
        .lines()
        .find(|line| line.contains("IOPlatformSerialNumber"))
        .unwrap_or("")
        .split('=')
        .nth(1)
        .unwrap_or("")
        .trim()
        .replace("\"", "");
    Ok(id)
}
