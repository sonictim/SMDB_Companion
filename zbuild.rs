use std::env;
use std::process::Command;

fn main() {
    let target_dir = env::var("OUT_DIR").unwrap();
    let output_dir = format!("{}/../../../../../target", target_dir);

    // Compile for x86_64
    Command::new("cargo")
        .args(&["build", "--target", "x86_64-apple-darwin", "--release"])
        .current_dir(env::var("CARGO_MANIFEST_DIR").unwrap())
        .status()
        .expect("Failed to build x86_64 target");

    // Compile for aarch64
    Command::new("cargo")
        .args(&["build", "--target", "aarch64-apple-darwin", "--release"])
        .current_dir(env::var("CARGO_MANIFEST_DIR").unwrap())
        .status()
        .expect("Failed to build aarch64 target");

    // Create a universal binary
    Command::new("lipo")
        .args(&["-create", "-output", &format!("{}/SMDB_Companion", output_dir), 
                &format!("{}/x86_64-apple-darwin/release/SMDB_Companion", output_dir),
                &format!("{}/aarch64-apple-darwin/release/SMDB_Companion", output_dir)])
        .status()
        .expect("Failed to create universal binary");
}
