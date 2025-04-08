pub use resample;
pub use wav;

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
    /// Attempts to decode the input bytes.
    fn decode(input: &[u8]) -> Result<AudioBuffer, String>;

    /// Encode the audio buffer to the format.
    fn encode(buffer: &AudioBuffer) -> Result<Vec<u8>, String>;

    /// Returns true if this decoder supports the given file signature or extension
    fn supports_format(data: &[u8]) -> bool;
    /// Return the file extension this encoder writes (e.g., "wav")
    fn file_extension() -> &'static str;
}
