use crate::*;
use anyhow::{Result, anyhow};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use packed_simd::{f32x8, i32x8};
use rayon::prelude::*;
use std::io::{Cursor, Read, Write};

// FLAC-specific constants
const FLAC_MARKER: &[u8; 4] = b"fLaC";
const STREAMINFO_BLOCK_TYPE: u8 = 0;
const SEEKTABLE_BLOCK_TYPE: u8 = 3;
const VORBIS_COMMENT_BLOCK_TYPE: u8 = 4;
const PICTURE_BLOCK_TYPE: u8 = 6;
const BLOCK_HEADER_SIZE: usize = 4;
const CRC16_POLYNOMIAL: u16 = 0x8005; // CRC-16 polynomial for FLAC validation

// Sample normalization constants (shared with lib.rs)
const U8_OFFSET: f32 = 128.0;
const U8_SCALE: f32 = 127.0;
const I16_MAX_F: f32 = 32767.0;
const I16_DIVISOR: f32 = 32768.0;
const I24_MAX_F: f32 = 8388607.0;
const I24_DIVISOR: f32 = 8388608.0;
const I32_MAX_F: f32 = 2147483647.0;
const I32_DIVISOR: f32 = 2147483648.0;

pub struct FlacCodec;

impl Codec for FlacCodec {
    fn file_extension() -> &'static str {
        "flac"
    }

    fn valid_file_format(data: &[u8]) -> bool {
        // Check for 'fLaC' marker at the beginning of the file
        data.len() >= 4 && &data[0..4] == FLAC_MARKER
    }
}

impl Decoder for FlacCodec {
    fn decode(input: &[u8]) -> Result<AudioBuffer> {
        if input.len() < 4 || &input[0..4] != FLAC_MARKER {
            return Err(anyhow!("Not a valid FLAC file"));
        }

        let mut cursor = Cursor::new(input);
        cursor.set_position(4); // Skip the 'fLaC' marker

        // Parse metadata blocks
        let (sample_rate, channels, bits_per_sample) = parse_metadata_blocks(&mut cursor)?;

        let mut audio_data = vec![];

        // Decode audio frames
        while (cursor.position() as usize) < input.len() {
            let frame_header = cursor.read_u16::<LittleEndian>()?;
            if frame_header & 0xFFF8 != 0xFFF8 {
                return Err(anyhow!("Invalid FLAC frame header"));
            }

            let block_size = (frame_header & 0x7) as usize;
            let mut frame_data = vec![0u8; block_size];
            cursor.read_exact(&mut frame_data)?;

            // Validate CRC
            validate_crc(&frame_data)?;

            let mut decoded_frame = decode_flac_frame_simd(&frame_data, channels, bits_per_sample)?;

            // Handle mid/side stereo channel assignments
            if channels == 2 {
                decode_mid_side(&mut decoded_frame);
            }

            audio_data.extend(decoded_frame);
        }

        Ok(AudioBuffer {
            sample_rate,
            channels,
            format: match bits_per_sample {
                8 => SampleFormat::U8,
                16 => SampleFormat::I16,
                24 => SampleFormat::I24,
                32 => SampleFormat::I32,
                _ => return Err(anyhow!("Unsupported bit depth")),
            },
            data: audio_data,
        })
    }
}

impl Encoder for FlacCodec {
    fn encode(buffer: &AudioBuffer) -> Result<Vec<u8>> {
        let mut output = Cursor::new(Vec::new());

        // Write 'fLaC' marker
        output.write_all(FLAC_MARKER)?;

        // Write metadata blocks
        write_metadata_blocks(
            &mut output,
            buffer.sample_rate,
            buffer.channels,
            buffer.format.bits_per_sample(),
        )?;

        // Write audio frames
        for frame in buffer.data.iter() {
            let mut encoded_frame =
                encode_flac_frame(frame, buffer.channels, buffer.format.bits_per_sample())?;

            // Handle mid/side stereo channel assignments
            if buffer.channels == 2 {
                encode_mid_side(&mut encoded_frame);
            }

            output.write_all(&encoded_frame)?;
        }

        Ok(output.into_inner())
    }
}

