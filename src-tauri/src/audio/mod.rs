pub mod chromaprint;
pub mod decode;
pub mod encode;
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
        let temp_file = std::env::temp_dir().join("temp_audio_file");
        let temp_path = temp_file.to_str().unwrap_or("");

        match get_encoder(output_file) {
            Ok(codec) => {
                codec.encode_file(self, temp_path)?;
                // codec.embed_file_metadata_chunks(temp_path, &self.metadata)?;
            }
            Err(error) => return Err(error),
        }

        match std::fs::rename(&temp_file, output_file) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
                // If the file already exists, we can just delete the temp file
                std::fs::copy(&temp_file, output_file)?;
                std::fs::remove_file(&temp_file)?;
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}
