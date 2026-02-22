use std::process::Command;

fn main() {
    let output = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .expect("failed to run date");
    let timestamp = String::from_utf8(output.stdout)
        .expect("invalid utf8")
        .trim()
        .to_string();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);
}
