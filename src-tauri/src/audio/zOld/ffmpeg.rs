use crate::prelude::*;
// use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn cleanup_multi_mono(path: &Path) -> Result<()> {
    // Get the architecture-specific FFmpeg binary
    let ffmpeg_path = get_bundled_ffmpeg();

    // Convert the file using the external FFmpeg binary
    convert_to_first_channel_mono(path, ffmpeg_path)
}

/// Converts any audio file to mono (keeping only the first channel) while preserving metadata
pub fn convert_to_first_channel_mono(
    input_path: &Path,
    ffmpeg_path: Option<PathBuf>,
) -> Result<()> {
    // Verify the input file exists
    if !input_path.exists() {
        return Err(anyhow::anyhow!(
            "Input file does not exist: {:?}",
            input_path
        ));
    }

    // Create temp path with proper extension
    let temp_path = get_temp_file_path(input_path);

    // Determine FFmpeg path
    let ffmpeg = match ffmpeg_path {
        Some(path) => path,
        None => PathBuf::from("ffmpeg"), // Use from PATH
    };

    println!("Using FFmpeg at: {:?}", ffmpeg);
    println!("Temp file path: {:?}", temp_path);

    // Build and run the FFmpeg command with fixed path to temp file
    let status = Command::new(&ffmpeg)
        .arg("-i")
        .arg(input_path)
        .arg("-map")
        .arg("0:a")
        .arg("-filter:a")
        .arg("pan=mono|c0=c0")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg("-metadata:s:a:0")
        .arg("channel_layout=mono")
        .arg("-c:v")
        .arg("copy")
        .arg("-map_metadata")
        .arg("0")
        .arg("-id3v2_version")
        .arg("3")
        .arg("-write_id3v1")
        .arg("1")
        .arg("-y")
        .arg(&temp_path) // Fixed: Use temp_path instead of temp_file.path()
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("FFmpeg failed with exit code: {}", status));
    }

    // Replace the original file with the temporary file
    std::fs::rename(&temp_path, input_path)?;

    Ok(())
}

fn get_temp_file_path(input_path: &Path) -> PathBuf {
    let parent = input_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = input_path
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("temp");
    let ext = input_path
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("wav");

    parent.join(format!("{}-temp.{}", stem, ext))
}

// Utility function to get the correct architecture-specific FFmpeg binary
#[cfg(target_os = "macos")]
pub fn get_bundled_ffmpeg() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let bundle = exe.parent()?.parent()?.parent()?;
    let resources = bundle.join("Resources");

    // Detect architecture
    let arch = std::env::consts::ARCH;
    let ffmpeg_path = match arch {
        "aarch64" => resources.join("ffmpeg").join("arm64").join("ffmpeg"),
        "x86_64" => resources.join("ffmpeg").join("x86_64").join("ffmpeg"),
        _ => {
            println!("Unsupported architecture: {}", arch);
            return None;
        }
    };

    if ffmpeg_path.exists() {
        println!("Found bundled FFmpeg for {} at: {:?}", arch, ffmpeg_path);
        Some(ffmpeg_path)
    } else {
        println!("FFmpeg not found at expected path: {:?}", ffmpeg_path);
        None
    }
}

#[cfg(target_os = "windows")]
pub fn get_bundled_ffmpeg() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let ffmpeg = dir.join("ffmpeg.exe");

    if ffmpeg.exists() { Some(ffmpeg) } else { None }
}

#[cfg(target_os = "linux")]
pub fn get_bundled_ffmpeg() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let ffmpeg = dir.join("ffmpeg");

    if ffmpeg.exists() { Some(ffmpeg) } else { None }
}

pub fn pcm_to_wav(
    pcm_data: &[u8],
    output_path: &Path,
    sample_rate: u32,
    channels: u32,
    format: &str,
    ffmpeg_path: Option<PathBuf>,
) -> Result<()> {
    // Create a temporary file for the raw PCM data
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path().join("temp.pcm");

    // Write PCM data to the temporary file
    let mut temp_file = File::create(&temp_path)?;
    temp_file.write_all(pcm_data)?;
    temp_file.flush()?;

    // Determine FFmpeg path
    let ffmpeg = match ffmpeg_path {
        Some(path) => path,
        None => PathBuf::from("ffmpeg"), // Use from PATH
    };

    // Build and run the FFmpeg command
    let status = Command::new(ffmpeg)
        .arg("-f")
        .arg(format)
        .arg("-ar")
        .arg(sample_rate.to_string())
        .arg("-ac")
        .arg(channels.to_string())
        .arg("-i")
        .arg(&temp_path)
        .arg("-y") // Overwrite output file if it exists
        .arg(output_path)
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("FFmpeg exited with status: {}", status));
    }

    Ok(())
}

// For convenience, provide a function to check if FFmpeg is available
pub fn check_ffmpeg(ffmpeg_path: Option<PathBuf>) -> bool {
    let ffmpeg = match ffmpeg_path {
        Some(path) => path,
        None => PathBuf::from("ffmpeg"),
    };

    Command::new(&ffmpeg)
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
