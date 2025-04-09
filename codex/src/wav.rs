// codecs/wav.rs
use crate::*;
use anyhow::{Result, anyhow};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use rayon::prelude::*;
use std::io::{Cursor, Read, Write}; // Add Rayon for parallelism
use wide::f32x8; // Only import what we need

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
const HEADER_SIZE: usize = 12; // RIFF + size + WAVE
const MIN_VALID_FILE_SIZE: usize = 12;

//Bit Operations
const I24_SIGN_BIT: i32 = 0x800000;
const I24_SIGN_EXTENSION_MASK: i32 = !0xFFFFFF;
const BYTE_MASK: i32 = 0xFF; // Mask for extracting a single byte

// fn decode_mapped(file_path: &str) -> Result<AudioBuffer> {
//     let file = std::fs::File::open(file_path)?;
//     let mapped_file = unsafe { MmapOptions::new().map(&file)? };

//     // Use mapped_file as &[u8] without loading into memory
//     WavCodec::decode(&mapped_file)
// }

pub struct WavCodec;

impl Codec for WavCodec {
    fn file_extension(&self) -> &'static str {
        "wav"
    }
    fn validate_file_format(&self, data: &[u8]) -> Result<()> {
        // Check for 'RIFF....WAVE' header
        if data.len() >= MIN_VALID_FILE_SIZE
            && &data[0..4] == RIFF_CHUNK_ID
            && &data[8..12] == WAVE_FORMAT_ID
        {
            return Err(anyhow!("Invalid WAV File"));
        }

        if data.len() < HEADER_SIZE {
            return Err(anyhow!("File too small to be a valid WAV"));
        }

        Ok(())
    }

    fn decode(&self, input: &[u8]) -> Result<AudioBuffer> {
        self.validate_file_format(input)?;

        let mut cursor = Cursor::new(input);

        // Step 1: Read RIFF header (total HEADER_SIZE bytes)
        let mut riff = [0u8; 4];
        cursor.read_exact(&mut riff)?;
        if &riff != RIFF_CHUNK_ID {
            return Err(anyhow!("Not a RIFF file"));
        }

        cursor.read_u32::<LittleEndian>()?; // File size
        let mut wave = [0u8; 4];
        cursor.read_exact(&mut wave)?;
        if &wave != WAVE_FORMAT_ID {
            return Err(anyhow!("Not a WAVE file"));
        }

        // Step 2: Find 'fmt ' chunk
        let mut fmt_chunk_found = false;
        let mut data_chunk_found = false;
        let mut sample_format = SampleFormat::I16;
        let mut channels = 0;
        let mut sample_rate = 0;
        let mut bits_per_sample = 0;
        let mut audio_data = vec![];

        while let Ok(chunk_id) = cursor.read_u32::<LittleEndian>() {
            let chunk_id = u32::to_le_bytes(chunk_id);
            let chunk_size = cursor.read_u32::<LittleEndian>()? as usize;
            match &chunk_id {
                FMT_CHUNK_ID => {
                    fmt_chunk_found = true;
                    let format_tag = cursor.read_u16::<LittleEndian>()?;
                    channels = cursor.read_u16::<LittleEndian>()?;
                    sample_rate = cursor.read_u32::<LittleEndian>()?;
                    cursor.read_u32::<LittleEndian>()?; // byte rate
                    cursor.read_u16::<LittleEndian>()?; // block align
                    bits_per_sample = cursor.read_u16::<LittleEndian>()?;

                    sample_format = match (format_tag, bits_per_sample) {
                        (FORMAT_PCM, BIT_DEPTH_8) => SampleFormat::U8,
                        (FORMAT_PCM, BIT_DEPTH_16) => SampleFormat::I16,
                        (FORMAT_PCM, BIT_DEPTH_24) => SampleFormat::I24,
                        (FORMAT_PCM, BIT_DEPTH_32) => SampleFormat::I32,
                        (FORMAT_IEEE_FLOAT, BIT_DEPTH_32) => SampleFormat::F32,
                        _ => {
                            return Err(anyhow!(format!(
                                "Unsupported format: tag {}, bits {}",
                                format_tag, bits_per_sample
                            )));
                        }
                    };

                    if chunk_size > STANDARD_FMT_CHUNK_SIZE as usize {
                        cursor.set_position(
                            cursor.position()
                                + (chunk_size - STANDARD_FMT_CHUNK_SIZE as usize) as u64,
                        );
                    }
                    if chunk_size % 2 != 0 {
                        // Skip padding byte
                        cursor.set_position(cursor.position() + 1);
                    }
                }

                DATA_CHUNK_ID => {
                    data_chunk_found = true;
                    let mut raw_data = vec![0u8; chunk_size];
                    cursor.read_exact(&mut raw_data)?;

                    audio_data = match detect_simd_support() {
                        true => decode_samples_parallel(
                            &raw_data,
                            channels,
                            bits_per_sample,
                            Some(sample_format == SampleFormat::F32),
                        )?,
                        false => {
                            decode_samples_parallel_non_simd(&raw_data, channels, bits_per_sample)?
                        }
                    };

                    if chunk_size % 2 != 0 {
                        // Skip padding byte
                        cursor.set_position(cursor.position() + 1);
                    }
                }

                _ => {
                    cursor.set_position(cursor.position() + chunk_size as u64);

                    if chunk_size % 2 != 0 {
                        // Skip padding byte
                        cursor.set_position(cursor.position() + 1);
                    }
                }
            }
        }

        if !fmt_chunk_found || !data_chunk_found {
            return Err(anyhow!("Missing 'fmt ' or 'data' chunk"));
        }

        Ok(AudioBuffer {
            sample_rate,
            channels,
            format: sample_format,
            data: audio_data,
        })
    }

    fn encode(&self, buffer: &AudioBuffer) -> Result<Vec<u8>> {
        let mut output = Cursor::new(Vec::new());

        // Placeholder for header
        output.write_all(RIFF_CHUNK_ID)?;
        output.write_u32::<LittleEndian>(0)?; // placeholder file size
        output.write_all(WAVE_FORMAT_ID)?;

        // ---- fmt chunk ----
        output.write_all(FMT_CHUNK_ID)?;
        output.write_u32::<LittleEndian>(STANDARD_FMT_CHUNK_SIZE)?; // PCM = 16 bytes
        let (format_tag, bits_per_sample) = match buffer.format {
            SampleFormat::F32 => (FORMAT_IEEE_FLOAT, BIT_DEPTH_32),
            SampleFormat::I16 => (FORMAT_PCM, BIT_DEPTH_16),
            SampleFormat::I24 => (FORMAT_PCM, BIT_DEPTH_24),
            SampleFormat::I32 => (FORMAT_PCM, BIT_DEPTH_32),
            SampleFormat::U8 => (FORMAT_PCM, BIT_DEPTH_8),
        };
        let channels = buffer.channels;
        let sample_rate = buffer.sample_rate;
        let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
        let block_align = channels * bits_per_sample / 8;

        output.write_u16::<LittleEndian>(format_tag)?;
        output.write_u16::<LittleEndian>(channels)?;
        output.write_u32::<LittleEndian>(sample_rate)?;
        output.write_u32::<LittleEndian>(byte_rate)?;
        output.write_u16::<LittleEndian>(block_align)?;
        output.write_u16::<LittleEndian>(bits_per_sample)?;

        // ---- data chunk ----
        output.write_all(DATA_CHUNK_ID)?;
        let data_pos = output.position();
        output.write_u32::<LittleEndian>(0)?; // placeholder

        let start_data = output.position();

        let interleaved_bytes = match detect_simd_support() {
            true => encode_samples_simd(buffer, bits_per_sample)?,
            false => {
                let mut interleaved_bytes = Vec::new();
                encode_samples(&mut interleaved_bytes, buffer, bits_per_sample)?;
                interleaved_bytes
            }
        };

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

// Refactor error handling and normalization factor calculation
pub fn decode_samples_parallel(
    input: &[u8],
    channels: u16,
    bits_per_sample: u16,
    is_float_format: Option<bool>,
) -> Result<Vec<Vec<f32>>> {
    let bytes_per_sample = match bits_per_sample {
        BIT_DEPTH_8 => 1,
        BIT_DEPTH_16 => 2,
        BIT_DEPTH_24 => 3,
        BIT_DEPTH_32 => 4,
        _ => return Err(anyhow!("Unsupported bit depth")),
    };

    let samples_per_channel = input.len() / (channels as usize * bytes_per_sample);
    let simd_chunks = samples_per_channel / 8;

    let output: Vec<Vec<f32>> = (0..channels as usize)
        .into_par_iter() // Parallelize over channels
        .map(|ch| {
            let mut channel_data = vec![0.0; samples_per_channel];

            for i in 0..simd_chunks {
                let mut samples = [0f32; 8];

                for (idx, j) in (0..8).enumerate() {
                    let pos = (i * 8 + j) * channels as usize + ch;
                    let sample_idx = pos * bytes_per_sample;

                    if sample_idx + bytes_per_sample - 1 < input.len() {
                        samples[idx] = decode_sample(
                            &input[sample_idx..sample_idx + bytes_per_sample],
                            bits_per_sample,
                            is_float_format,
                        )?;
                    }
                }

                // Create SIMD vector properly
                let samples_array = [
                    samples[0], samples[1], samples[2], samples[3], samples[4], samples[5],
                    samples[6], samples[7],
                ];
                let simd_samples = f32x8::from(samples_array);
                let start_idx = i * 8;
                // Store the results back into the channel data
                let array = simd_samples.to_array();
                channel_data[start_idx..start_idx + 8].copy_from_slice(&array);
            }

            let remaining_start = simd_chunks * 8;
            for (i, value) in channel_data.iter_mut().enumerate().skip(remaining_start) {
                let pos = i * channels as usize + ch;
                let sample_idx = pos * bytes_per_sample;

                if sample_idx + bytes_per_sample - 1 < input.len() {
                    *value = decode_sample(
                        &input[sample_idx..sample_idx + bytes_per_sample],
                        bits_per_sample,
                        is_float_format,
                    )?;
                }
            }

            Ok(channel_data)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(output)
}

// Abstracted per-sample decoding logic
fn decode_sample(
    sample_bytes: &[u8],
    bits_per_sample: u16,
    is_float_format: Option<bool>,
) -> Result<f32> {
    match bits_per_sample {
        BIT_DEPTH_8 => Ok(sample_bytes[0] as f32 / U8_SCALE - 1.0),
        BIT_DEPTH_16 => {
            let val = i16::from_le_bytes([sample_bytes[0], sample_bytes[1]]);
            Ok(val as f32 / I16_DIVISOR)
        }
        BIT_DEPTH_24 => {
            let val = ((sample_bytes[2] as i32) << 16)
                | ((sample_bytes[1] as i32) << 8)
                | (sample_bytes[0] as i32);
            let val = if val & I24_SIGN_BIT != 0 {
                val | I24_SIGN_EXTENSION_MASK
            } else {
                val
            };
            Ok(val as f32 / I24_DIVISOR)
        }
        BIT_DEPTH_32 => {
            if is_float_format.unwrap_or(false) {
                let bytes = [
                    sample_bytes[0],
                    sample_bytes[1],
                    sample_bytes[2],
                    sample_bytes[3],
                ];
                Ok(f32::from_le_bytes(bytes))
            } else {
                let val = i32::from_le_bytes([
                    sample_bytes[0],
                    sample_bytes[1],
                    sample_bytes[2],
                    sample_bytes[3],
                ]);
                Ok(val as f32 / I32_DIVISOR)
            }
        }
        _ => Err(anyhow!("Unsupported bit depth")),
    }
}

fn decode_samples_parallel_non_simd(
    input: &[u8],
    channels: u16,
    bits_per_sample: u16,
) -> Result<Vec<Vec<f32>>> {
    let bytes_per_sample = match bits_per_sample {
        BIT_DEPTH_8 => 1,
        BIT_DEPTH_16 => 2,
        BIT_DEPTH_24 => 3,
        BIT_DEPTH_32 => 4,
        _ => return Err(anyhow!("Unsupported bit depth")),
    };

    let samples_per_channel = input.len() / (channels as usize * bytes_per_sample);

    let output: Vec<Vec<f32>> = (0..channels as usize)
        .into_par_iter() // Parallelize over channels
        .map(|ch| {
            let mut channel_data = vec![0.0; samples_per_channel];

            #[allow(clippy::needless_range_loop)]
            for i in 0..samples_per_channel {
                let pos = i * channels as usize + ch;
                let sample_idx = pos * bytes_per_sample;

                if sample_idx + bytes_per_sample - 1 < input.len() {
                    let val = match bits_per_sample {
                        BIT_DEPTH_8 => input[sample_idx] as f32 / U8_SCALE - 1.0,
                        BIT_DEPTH_16 => {
                            let val =
                                i16::from_le_bytes([input[sample_idx], input[sample_idx + 1]]);
                            val as f32 / I16_DIVISOR
                        }
                        BIT_DEPTH_24 => {
                            let val = ((input[sample_idx + 2] as i32) << 16)
                                | ((input[sample_idx + 1] as i32) << 8)
                                | (input[sample_idx] as i32);
                            let val = if val & I24_SIGN_BIT != 0 {
                                val | I24_SIGN_EXTENSION_MASK
                            } else {
                                val
                            };
                            val as f32 / I24_DIVISOR
                        }
                        BIT_DEPTH_32 => {
                            let val = i32::from_le_bytes([
                                input[sample_idx],
                                input[sample_idx + 1],
                                input[sample_idx + 2],
                                input[sample_idx + 3],
                            ]);
                            val as f32 / I32_DIVISOR
                        }
                        _ => return vec![],
                    };
                    channel_data[i] = val;
                }
            }

            channel_data
        })
        .collect();

    Ok(output)
}

// fn decode_samples(input: &[u8], channels: u16, bits_per_sample: u16) -> Result<Vec<Vec<f32>>> {
//     let mut reader = Cursor::new(input);
//     let samples_per_channel = input.len() / (channels as usize * (bits_per_sample as usize / 8));
//     let mut output: Vec<Vec<f32>> = (0..channels)
//         .map(|_| Vec::with_capacity(samples_per_channel))
//         .collect();

//     while (reader.position() as usize) < input.len() {
//         for ch in 0..channels {
//             let sample = match bits_per_sample {
//                 BIT_DEPTH_8 => reader.read_u8()? as f32 / U8_SCALE - 1.0,
//                 BIT_DEPTH_16 => reader.read_i16::<LittleEndian>()? as f32 / I16_DIVISOR,
//                 BIT_DEPTH_24 => {
//                     let mut bytes = [0u8; 3];
//                     reader.read_exact(&mut bytes)?;
//                     let val =
//                         ((bytes[2] as i32) << 16) | ((bytes[1] as i32) << 8) | (bytes[0] as i32);
//                     let val = if val & I24_SIGN_BIT != 0 {
//                         val | I24_SIGN_EXTENSION_MASK
//                     } else {
//                         val
//                     };
//                     val as f32 / I24_DIVISOR
//                 }
//                 BIT_DEPTH_32 => reader.read_i32::<LittleEndian>()? as f32 / I32_DIVISOR,
//                 _ => return Err(anyhow!("Unsupported bit depth")),
//             };
//             output[ch as usize].push(sample);
//         }
//     }

//     Ok(output)
// }

// Refactor encoding logic for better error handling and memory allocation
pub fn encode_samples_simd(buffer: &AudioBuffer, bits_per_sample: u16) -> Result<Vec<u8>> {
    let channels = buffer.channels as usize;
    let frames = buffer.data[0].len();
    let mut interleaved_bytes =
        Vec::with_capacity(frames * channels * (bits_per_sample as usize / 8));

    let simd_chunks = frames / 8;

    for ch in 0..channels {
        for i in 0..simd_chunks {
            let start_idx = i * 8;
            let samples_array = [
                buffer.data[ch][start_idx],
                buffer.data[ch][start_idx + 1],
                buffer.data[ch][start_idx + 2],
                buffer.data[ch][start_idx + 3],
                buffer.data[ch][start_idx + 4],
                buffer.data[ch][start_idx + 5],
                buffer.data[ch][start_idx + 6],
                buffer.data[ch][start_idx + 7],
            ];
            let samples_f32 = f32x8::from(samples_array);

            encode_sample_chunk(
                &mut interleaved_bytes,
                samples_f32,
                bits_per_sample,
                buffer.format,
            )?;
        }
    }

    let remaining_start = simd_chunks * 8;
    for i in remaining_start..frames {
        for ch in 0..channels {
            let sample = buffer.data[ch][i];
            encode_sample(
                &mut interleaved_bytes,
                sample,
                bits_per_sample,
                buffer.format,
            )?;
        }
    }

    Ok(interleaved_bytes)
}

// Abstracted per-sample encoding logic
fn encode_sample(
    out: &mut Vec<u8>,
    sample: f32,
    bits_per_sample: u16,
    format: SampleFormat,
) -> Result<()> {
    match bits_per_sample {
        BIT_DEPTH_8 => {
            let val = ((sample * U8_SCALE + U8_OFFSET).clamp(0.0, 255.0)) as u8;
            out.push(val);
        }
        BIT_DEPTH_16 => {
            let val = (sample.clamp(-1.0, 1.0) * I16_MAX_F) as i16;
            out.extend_from_slice(&val.to_le_bytes());
        }
        BIT_DEPTH_24 => {
            let val = (sample.clamp(-1.0, 1.0) * I24_MAX_F) as i32;
            out.push((val & BYTE_MASK) as u8);
            out.push(((val >> 8) & BYTE_MASK) as u8);
            out.push(((val >> 16) & BYTE_MASK) as u8);
        }
        BIT_DEPTH_32 => {
            if format == SampleFormat::F32 {
                out.extend_from_slice(&sample.to_le_bytes());
            } else {
                let val = (sample.clamp(-1.0, 1.0) * I32_MAX_F) as i32;
                out.extend_from_slice(&val.to_le_bytes());
            }
        }
        _ => return Err(anyhow!("Unsupported bit depth")),
    }
    Ok(())
}

// Abstracted per-chunk encoding logic
fn encode_sample_chunk(
    out: &mut Vec<u8>,
    samples_f32: f32x8,
    bits_per_sample: u16,
    format: SampleFormat,
) -> Result<()> {
    let samples_array = samples_f32.to_array();

    match bits_per_sample {
        BIT_DEPTH_8 => {
            for &sample in samples_array.iter() {
                let val = ((sample * U8_SCALE + U8_OFFSET).clamp(0.0, 255.0)) as u8;
                out.push(val);
            }
        }
        BIT_DEPTH_16 => {
            for &sample in samples_array.iter() {
                let val = (sample.clamp(-1.0, 1.0) * I16_MAX_F) as i16;
                out.extend_from_slice(&val.to_le_bytes());
            }
        }
        BIT_DEPTH_24 => {
            for &sample in samples_array.iter() {
                let val = (sample.clamp(-1.0, 1.0) * I24_MAX_F) as i32;
                out.push((val & BYTE_MASK) as u8);
                out.push(((val >> 8) & BYTE_MASK) as u8);
                out.push(((val >> 16) & BYTE_MASK) as u8);
            }
        }
        BIT_DEPTH_32 => {
            if format == SampleFormat::F32 {
                for &sample in samples_array.iter() {
                    out.extend_from_slice(&sample.to_le_bytes());
                }
            } else {
                for &sample in samples_array.iter() {
                    let val = (sample.clamp(-1.0, 1.0) * I32_MAX_F) as i32;
                    out.extend_from_slice(&val.to_le_bytes());
                }
            }
        }
        _ => return Err(anyhow!("Unsupported bit depth")),
    }
    Ok(())
}

fn encode_samples<W: Write>(out: &mut W, buffer: &AudioBuffer, bits_per_sample: u16) -> Result<()> {
    let channels = buffer.channels as usize;
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
                    if buffer.format == SampleFormat::F32 {
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

// Helper function to detect SIMD support for runtime branching
fn detect_simd_support() -> bool {
    // The wide crate automatically uses the best available SIMD instructions
    // This is just for explicit control flow
    true
}
