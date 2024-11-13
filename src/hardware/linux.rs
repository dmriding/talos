use std::process::Command;

pub fn get_cpu_id() -> String {
    let output = Command::new("lscpu")
        .args(&["-J"])
        .output()
        .expect("Failed to execute command");
    let result = String::from_utf8_lossy(&output.stdout);
    let id_line = result.lines().find(|line| line.contains("Model name"))
        .unwrap_or("");
    id_line.split(':').nth(1).unwrap_or("").trim().to_string()
}

pub fn get_motherboard_id() -> String {
    let output = Command::new("cat")
        .args(&["/sys/devices/virtual/dmi/id/board_serial"])
        .output()
        .expect("Failed to read motherboard ID");
    let result = String::from_utf8_lossy(&output.stdout);
    result.trim().to_string()
}
