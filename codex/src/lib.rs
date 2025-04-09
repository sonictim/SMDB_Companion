pub mod aif;
pub mod flac;
pub mod mp3;
pub mod resample;
pub mod wav;
use aif::AifCodec;
use anyhow::Result;
use flac::FlacCodec;
use mp3::Mp3Codec;
use std::path::Path;
use wav::WavCodec;

// Standard bit depths
const BIT_DEPTH_8: u16 = 8;
const BIT_DEPTH_16: u16 = 16;
const BIT_DEPTH_24: u16 = 24;
const BIT_DEPTH_32: u16 = 32;

// Sample normalization constants
const U8_OFFSET: f32 = 128.0;
const U8_SCALE: f32 = 127.0;
const I16_MAX_F: f32 = 32767.0;
const I16_DIVISOR: f32 = 32768.0;
const I24_MAX_F: f32 = 8388607.0;
const I24_DIVISOR: f32 = 8388608.0;
const I32_MAX_F: f32 = 2147483647.0;
const I32_DIVISOR: f32 = 2147483648.0;

use memmap2::MmapOptions;

pub fn get_codec(file_path: &str) -> Result<Box<dyn Codec>> {
    let extension = std::path::Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid file extension"))?;

    match extension {
        "wav" => Ok(Box::new(WavCodec)),
        "flac" => Ok(Box::new(FlacCodec)),
        "aif" => Ok(Box::new(AifCodec)),
        "mp3" => Ok(Box::new(Mp3Codec)),
        _ => Err(anyhow::anyhow!(
            "No codec found for extension: {}",
            extension
        )),
    }
}

pub fn encode_to_file(buffer: &AudioBuffer, file_path: &str) -> Result<()> {
    let temp_file = std::env::temp_dir().join("temp_audio_file");

    match get_codec(file_path) {
        Ok(codec) => codec.encode_file(buffer, temp_file.to_str().unwrap_or(""))?,
        Err(error) => return Err(error),
    }

    std::fs::rename(temp_file, file_path)?;
    Ok(())
}

pub fn decode_from_file(file_path: &str) -> Result<AudioBuffer> {
    match get_codec(file_path) {
        Ok(codec) => codec.decode_file(file_path),
        Err(error) => Err(error),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    U8,
    I16,
    I24,
    I32,
    F32,
}

impl SampleFormat {
    pub fn bits_per_sample(&self) -> u16 {
        match self {
            SampleFormat::U8 => 8,
            SampleFormat::I16 => 16,
            SampleFormat::I24 => 24,
            SampleFormat::I32 => 32,
            SampleFormat::F32 => 32,
        }
    }
}

#[derive(Debug)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub format: SampleFormat,
    pub data: Vec<Vec<f32>>, // deinterleaved float audio
}

impl AudioBuffer {
    pub fn resample(&mut self, new_rate: u32) {
        if self.sample_rate != new_rate {
            let resampled_data =
                resample::resample_windowed_sinc(&self.data[0], self.sample_rate, new_rate);
            self.data[0] = resampled_data;
            self.sample_rate = new_rate;
        }
    }
}

pub trait Codec: Send + Sync {
    /// Returns true if this decoder supports the given file signature or extension
    fn validate_file_format(&self, data: &[u8]) -> Result<()>;
    /// Return the file extension this encoder writes (e.g., "wav")
    fn file_extension(&self) -> &'static str;

    /// Encode the audio buffer to the format.
    fn encode(&self, buffer: &AudioBuffer) -> Result<Vec<u8>>;

    fn encode_file(&self, buffer: &AudioBuffer, file_path: &str) -> Result<()> {
        let encoded_data = self.encode(buffer)?;
        std::fs::write(file_path, encoded_data)?;
        Ok(())
    }

    /// Attempts to decode the input bytes.
    fn decode(&self, input: &[u8]) -> Result<AudioBuffer>;

    fn decode_file(&self, file_path: &str) -> Result<AudioBuffer> {
        let file = std::fs::File::open(file_path)?;
        let mapped_file = unsafe { MmapOptions::new().map(&file)? };

        // Use mapped_file as &[u8] without loading into memory
        self.decode(&mapped_file)
    }
}

pub fn detect_simd_support() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::*;
        if is_x86_feature_detected!("sse2") {
            return true;
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        use std::arch::aarch64::*;
        if std::is_aarch64_feature_detected!("neon") {
            return true;
        }
    }
    false
}

pub fn get_str(path: &Path) -> &str {
    path.to_str().unwrap_or("")
}
