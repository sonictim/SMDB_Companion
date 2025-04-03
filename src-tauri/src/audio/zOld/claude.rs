use anyhow::{Result, anyhow};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Compares left and right channels of an audio file
/// Returns true if the channels are identical, false otherwise
/// If the file is mono or has more than 2 channels, returns an error
pub fn are_channels_identical<P: AsRef<Path>>(path: P) -> Result<bool> {
    // Open the media source
    let file = File::open(&path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint to help the format registry guess what format the media is
    let mut hint = Hint::new();
    // Use the file extension to provide a hint
    if let Some(extension) = path.as_ref().extension() {
        if let Some(extension_str) = extension.to_str() {
            hint.with_extension(extension_str);
        }
    }

    // Probe the media source
    let codecs: &symphonia::core::codecs::CodecRegistry = symphonia::default::get_codecs();
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();
    let decoder_opts = DecoderOptions::default();

    let probed =
        symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
    let mut format = probed.format;

    // Get the default track
    let track = format
        .default_track()
        .ok_or_else(|| anyhow!("No default track found"))?;

    // Verify it's a stereo file (2 channels)
    if track.codec_params.channels.map(|ch| ch.count()) != Some(2) {
        return Err(anyhow!("File must be stereo (2 channels)"));
    }

    // Create a decoder for the track
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .map_err(|_| anyhow!("Unsupported codec"))?;

    // Process frames and compare channels
    loop {
        // Get the next packet from the format reader
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(_) => break, // End of stream or error
        };

        // Decode the packet
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(_) => continue, // Skip packets that cannot be decoded
        };

        // Get the decoded audio data
        let spec = *decoded.spec();
        let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        let samples = sample_buf.samples();

        // Compare left and right channels
        for frame in 0..samples.len() / 2 {
            let left_sample = samples[frame * 2];
            let right_sample = samples[frame * 2 + 1];

            // Using an epsilon for floating point comparison
            const EPSILON: f32 = 1e-6;
            if (left_sample - right_sample).abs() > EPSILON {
                return Ok(false);
            }
        }
    }

    // If we got here, all samples were identical
    Ok(true)
}
