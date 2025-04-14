use crate::prelude::*;
pub use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use flacenc::component::BitRepr;
use flacenc::error::Verify;
pub use memmap2::MmapOptions;
pub use rayon::prelude::*;
pub use std::io::{Cursor, Read, Seek, SeekFrom, Write};

// Sample normalization constants
const I16_MAX_F: f32 = 32767.0;
const I24_MAX_F: f32 = 8388607.0;
const I32_MAX_F: f32 = 2147483647.0;

// Standard bit depths
const BIT_DEPTH_8: u16 = 8;
const BIT_DEPTH_16: u16 = 16;
const BIT_DEPTH_24: u16 = 24;
const BIT_DEPTH_32: u16 = 32;

// Sample normalization constants
const U8_OFFSET: f32 = 128.0;
const U8_SCALE: f32 = 127.0;

//Bit Operations
const BYTE_MASK: i32 = 0xFF; // Mask for extracting a single byte

// Format tags
const FORMAT_PCM: u16 = 1;
const FORMAT_IEEE_FLOAT: u16 = 3;

// Chunk Identifiers
const RIFF_CHUNK_ID: &[u8; 4] = b"RIFF";
const WAVE_FORMAT_ID: &[u8; 4] = b"WAVE";
const FMT_CHUNK_ID: &[u8; 4] = b"fmt ";
const DATA_CHUNK_ID: &[u8; 4] = b"data";

// Chunk Structures
const STANDARD_FMT_CHUNK_SIZE: u32 = 16;

pub fn get_encoder(file_path: &str) -> Result<Box<dyn Encoder>> {
    let extension = std::path::Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid file extension"))?;

    match extension {
        "wav" => Ok(Box::new(WavCodec)),
        "flac" => Ok(Box::new(FlacCodec)),
        // "aif" => Ok(Box::new(AifCodec)),
        // "mp3" => Ok(Box::new(Mp3Codec)),
        _ => Err(anyhow::anyhow!(
            "No Encoder found for extension: {}",
            extension
        )),
    }
}

pub trait Encoder: Send + Sync {
    fn encode(&self, buffer: &AudioBuffer) -> Result<Vec<u8>>;