// Add support for parsing and writing metadata blocks
fn parse_metadata_blocks(cursor: &mut Cursor<&[u8]>) -> Result<(u32, u16, u16)> {
    let mut sample_rate = 0;
    let mut channels = 0;
    let mut bits_per_sample = 0;

    while let Ok(block_header) = cursor.read_u8() {
        let is_last_block = block_header & 0x80 != 0;
        let block_type = block_header & 0x7F;
        let block_size = cursor.read_u24::<LittleEndian>()? as usize;

        match block_type {
            STREAMINFO_BLOCK_TYPE => {
                // STREAMINFO block
                sample_rate = cursor.read_u24::<LittleEndian>()? >> 4;
                channels = ((cursor.read_u8()? & 0xF) >> 1) + 1;
                bits_per_sample = ((cursor.read_u8()? & 0x1F) >> 1) + 1;
                cursor.set_position(cursor.position() + block_size as u64 - 6);
            }
            SEEKTABLE_BLOCK_TYPE => {
                // SEEKTABLE block (skip for now)
                cursor.set_position(cursor.position() + block_size as u64);
            }
            VORBIS_COMMENT_BLOCK_TYPE => {
                // VORBIS_COMMENT block (skip for now)
                cursor.set_position(cursor.position() + block_size as u64);
            }
            PICTURE_BLOCK_TYPE => {
                // PICTURE block (skip for now)
                cursor.set_position(cursor.position() + block_size as u64);
            }
            _ => {
                cursor.set_position(cursor.position() + block_size as u64);
            }
        }

        if is_last_block {
            break;
        }
    }

    Ok((sample_rate, channels, bits_per_sample))
}

fn write_metadata_blocks(
    output: &mut Cursor<Vec<u8>>,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
) -> Result<()> {
    // Write STREAMINFO block
    output.write_u8(STREAMINFO_BLOCK_TYPE)?; // Block type: STREAMINFO
    output.write_u24::<LittleEndian>(34)?; // Block size
    output.write_u24::<LittleEndian>(sample_rate << 4)?;
    output.write_u8(((channels - 1) << 1) as u8)?;
    output.write_u8(((bits_per_sample - 1) << 1) as u8)?;

    // Placeholder for other metadata blocks (SEEKTABLE, VORBIS_COMMENT, etc.)
    Ok(())
}

fn decode_flac_frame(
    frame_data: &[u8],
    channels: u16,
    bits_per_sample: u16,
) -> Result<Vec<Vec<f32>>> {
    let mut cursor = Cursor::new(frame_data);

    // Parse frame header (simplified for demonstration purposes)
    let block_size = cursor.read_u16::<LittleEndian>()? as usize;
    let mut samples = vec![vec![0.0; block_size]; channels as usize];

    for ch in 0..channels as usize {
        for i in 0..block_size {
            let sample = match bits_per_sample {
                8 => cursor.read_u8()? as f32 / U8_SCALE - 0.5,
                16 => cursor.read_i16::<LittleEndian>()? as f32 / I16_DIVISOR,
                24 => {
                    let mut bytes = [0u8; 3];
                    cursor.read_exact(&mut bytes)?;
                    let val =
                        ((bytes[2] as i32) << 16) | ((bytes[1] as i32) << 8) | (bytes[0] as i32);
                    let val = if val & 0x800000 != 0 {
                        val | !0xFFFFFF
                    } else {
                        val
                    };
                    val as f32 / I24_DIVISOR
                }
                32 => cursor.read_i32::<LittleEndian>()? as f32 / I32_DIVISOR,
                _ => return Err(anyhow!("Unsupported bit depth")),
            };
            samples[ch][i] = sample;
        }
    }

    Ok(samples)
}

fn decode_flac_frame_simd(
    frame_data: &[u8],
    channels: u16,
    bits_per_sample: u16,
) -> Result<Vec<Vec<f32>>> {
    let mut cursor = Cursor::new(frame_data);
    let block_size = cursor.read_u16::<LittleEndian>()? as usize;
    let mut samples = vec![vec![0.0; block_size]; channels as usize];

    for ch in 0..channels as usize {
        for i in (0..block_size).step_by(8) {
            let simd_samples = match bits_per_sample {
                16 => {
                    let mut buffer = [0i16; 8];
                    for j in 0..8 {
                        buffer[j] = cursor.read_i16::<LittleEndian>()?;
                    }
                    i32x8::from_slice_unaligned(&buffer).cast::<f32>() / f32x8::splat(I16_DIVISOR)
                }
                24 => {
                    let mut buffer = [0i32; 8];
                    for j in 0..8 {
                        let mut bytes = [0u8; 3];
                        cursor.read_exact(&mut bytes)?;
                        buffer[j] = ((bytes[2] as i32) << 16)
                            | ((bytes[1] as i32) << 8)
                            | (bytes[0] as i32);
                    }
                    i32x8::from_slice_unaligned(&buffer).cast::<f32>() / f32x8::splat(I24_DIVISOR)
                }
                _ => return Err(anyhow!("Unsupported bit depth for SIMD decoding")),
            };

            simd_samples.store_unaligned(&mut samples[ch][i..i + 8]);
        }
    }

    Ok(samples)
}

