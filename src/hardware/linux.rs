use std::process::Command;

pub fn get_cpu_id() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("lscpu")
        .args(&["-J"])
        .output()?;
    let result = String::from_utf8_lossy(&output.stdout);
    let id_line = result
        .lines()
        .find(|line| line.contains("Model name"))
        .unwrap_or("");
    Ok(id_line.split(':').nth(1).unwrap_or("").trim().to_string())
}

pub fn get_motherboard_id() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("cat")
        .args(&["/sys/devices/virtual/dmi/id/board_serial"])
        .output()?;
    let result = String::from_utf8_lossy(&output.stdout);
    Ok(result.trim().to_string())
}
