pub mod chromaprint;
pub mod decode;
pub mod encode;
pub mod metadata;
// pub mod ffmpeg;
pub mod playback;
// pub mod shazam_fingerprint_processing;
// pub mod landmarks;
// pub mod claude;
// pub mod encode;
// pub mod decode_old;
// pub mod fft;
// pub mod shazam_style;
// pub mod shazam_search_claude;
// pub mod symphonia_play;

pub use chromaprint::*;
pub use decode::*;
pub use encode::*;
// pub use ffmpeg::*;
pub use metadata::*;
pub use playback::*;
// pub use shazam_fingerprint_processing::*;
// pub use landmarks::*;
// pub use claude::*;
// pub use encode::*;
// pub use fft::*;
// pub use shazam_style::*;
// pub use shazam_search_claude::*;
// pub use symphonia_play::*;

pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: SampleFormat,
    pub data: Vec<Vec<f32>>, // deinterleaved float audio
}

impl AudioBuffer {
    pub fn strip_multi_mono(&mut self) -> Result<()> {
        if self.data.is_empty() || self.channels < 2 {
            return Ok(());
        }

        let first_channel = std::mem::take(&mut self.data[0]);
        self.data.clear();
        self.data.push(first_channel);

        self.channels = 1;

        Ok(())
    }

    pub fn export(&self, output_file: &str) -> Result<()> {
        // Create a Path object for the output file
        let output_path = Path::new(output_file);

        // Extract the parent directory, file stem, and extension
        let parent_dir = output_path.parent().unwrap_or(Path::new("."));

        // Check if parent directory exists
        if !parent_dir.exists() {
            return Err(anyhow::anyhow!(
                "Parent directory doesn't exist: {}",
                parent_dir.display()
            ));
        }

        // Verify write permissions to parent directory by testing with a marker file
        let test_file_path = parent_dir.join(".write_test_delete_me");
        match std::fs::File::create(&test_file_path) {
            Ok(_) => {
                // Successfully created test file, clean it up
                let _ = std::fs::remove_file(&test_file_path);
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Cannot write to directory {}: {}",
                    parent_dir.display(),
                    e
                ));
            }
        }

        let file_stem = output_path
            .file_stem()
            .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?
            .to_string_lossy();

        let extension = output_path
            .extension()
            .map(|ext| ext.to_string_lossy())
            .unwrap_or_default();

        // Create the temp filename: [filename].temp.[extension]
        let temp_filename = if extension.is_empty() {
            format!("{}.temp", file_stem)
        } else {
            format!("{}.temp.{}", file_stem, extension)
        };

        // Create the full temp path in the same directory as the output file
        let temp_file = parent_dir.join(temp_filename);
        let temp_path = temp_file
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid temp path"))?;

        println!("Creating temp file at: {}", temp_path);

        // Check if output file exists and verify we can replace it
        if output_path.exists() {
            match std::fs::metadata(output_file) {
                Ok(metadata) => {
                    let permissions = metadata.permissions();
                    if permissions.readonly() {
                        return Err(anyhow::anyhow!("Output file is read-only: {}", output_file));
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Cannot access output file metadata: {}", e));
                }
            }
        }

        // Get encoder and encode directly to the temp file
        match get_encoder(output_file) {
            Ok(codec) => {
                codec.encode_file(self, temp_path)?;
                // codec.embed_file_metadata_chunks(temp_path, &self.metadata)?;
            }
            Err(error) => return Err(error),
        }

        // Rename the temp file to the output file
        // This should work since both files are on the same filesystem
        match std::fs::rename(&temp_file, output_file) {
            Ok(_) => {
                println!("Successfully renamed temp file to: {}", output_file);
                Ok(())
            }
            Err(e) => {
                // If rename fails, try to analyze and provide a helpful error
                let error_message = match e.kind() {
                    std::io::ErrorKind::PermissionDenied => {
                        format!("Permission denied when renaming to {}", output_file)
                    }
                    std::io::ErrorKind::NotFound => format!(
                        "Temporary file disappeared during rename: {}",
                        temp_file.display()
                    ),
                    std::io::ErrorKind::CrossesDevices => "Cannot rename across different volumes - this shouldn't happen with our approach".to_string(),
                    _ => format!("Error renaming temp file: {}", e),
                };

                println!("{}", error_message);

                // As a fallback, try to copy then delete
                println!("Attempting copy+delete as fallback...");
                if let Err(copy_err) = std::fs::copy(&temp_file, output_file) {
                    println!("Copy failed: {}", copy_err);
                    Err(e.into()) // Return the original error
                } else {
                    let _ = std::fs::remove_file(&temp_file); // Try to cleanup
                    println!("Copy+delete successful");
                    Ok(())
                }
            }
        }
    }
}
