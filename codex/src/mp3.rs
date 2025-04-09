use crate::*;
use anyhow::{Result, anyhow};
use minimp3::{Decoder as Mp3Decoder, Frame};
use rayon::prelude::*;
use std::io::Cursor;

pub struct Mp3Codec;

impl Codec for Mp3Codec {
    fn file_extension(&self) -> &'static str {
        "mp3"
    }

    fn validate_file_format(&self, data: &[u8]) -> Result<()> {
        if data.len() < 3 || &data[0..3] != b"ID3" {
            return Err(anyhow!("Invalid MP3 file: Missing ID3 tag"));
        }
        Ok(())
    }

    fn decode(&self, input: &[u8]) -> Result<AudioBuffer> {
        self.validate_file_format(input)?;

        let mut decoder = Mp3Decoder::new(Cursor::new(input));
        let mut audio_data: Vec<Vec<f32>> = Vec::new();
        let mut sample_rate = 0;
        let mut channels = 0;

        while let Ok(Frame {
            data,
            sample_rate: sr,
            channels: ch,
            ..
        }) = decoder.next_frame()
        {
            sample_rate = sr;
            channels = ch;

            if audio_data.is_empty() {
                audio_data = vec![Vec::new(); channels];
            }

            data.chunks_exact(channels).for_each(|chunk| {
                for (i, &sample) in chunk.iter().enumerate() {
                    audio_data[i].push(sample as f32 / i16::MAX as f32);
                }
            });
        }

        Ok(AudioBuffer {
            sample_rate: sample_rate as u32,
            channels: channels as u16,
            format: SampleFormat::F32,
            data: audio_data,
        })
    }

    fn encode(&self, buffer: &AudioBuffer) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        let mut lame =
            lame::Lame::new().ok_or_else(|| anyhow!("Failed to initialize LAME encoder"))?;

        lame.set_sample_rate(buffer.sample_rate)
            .map_err(|e| anyhow!("Lame error: {:?}", e))?;
        lame.set_channels(buffer.channels as u8)
            .map_err(|e| anyhow!("Lame error: {:?}", e))?;
        lame.set_quality(2)
            .map_err(|e| anyhow!("Lame error: {:?}", e))?; // High quality

        let interleaved_samples: Vec<i16> = buffer
            .data
            .par_iter()
            .flat_map(|channel| {
                channel
                    .iter()
                    .map(|&sample| (sample * i16::MAX as f32) as i16)
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut mp3_buffer = vec![0; interleaved_samples.len() * 5 / 4 + 7200];
        let bytes_written = match buffer.channels {
            1 => lame
                .encode(&interleaved_samples, &[], &mut mp3_buffer)
                .map_err(|e| anyhow!("Lame encoding error: {:?}", e))?,
            2 => {
                // Split interleaved stereo into left and right channels
                let (left, right): (Vec<i16>, Vec<i16>) = interleaved_samples
                    .chunks(2)
                    .map(|chunk| (chunk[0], chunk[1]))
                    .unzip();
                lame.encode(&left, &right, &mut mp3_buffer)
                    .map_err(|e| anyhow!("Lame encoding error: {:?}", e))?
            }
            _ => return Err(anyhow!("Only mono and stereo supported for MP3 encoding")),
        };
        mp3_buffer.truncate(bytes_written);
        output.extend_from_slice(&mp3_buffer);

        // Get any remaining frames from the encoder
        let mut flush_buffer = vec![0; 7200];
        // The 0.1.3 version of the lame crate doesn't have an encode_flush method,
        // so we need to use a different approach
        let bytes_written = match buffer.channels {
            1 => lame
                .encode(&[], &[], &mut flush_buffer)
                .map_err(|e| anyhow!("Lame flush error: {:?}", e))?,
            2 => lame
                .encode(&[], &[], &mut flush_buffer)
                .map_err(|e| anyhow!("Lame flush error: {:?}", e))?,
            _ => return Err(anyhow!("Only mono and stereo supported for MP3 encoding")),
        };
        flush_buffer.truncate(bytes_written);
        output.extend_from_slice(&flush_buffer);

        Ok(output)
    }
}
