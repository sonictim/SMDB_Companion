// codecs/wav.rs

use super::types::*;
use crate::codecs::Codec;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

pub struct WavCodec;

impl Codec for WavCodec {
    fn file_extension() -> &'static str {
        "wav"
    }
    fn supports_format(data: &[u8]) -> bool {
        // Check for 'RIFF....WAVE' header
        data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE"
    }

    fn decode(input: &[u8]) -> Result<AudioBuffer, String> {
        let mut cursor = Cursor::new(input);

        // Step 1: Read RIFF header
        let mut riff = [0u8; 4];
        cursor.read_exact(&mut riff)?;
        if &riff != b"RIFF" {
            return Err("Not a RIFF file".into());
        }

        cursor.read_u32::<LittleEndian>()?; // File size
        let mut wave = [0u8; 4];
        cursor.read_exact(&mut wave)?;
        if &wave != b"WAVE" {
            return Err("Not a WAVE file".into());
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
                b"fmt " => {
                    fmt_chunk_found = true;
                    let format_tag = cursor.read_u16::<LittleEndian>()?;
                    channels = cursor.read_u16::<LittleEndian>()?;
                    sample_rate = cursor.read_u32::<LittleEndian>()?;
                    cursor.read_u32::<LittleEndian>()?; // byte rate
                    cursor.read_u16::<LittleEndian>()?; // block align
                    bits_per_sample = cursor.read_u16::<LittleEndian>()?;

                    sample_format = match (format_tag, bits_per_sample) {
                        (1, 8) => SampleFormat::U8,
                        (1, 16) => SampleFormat::I16,
                        (1, 24) => SampleFormat::I24,
                        (1, 32) => SampleFormat::I32,
                        (3, 32) => SampleFormat::F32,
                        _ => {
                            return Err(format!(
                                "Unsupported format: tag {}, bits {}",
                                format_tag, bits_per_sample
                            ));
                        }
                    };

                    if chunk_size > 16 {
                        cursor.set_position(cursor.position() + (chunk_size - 16) as u64);
                    }
                }

                b"data" => {
                    data_chunk_found = true;
                    let mut raw_data = vec![0u8; chunk_size];
                    cursor.read_exact(&mut raw_data)?;

                    audio_data = decode_samples(&raw_data, channels, bits_per_sample)?;
                }

                _ => {
                    cursor.set_position(cursor.position() + chunk_size as u64);
                }
            }
        }

        if !fmt_chunk_found || !data_chunk_found {
            return Err("Missing 'fmt ' or 'data' chunk".into());
        }

        Ok(AudioBuffer {
            sample_rate,
            channels,
            format: sample_format,
            data: audio_data,
        })
    }
}

fn decode_samples(
    input: &[u8],
    channels: u16,
    bits_per_sample: u16,
) -> Result<Vec<Vec<f32>>, String> {
    let mut reader = Cursor::new(input);
    let mut output: Vec<Vec<f32>> = vec![vec![]; channels as usize];

    while (reader.position() as usize) < input.len() {
        for ch in 0..channels {
            let sample = match bits_per_sample {
                8 => reader.read_u8()? as f32 / 128.0 - 1.0,
                16 => reader.read_i16::<LittleEndian>()? as f32 / 32768.0,
                24 => {
                    let mut bytes = [0u8; 3];
                    reader.read_exact(&mut bytes)?;
                    let val =
                        ((bytes[2] as i32) << 16) | ((bytes[1] as i32) << 8) | (bytes[0] as i32);
                    let val = if val & 0x800000 != 0 {
                        val | !0xFFFFFF
                    } else {
                        val
                    };
                    val as f32 / 8388608.0
                }
                32 => reader.read_i32::<LittleEndian>()? as f32 / 2147483648.0,
                _ => return Err("Unsupported bit depth".into()),
            };
            output[ch as usize].push(sample);
        }
    }

    Ok(output)
}

impl Codec for WavCodec {
    fn encode(buffer: &AudioBuffer) -> Result<Vec<u8>, String> {
        let mut output = Cursor::new(Vec::new());

        // Placeholder for header
        output.write_all(b"RIFF")?;
        output.write_u32::<LittleEndian>(0)?; // placeholder file size
        output.write_all(b"WAVE")?;

        // ---- fmt chunk ----
        output.write_all(b"fmt ")?;
        output.write_u32::<LittleEndian>(16)?; // PCM = 16 bytes
        let (format_tag, bits_per_sample) = match buffer.format {
            SampleFormat::F32 => (3u16, 32),
            SampleFormat::I16 => (1u16, 16),
            SampleFormat::I24 => (1u16, 24),
            SampleFormat::I32 => (1u16, 32),
            SampleFormat::U8 => (1u16, 8),
        };
        let channels = buffer.channels;
        let sample_rate = buffer.sample_rate;
        let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
        let block_align = channels * (bits_per_sample / 8) as u16;

        output.write_u16::<LittleEndian>(format_tag)?;
        output.write_u16::<LittleEndian>(channels)?;
        output.write_u32::<LittleEndian>(sample_rate)?;
        output.write_u32::<LittleEndian>(byte_rate)?;
        output.write_u16::<LittleEndian>(block_align)?;
        output.write_u16::<LittleEndian>(bits_per_sample)?;

        // ---- data chunk ----
        output.write_all(b"data")?;
        let data_pos = output.position();
        output.write_u32::<LittleEndian>(0)?; // placeholder

        let start_data = output.position();

        write_samples(&mut output, buffer, bits_per_sample)?;

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

fn write_samples<W: Write>(
    out: &mut W,
    buffer: &AudioBuffer,
    bits_per_sample: u16,
) -> Result<(), String> {
    let channels = buffer.channels as usize;
    let frames = buffer.data[0].len();

    for i in 0..frames {
        for ch in 0..channels {
            let sample = buffer.data[ch][i];
            match bits_per_sample {
                8 => {
                    let val = ((sample * 127.0 + 128.0).clamp(0.0, 255.0)) as u8;
                    out.write_u8(val)?;
                }
                16 => {
                    let val = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                    out.write_i16::<LittleEndian>(val)?;
                }
                24 => {
                    let val = (sample.clamp(-1.0, 1.0) * 8_388_607.0) as i32;
                    let bytes = [
                        (val & 0xFF) as u8,
                        ((val >> 8) & 0xFF) as u8,
                        ((val >> 16) & 0xFF) as u8,
                    ];
                    out.write_all(&bytes)?;
                }
                32 => {
                    if buffer.format == SampleFormat::F32 {
                        out.write_f32::<LittleEndian>(sample)?;
                    } else {
                        let val = (sample.clamp(-1.0, 1.0) * 2_147_483_647.0) as i32;
                        out.write_i32::<LittleEndian>(val)?;
                    }
                }
                _ => return Err("Unsupported bit depth".into()),
            }
        }
    }

    Ok(())
}
