pub mod flac;
pub mod resample;
pub mod wav;
use anyhow::Result;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    U8,
    I16,
    I24,
    I32,
    F32,
}

#[derive(Debug)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub format: SampleFormat,
    pub data: Vec<Vec<f32>>, // deinterleaved float audio
}

pub trait Codec {
    /// Returns true if this decoder supports the given file signature or extension
    fn valid_file_format(data: &[u8]) -> bool;
    /// Return the file extension this encoder writes (e.g., "wav")
    fn file_extension() -> &'static str;
}

pub trait Encoder {
    /// Encode the audio buffer to the format.
    fn encode(buffer: &AudioBuffer) -> Result<Vec<u8>>;
}

pub trait Decoder {
    /// Attempts to decode the input bytes.
    fn decode(input: &[u8]) -> Result<AudioBuffer>;
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
