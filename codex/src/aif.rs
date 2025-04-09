use crate::wav::*;
use crate::*;
use anyhow::{Result, anyhow};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read, Write};

// Format tags
// const FORMAT_PCM: u16 = 1;
// const FORMAT_IEEE_FLOAT: u16 = 3;

// Chunk Identifiers
const FORM_CHUNK_ID: &[u8; 4] = b"FORM";
const AIFF_FORMAT_ID: &[u8; 4] = b"AIFF";
const FMT_CHUNK_ID: &[u8; 4] = b"COMM";
const DATA_CHUNK_ID: &[u8; 4] = b"SSND";

// Chunk Structures
const HEADER_SIZE: usize = 12; // FORM + size + AIFF
const MIN_VALID_FILE_SIZE: usize = 12;

pub struct AifCodec;

impl Codec for AifCodec {
    fn file_extension(&self) -> &'static str {
        "aif"
    }

    fn validate_file_format(&self, data: &[u8]) -> Result<()> {
        if data.len() < MIN_VALID_FILE_SIZE {
            return Err(anyhow!("File too small to be a valid AIFF"));
        }

        let mut cursor = Cursor::new(data);

        // Read FORM header
        let mut form = [0u8; 4];
        cursor.read_exact(&mut form)?;
        if &form != FORM_CHUNK_ID {
            return Err(anyhow!("Not a FORM file"));
        }

        cursor.read_u32::<BigEndian>()?; // File size
        let mut aiff = [0u8; 4];
        cursor.read_exact(&mut aiff)?;
        if &aiff != AIFF_FORMAT_ID {
            return Err(anyhow!("Not an AIFF file"));
        }

        Ok(())
    }

    fn encode(&self, buffer: &AudioBuffer) -> Result<Vec<u8>> {
        let mut output = Cursor::new(Vec::new());

        // Write FORM header
        output.write_all(FORM_CHUNK_ID)?;
        output.write_u32::<BigEndian>(0)?; // Placeholder for file size
        output.write_all(AIFF_FORMAT_ID)?;

        // Write COMM chunk
        output.write_all(FMT_CHUNK_ID)?;
        output.write_u32::<BigEndian>(18)?; // COMM chunk size
        output.write_u16::<BigEndian>(buffer.channels)?;
        output.write_u32::<BigEndian>(buffer.data[0].len() as u32)?; // Total frames
        let bits_per_sample = match buffer.format {
            SampleFormat::F32 => 32,
            SampleFormat::I16 => 16,
            SampleFormat::I24 => 24,
            SampleFormat::I32 => 32,
            SampleFormat::U8 => 8,
        };
        output.write_u16::<BigEndian>(bits_per_sample)?;
        output.write_f64::<BigEndian>(buffer.sample_rate as f64)?;

        // Write SSND chunk
        output.write_all(DATA_CHUNK_ID)?;
        output.write_u32::<BigEndian>(0)?; // Placeholder for chunk size
        output.write_u32::<BigEndian>(0)?; // Offset
        output.write_u32::<BigEndian>(0)?; // Block size

        let start_data = output.position();

        let interleaved_bytes = encode_samples_simd(buffer, bits_per_sample)?;
        output.write_all(&interleaved_bytes)?;

        let end_data = output.position();
        let data_size = (end_data - start_data) as u32;

        // Fill in SSND chunk size
        let mut out = output.into_inner();
        (&mut out[(start_data as usize - 8)..(start_data as usize - 4)])
            .write_u32::<BigEndian>(data_size + 8)?;

        // Fill in FORM file size
        let form_size = out.len() as u32 - 8;
        (&mut out[4..8]).write_u32::<BigEndian>(form_size)?;

        Ok(out)
    }

    fn decode(&self, input: &[u8]) -> Result<AudioBuffer> {
        self.validate_file_format(input)?;

        let mut cursor = Cursor::new(input);
        cursor.set_position(HEADER_SIZE as u64);

        let mut fmt_chunk_found = false;
        let mut data_chunk_found = false;
        let mut sample_format = SampleFormat::I16;
        let mut channels = 0;
        let mut sample_rate = 0;
        let mut bits_per_sample = 0;
        let mut audio_data = vec![];

        while (cursor.position() as usize) < input.len() {
            let mut chunk_id = [0u8; 4];
            cursor.read_exact(&mut chunk_id)?;
            let chunk_size = cursor.read_u32::<BigEndian>()? as usize;

            match &chunk_id {
                FMT_CHUNK_ID => {
                    fmt_chunk_found = true;
                    channels = cursor.read_u16::<BigEndian>()?;
                    // let frames = cursor.read_u32::<BigEndian>()?; // Total frames
                    bits_per_sample = cursor.read_u16::<BigEndian>()?;
                    sample_rate = cursor.read_f64::<BigEndian>()? as u32;

                    sample_format = match bits_per_sample {
                        8 => SampleFormat::U8,
                        16 => SampleFormat::I16,
                        24 => SampleFormat::I24,
                        32 => SampleFormat::I32,
                        _ => {
                            return Err(anyhow!("Unsupported bit depth: {}", bits_per_sample));
                        }
                    };
                }

                DATA_CHUNK_ID => {
                    data_chunk_found = true;
                    cursor.read_u32::<BigEndian>()?; // Offset
                    cursor.read_u32::<BigEndian>()?; // Block size

                    let mut raw_data = vec![0u8; chunk_size - 8];
                    cursor.read_exact(&mut raw_data)?;

                    audio_data = decode_samples_parallel(
                        &raw_data,
                        channels,
                        bits_per_sample,
                        Some(sample_format == SampleFormat::F32),
                    )?;
                }

                _ => {
                    cursor.set_position(cursor.position() + chunk_size as u64);
                }
            }
        }

        if !fmt_chunk_found || !data_chunk_found {
            return Err(anyhow!("Missing 'COMM' or 'SSND' chunk"));
        }

        Ok(AudioBuffer {
            sample_rate,
            channels,
            format: sample_format,
            data: audio_data,
        })
    }
}
