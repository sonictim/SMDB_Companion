use std::env;
use std::process::Command;
use std::fs;
use std::io;

fn main() {

    let name = env!("CARGO_PKG_NAME");
    // let name = "SMDB_Companion";
    let x86_64_dir = "target/x86_64-apple-darwin/release";
    let aarch64_dir = "target/aarch64-apple-darwin/release";
    let target_dir = "target/universal/release";

    // Only run for full builds, skip during `cargo check`
    if std::env::var("DEBUG").unwrap_or_else(|_| "false".to_string()) == "true" {
        println!("Debug mode detected");
    } else {

            // Compile for x86_64
        Command::new("cargo")
            .args(&["bundle", "--release", "--target", "x86_64-apple-darwin"])
            .status()
            .expect("Failed to build for x86_64");
        
            // Compile for aarch64
        Command::new("cargo")
            .args(&["bundle", "--release", "--target", "aarch64-apple-darwin"])
            .status()
            .expect("Failed to build for aarch64");
        
    }


        // Check path for universal binary exists 
    if fs::metadata(target_dir).is_err() {
        fs::create_dir_all(target_dir).ok();}

        
        // Create a universal binary app shell
    Command::new("cp")
        .args(&[
            "-R", 
            &format!("{}/bundle/osx/{}.app", aarch64_dir, name), 
            &format!("{}", target_dir),
                ])
        .status()
        .expect("Failed to create universal binary");


    // Create a universal binary
    Command::new("lipo")
        .args(&["-create", "-output", &format!("{}/{}.app/Contents/MacOS/{}", target_dir, name, name), 
                &format!("{}/{}", x86_64_dir, name),
                &format!("{}/{}", aarch64_dir, name)])
        .status()
        .expect("Failed to create universal binary");


    Command::new("cp")
        .args(&[
            "-R", 
            &format!("{}/{}.app", target_dir, name), 
            &format!(r"/Users/tfarrell/Library/CloudStorage/GoogleDrive-tim@farrellsound.com/Shared\ drives/PUBLIC/{}/", name),
                ])
        .status()
        .expect("Failed to create universal binary");



    

}


