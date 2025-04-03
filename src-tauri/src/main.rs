// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "ffmpeg")]
extern crate ffmpeg_next as ffmpeg;

// Ensure FFmpeg is initialized only once
pub fn init_ffmpeg() -> Result<(), String> {
    static INITIALIZED: std::sync::Once = std::sync::Once::new();
    let mut init_error = None;

    INITIALIZED.call_once(|| {
        if let Err(e) = ffmpeg::init() {
            init_error = Some(format!("FFmpeg initialization error: {}", e));
        }
    });

    match init_error {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

fn main() {
    println!(
        "Current Working Dir: {}",
        std::env::current_dir().unwrap().display()
    );

    smdbc_lib::run()
}
