use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Converts raw PCM audio data to a WAV file
///
/// # Arguments
/// * `pcm_data` - Raw PCM audio data (mono)
/// * `sample_rate` - Sample rate in Hz (e.g., 44100)
/// * `bits_per_sample` - Bits per sample (typically 16)
/// * `output_path` - Path where the WAV file will be saved
///
/// # Returns
/// * `io::Result<()>` - Success or error
pub fn pcm_to_wav(
    pcm_data: &[u8],
    sample_rate: u32,
    bits_per_sample: u16,
    output_path: &Path,
) -> io::Result<()> {
    let mut file = File::create(output_path)?;

    // Calculate sizes
    let data_size = pcm_data.len() as u32;
    let bytes_per_sample = bits_per_sample / 8;
    let block_align = bytes_per_sample;
    let byte_rate = sample_rate * block_align as u32;

    // Write WAV header
    // "RIFF" chunk descriptor
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_size).to_le_bytes())?; // Chunk size
    file.write_all(b"WAVE")?;

    // "fmt " sub-chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // Subchunk1Size (16 for PCM)
    file.write_all(&1u16.to_le_bytes())?; // AudioFormat (1 for PCM)
    file.write_all(&1u16.to_le_bytes())?; // NumChannels (1 for mono)
    file.write_all(&sample_rate.to_le_bytes())?; // SampleRate
    file.write_all(&byte_rate.to_le_bytes())?; // ByteRate
    file.write_all(&block_align.to_le_bytes())?; // BlockAlign
    file.write_all(&bits_per_sample.to_le_bytes())?; // BitsPerSample

    // "data" sub-chunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?; // Subchunk2Size
    file.write_all(pcm_data)?; // The actual data

    Ok(())
}

/// Converts raw PCM audio data to a FLAC file
///
/// # Arguments
/// * `pcm_data` - Raw PCM audio data (mono)
/// * `sample_rate` - Sample rate in Hz (e.g., 44100)
/// * `bits_per_sample` - Bits per sample (typically 16)
/// * `output_path` - Path where the FLAC file will be saved
///
/// # Returns
/// * `io::Result<()>` - Success or error
pub fn pcm_to_flac(
    pcm_data: &[u8],
    sample_rate: u32,
    bits_per_sample: u16,
    output_path: &Path,
) -> io::Result<()> {
    // For FLAC encoding, we'll use the claxon crate
    // This function demonstrates how to use it

    // This would typically require adding claxon to your Cargo.toml:
    // [dependencies]
    // claxon = "0.4.3"

    use claxon::FlacEncoder;
    use claxon::FlacStreamInfo;
    use claxon::frame;

    let mut encoder = FlacEncoder::new(
        File::create(output_path)?,
        claxon::FlacStreamInfo {
            sample_rate,
            channels: 1, // Mono
            bits_per_sample: bits_per_sample as u32,
            max_frame_len: None, // Let the encoder decide
        },
    )?;

    // Convert PCM bytes to samples (assuming 16-bit PCM)
    let samples: Vec<i32> = if bits_per_sample == 16 {
        let mut samples = Vec::with_capacity(pcm_data.len() / 2);
        for chunk in pcm_data.chunks_exact(2) {
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as i32;
            samples.push(sample);
        }
        samples
    } else if bits_per_sample == 24 {
        let mut samples = Vec::with_capacity(pcm_data.len() / 3);
        for chunk in pcm_data.chunks_exact(3) {
            let sample = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], 0]);
            samples.push(sample);
        }
        samples
    } else if bits_per_sample == 32 {
        let mut samples = Vec::with_capacity(pcm_data.len() / 4);
        for chunk in pcm_data.chunks_exact(4) {
            let sample = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            samples.push(sample);
        }
        samples
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Unsupported bits per sample",
        ));
    };

    // Create a frame (buffer of samples)
    let frame = Frame::from_mono(&samples);

    // Write the frame
    encoder.write_frame(&frame)?;

    // Finalize the encoder
    encoder.finish()?;

    Ok(())
}

// Example usage
fn main() -> io::Result<()> {
    // This is just a simple example - in a real application,
    // you'd likely read PCM data from a file or another source
    let sample_pcm_data = vec![0u8; 1000]; // Dummy PCM data
    let sample_rate = 44100;
    let bits_per_sample = 16;

    // Convert to WAV
    pcm_to_wav(
        &sample_pcm_data,
        sample_rate,
        bits_per_sample,
        Path::new("output.wav"),
    )?;

    // Convert to FLAC
    pcm_to_flac(
        &sample_pcm_data,
        sample_rate,
        bits_per_sample,
        Path::new("output.flac"),
    )?;

    println!("Conversion complete!");
    Ok(())
}
