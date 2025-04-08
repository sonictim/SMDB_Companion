pub mod resample;
pub mod wav;

use anyhow::Result;

#[derive(Debug, Clone, Copy)]
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