    fn encode_file(&self, buffer: &AudioBuffer, file_path: &str) -> Result<()> {
        let encoded_data = self.encode(buffer)?;
        std::fs::write(file_path, encoded_data)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleFormat {
    U8,
    I16,
    I24,
    I32,
    #[default]
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

pub struct WavCodec;
impl Encoder for WavCodec {
    fn encode(&self, buffer: &AudioBuffer) -> Result<Vec<u8>> {
        let mut output = Cursor::new(Vec::new());

        // Ensure channel count in buffer is consistent with data
        let actual_channels = buffer.data.len() as u16;
        let channels = if actual_channels != buffer.channels {
            actual_channels
        } else {
            buffer.channels
        };

        // Placeholder for header
        output.write_all(RIFF_CHUNK_ID)?;
        output.write_u32::<LittleEndian>(0)?; // placeholder file size
        output.write_all(WAVE_FORMAT_ID)?;

        // ---- fmt chunk ----
        output.write_all(FMT_CHUNK_ID)?;
        output.write_u32::<LittleEndian>(STANDARD_FMT_CHUNK_SIZE)?; // PCM = 16 bytes
        let (format_tag, bits_per_sample) = match buffer.sample_format {
            SampleFormat::F32 => (FORMAT_IEEE_FLOAT, BIT_DEPTH_32),
            SampleFormat::I16 => (FORMAT_PCM, BIT_DEPTH_16),
            SampleFormat::I24 => (FORMAT_PCM, BIT_DEPTH_24),
            SampleFormat::I32 => (FORMAT_PCM, BIT_DEPTH_32),
            SampleFormat::U8 => (FORMAT_PCM, BIT_DEPTH_8),
        };
        let sample_rate = buffer.sample_rate;
        let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
        let block_align = channels * bits_per_sample / 8;

        output.write_u16::<LittleEndian>(format_tag)?;
        output.write_u16::<LittleEndian>(channels)?; // Use the verified channel count
        output.write_u32::<LittleEndian>(sample_rate)?;
        output.write_u32::<LittleEndian>(byte_rate)?;
        output.write_u16::<LittleEndian>(block_align)?;
        output.write_u16::<LittleEndian>(bits_per_sample)?;

        // ---- data chunk ----
        output.write_all(DATA_CHUNK_ID)?;
        let data_pos = output.position();
        output.write_u32::<LittleEndian>(0)?; // placeholder

        let start_data = output.position();

        let mut interleaved_bytes = Vec::new();
        encode_samples(&mut interleaved_bytes, buffer, bits_per_sample)?;

        output.write_all(&interleaved_bytes)?;

        let end_data = output.position();
        let data_size = (end_data - start_data) as u32;

        // Fill in data chunk size
        let mut out = output.into_inner();
        (&mut out[(data_pos as usize)..(data_pos as usize + 4)])
            .write_u32::<LittleEndian>(data_size)?;

        // Fill in RIFF file size
        let riff_size = out.len() as u32 - 8;
        (&mut out[4..8]).write_u32::<LittleEndian>(riff_size)?;

        Ok(out)
    }
}

pub struct FlacCodec;
impl Encoder for FlacCodec {
    fn encode(&self, buffer: &AudioBuffer) -> Result<Vec<u8>> {
        // Get audio parameters
        let bits_per_sample = get_bits_per_sample(buffer.sample_format);
        let channels = buffer.channels as usize;
        let sample_rate = buffer.sample_rate as usize;

        if buffer.data.is_empty() || buffer.data[0].is_empty() {
            return Err(anyhow!("Cannot encode empty audio buffer"));
        }

        let num_samples = buffer.data[0].len();

        // Pre-calculate conversion factors outside of the loop for better performance
        let scale_factor = match bits_per_sample {
            8 => 127.0,
            16 => I16_MAX_F,
            24 => I24_MAX_F,
            32 => I32_MAX_F,
            _ => {
                return Err(anyhow!(
                    "Unsupported bit depth for FLAC encoding: {}",
                    bits_per_sample
                ));
            }
        };

        // Create the interleaved samples vector using either parallel or sequential approach
        let interleaved_samples = if num_samples > 100_000 {
            // For large files, use parallel processing with thread-local data
            let chunk_size = (num_samples / rayon::current_num_threads()).max(1024);

            // Use parallel iterator with collect to build the final vector
            (0..num_samples)
                .into_par_iter()
                .chunks(chunk_size)
                .flat_map(|chunk_indices| {
                    // Create a local buffer for each thread
                    let mut local_buffer = Vec::with_capacity(chunk_indices.len() * channels);

                    // Process samples in this chunk
                    for i in chunk_indices {
                        for ch in 0..channels {
                            let sample = buffer.data[ch][i];
                            let val = (sample * scale_factor).round() as i32;
                            local_buffer.push(val);
                        }
                    }

                    local_buffer
                })
                .collect()
        } else {
            // For smaller files, use a straightforward sequential approach
            // which avoids overhead of parallelism for small datasets
            let mut samples = Vec::with_capacity(num_samples * channels);
            for i in 0..num_samples {
                for ch in 0..channels {
                    let sample = buffer.data[ch][i];
                    let val = (sample * scale_factor).round() as i32;
                    samples.push(val);
                }
            }
            samples
        };

        // Configure the encoder with optimized settings
        let mut config = flacenc::config::Encoder::default();

        // Set larger block size for better throughput and compression
        config.block_size = 8192;

        // Create a verified config
        let config = config
            .into_verified()
            .map_err(|e| anyhow!("Invalid FLAC encoder configuration: {:?}", e))?;

        // Create a source from the interleaved samples
        let source = flacenc::source::MemSource::from_samples(
            &interleaved_samples,
            channels,
            bits_per_sample as usize,
            sample_rate,
        );

        // Use a fixed block size for consistent performance
        let flac_stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
            .map_err(|e| anyhow!("FLAC encoding error: {:?}", e))?;

        // Estimate final buffer size (typically FLAC is ~50-60% of raw PCM)
        let estimated_size = (num_samples * channels * (bits_per_sample as usize / 8) / 2) + 8192;

        // Create a byte sink with sufficient capacity
        let mut sink = flacenc::bitsink::ByteSink::new();
        sink.reserve(estimated_size);

        // Write the encoded stream
        flac_stream.write(&mut sink)?;

        // Return the encoded FLAC data
        Ok(sink.as_slice().to_vec())
    }
}

fn encode_samples<W: Write>(out: &mut W, buffer: &AudioBuffer, bits_per_sample: u16) -> Result<()> {
    // Ensure channel count doesn't exceed available data channels
    let available_channels = buffer.data.len();
    let channels = std::cmp::min(buffer.channels as usize, available_channels);

    // Ensure consistent channel count between metadata and actual data
    let frames = buffer.data[0].len();

    for i in 0..frames {
        for ch in 0..channels {
            let sample = buffer.data[ch][i];
            match bits_per_sample {
                BIT_DEPTH_8 => {
                    let val = ((sample * U8_SCALE + U8_OFFSET).clamp(0.0, 255.0)) as u8;
                    out.write_u8(val)?;
                }
                BIT_DEPTH_16 => {
                    let val = (sample.clamp(-1.0, 1.0) * I16_MAX_F) as i16;
                    out.write_i16::<LittleEndian>(val)?;
                }
                BIT_DEPTH_24 => {
                    let val = (sample.clamp(-1.0, 1.0) * I24_MAX_F) as i32;
                    let bytes = [
                        (val & BYTE_MASK) as u8,
                        ((val >> 8) & BYTE_MASK) as u8,
                        ((val >> 16) & BYTE_MASK) as u8,
                    ];
                    out.write_all(&bytes)?;
                }
                BIT_DEPTH_32 => {
                    if buffer.sample_format == SampleFormat::F32 {
                        out.write_f32::<LittleEndian>(sample)?;
                    } else {
                        let val = (sample.clamp(-1.0, 1.0) * I32_MAX_F) as i32;
                        out.write_i32::<LittleEndian>(val)?;
                    }
                }
                _ => return Err(anyhow!("Unsupported bit depth")),
            }
        }
    }

    Ok(())
}

// Helper function to get bits per sample from SampleFormat
fn get_bits_per_sample(format: SampleFormat) -> u16 {
    match format {
        SampleFormat::U8 => 8,
        SampleFormat::I16 => 16,
        SampleFormat::I24 => 24,
        SampleFormat::I32 | SampleFormat::F32 => 32,
    }
}
