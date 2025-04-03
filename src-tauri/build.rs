use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Install required system dependency
    if let Ok(output) = Command::new("brew").args(&["list", "jpeg-xl"]).output() {
        if !output.status.success() {
            println!("cargo:warning=Installing jpeg-xl...");
            let status = Command::new("brew")
                .args(&["install", "jpeg-xl"])
                .status()
                .expect("Failed to install jpeg-xl");

            if !status.success() {
                panic!("Failed to install jpeg-xl");
            }
        }
    }

    unsafe {
        // Explicitly set all these environment variables inside build.rs too
        env::set_var("FFMPEG_PKG_CONFIG_PATH", "NONE");
        env::set_var("LIBAVCODEC_NO_PKG_CONFIG", "1");
        env::set_var("LIBAVDEVICE_NO_PKG_CONFIG", "1");
        env::set_var("LIBAVFILTER_NO_PKG_CONFIG", "1");
        env::set_var("LIBAVFORMAT_NO_PKG_CONFIG", "1");
        env::set_var("LIBAVRESAMPLE_NO_PKG_CONFIG", "1");
        env::set_var("LIBAVUTIL_NO_PKG_CONFIG", "1");
        env::set_var("LIBPOSTPROC_NO_PKG_CONFIG", "1");
        env::set_var("LIBSWRESAMPLE_NO_PKG_CONFIG", "1");
        env::set_var("LIBSWSCALE_NO_PKG_CONFIG", "1");
    }
    // Get absolute paths
    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let ffmpeg_dir = project_dir.join("resources/ffmpeg/universal");
    let ffmpeg_lib_dir = ffmpeg_dir.join("lib");
    let ffmpeg_include_dir = ffmpeg_dir.join("include");
    unsafe {
        // Set include directory
        env::set_var("FFMPEG_INCLUDE_DIR", ffmpeg_include_dir.to_str().unwrap());
    }
    // Make sure the directories exist
    if !ffmpeg_lib_dir.exists() {
        panic!(
            "FFmpeg libraries directory not found: {}",
            ffmpeg_lib_dir.display()
        );
    }

    if !ffmpeg_include_dir.exists() {
        panic!(
            "FFmpeg include directory not found: {}",
            ffmpeg_include_dir.display()
        );
    }

    // Link against chromaprint
    println!(
        "cargo:rustc-link-search=native={}",
        project_dir.join("resources").display()
    );
    println!("cargo:rustc-link-lib=static=chromaprint");

    // Link against C++ standard library
    println!("cargo:rustc-link-lib=dylib=c++");

    // Link against Accelerate framework
    println!("cargo:rustc-link-lib=framework=Accelerate");

    // Link against FFmpeg libraries
    println!(
        "cargo:rustc-link-search=native={}",
        ffmpeg_lib_dir.display()
    );
    println!("cargo:rustc-link-lib=static=avcodec");
    println!("cargo:rustc-link-lib=static=avformat");
    println!("cargo:rustc-link-lib=static=avutil");
    println!("cargo:rustc-link-lib=static=swresample");
    println!("cargo:rustc-link-lib=static=swscale");

    // Tell cargo to invalidate the built crate whenever any of these change
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=resources/libchromaprint.a");
    println!("cargo:rerun-if-changed={}", ffmpeg_lib_dir.display());
    println!("cargo:rerun-if-changed={}", ffmpeg_include_dir.display());

    tauri_build::build()
}