fn encode_flac_frame(frame: &[f32], channels: u16, bits_per_sample: u16) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let block_size = frame.len() / channels as usize;

    // Write frame header (simplified for demonstration purposes)
    output.write_u16::<LittleEndian>(block_size as u16)?;

    for ch in 0..channels as usize {
        for i in 0..block_size {
            let sample = frame[ch * block_size + i];
            match bits_per_sample {
                8 => {
                    let val = ((sample + 0.5) * U8_SCALE).clamp(0.0, 255.0) as u8;
                    output.write_u8(val)?;
                }
                16 => {
                    let val = (sample * I16_DIVISOR).clamp(-I16_MAX_F, I16_MAX_F) as i16;
                    output.write_i16::<LittleEndian>(val)?;
                }
                24 => {
                    let val = (sample * I24_DIVISOR).clamp(-I24_MAX_F, I24_MAX_F) as i32;
                    output.write_all(&[
                        (val & 0xFF) as u8,
                        ((val >> 8) & 0xFF) as u8,
                        ((val >> 16) & 0xFF) as u8,
                    ])?;
                }
                32 => {
                    let val = (sample * I32_DIVISOR).clamp(-I32_MAX_F, I32_MAX_F) as i32;
                    output.write_i32::<LittleEndian>(val)?;
                }
                _ => return Err(anyhow!("Unsupported bit depth")),
            }
        }
    }

    Ok(output)
}

// Add CRC validation for FLAC frames
fn validate_crc(frame_data: &[u8]) -> Result<()> {
    let expected_crc = frame_data[frame_data.len() - 2..].read_u16::<LittleEndian>()?;
    let calculated_crc =
        crc16::State::<crc16::XMODEM>::calculate(&frame_data[..frame_data.len() - 2]);

    if expected_crc != calculated_crc {
        return Err(anyhow!(
            "CRC mismatch: expected {}, calculated {}",
            expected_crc,
            calculated_crc
        ));
    }

    Ok(())
}

// Add support for mid/side stereo channel assignments
fn decode_mid_side(samples: &mut [Vec<f32>]) {
    for i in 0..samples[0].len() {
        let mid = samples[0][i];
        let side = samples[1][i];
        samples[0][i] = mid + side;
        samples[1][i] = mid - side;
    }
}

fn encode_mid_side(samples: &mut [Vec<f32>]) {
    for i in 0..samples[0].len() {
        let left = samples[0][i];
        let right = samples[1][i];
        samples[0][i] = (left + right) / 2.0;
        samples[1][i] = (left - right) / 2.0;
    }
}

// Add support for seeking within FLAC files
fn seek_to_sample(cursor: &mut Cursor<&[u8]>, seek_table: &[u8], target_sample: u64) -> Result<()> {
    for entry in seek_table.chunks_exact(18) {
        let sample_number = u64::from_be_bytes(entry[0..8].try_into()?);
        let offset = u64::from_be_bytes(entry[8..16].try_into()?);

        if sample_number >= target_sample {
            cursor.set_position(offset);
            return Ok(());
        }
    }

    Err(anyhow!("Target sample not found in SEEKTABLE"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flac_decode_valid_file() {
        let flac_data = include_bytes!("../test_data/valid.flac");
        let result = FlacCodec::decode(flac_data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_flac_decode_invalid_file() {
        let invalid_data = b"NotAFlacFile";
        let result = FlacCodec::decode(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_flac_encode_decode_roundtrip() {
        let buffer = AudioBuffer {
            sample_rate: 44100,
            channels: 2,
            format: SampleFormat::I16,
            data: vec![vec![0.0; 44100], vec![0.0; 44100]],
        };

        let encoded = FlacCodec::encode(&buffer).unwrap();
        let decoded = FlacCodec::decode(&encoded).unwrap();

        assert_eq!(buffer.sample_rate, decoded.sample_rate);
        assert_eq!(buffer.channels, decoded.channels);
        assert_eq!(buffer.format, decoded.format);
    }
}
