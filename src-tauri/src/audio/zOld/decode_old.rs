use anyhow::{Context, Result};
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, SampleBuffer, Signal, SignalSpec};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType};
use std::fs::File;

pub fn convert_to_raw_pcm(input_path: &str) -> Result<Vec<u8>> {
    use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType};

    let file = std::fs::File::open(input_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint to help the format registry guess what format the file is
    let mut hint = Hint::new();
    if let Some(extension) = Path::new(input_path).extension() {
        if let Some(ext_str) = extension.to_str() {
            hint.with_extension(ext_str);
        }
    }

    // Use the default options for format and metadata
    let format_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };
    let metadata_opts = MetadataOptions::default();

    // Probe the media source to determine its format
    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .context("Failed to probe media format")?;

    // Get the default track
    let track = probed
        .format
        .default_track()
        .ok_or_else(|| anyhow::anyhow!("No default track found"))?;

    // Create a decoder for the track
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create decoder")?;

    // Store the decoded PCM data
    let mut pcm_data = Vec::with_capacity(1_000_000); // Pre-allocate 1MB

    // Decode the track
    let mut sample_count = 0;
    let target_sample_rate = 48000; // Target sample rate for fingerprinting

    // Initialize resampler storage (only created if needed)
    let mut resampler = None;
    let mut last_spec = None;

    loop {
        // Get the next packet from the format reader
        let packet = match probed.format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::ResetRequired) => {
                // Reset the decoder when required
                decoder.reset();
                continue;
            }
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                // End of file reached
                break;
            }
            Err(e) => {
                // Some other error occurred
                return Err(anyhow::anyhow!("Error reading packet: {}", e));
            }
        };

        // Decode the packet
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(symphonia::core::errors::Error::IoError(_)) => {
                // Skip decoding errors
                continue;
            }
            Err(e) => {
                eprintln!("Error decoding packet: {}", e);
                continue;
            }
        };

        // Get the decoded audio buffer
        let spec = *decoded.spec();
        last_spec = Some(spec);

        // Create a buffer for the decoded audio
        let mut sample_buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);

        // Copy the decoded audio to the sample buffer
        sample_buffer.copy_interleaved_ref(decoded);
        let samples = sample_buffer.samples();

        // Check if we need to resample
        if spec.rate != target_sample_rate {
            // Create resampler if this is the first packet or if format changed
            if resampler.is_none() {
                println!(
                    "Resampling from {}Hz to {}Hz for {}",
                    spec.rate, target_sample_rate, input_path
                );

                // Calculate frames (samples per channel)
                let frames = samples.len() / spec.channels.count();

                // Create the resampler
                let resampler_result = SincFixedIn::<f32>::new(
                    target_sample_rate as f64 / spec.rate as f64,
                    2.0, // Oversampling factor
                    SincInterpolationParameters {
                        sinc_len: 256,
                        f_cutoff: 0.95,
                        interpolation: SincInterpolationType::Linear,
                        oversampling_factor: 256,
                        window: rubato::WindowFunction::Blackman,
                    },
                    frames,
                    spec.channels.count(),
                );

                match resampler_result {
                    Ok(r) => resampler = Some(r),
                    Err(e) => {
                        eprintln!("Failed to create resampler: {}", e);
                        resampler = None;
                    }
                }
            }

            // Prepare samples for resampling (convert interleaved to per-channel)
            let channels = spec.channels.count();
            let frames = samples.len() / channels;

            // Split interleaved samples into separate channel vectors
            let mut channel_samples = vec![Vec::with_capacity(frames); channels];
            for (i, &sample) in samples.iter().enumerate() {
                channel_samples[i % channels].push(sample);
            }

            // Perform resampling
            if let Some(resampler) = resampler.as_mut() {
                match resampler.process(&channel_samples, None) {
                    Ok(resampled) => {
                        // Calculate how many samples we have after resampling
                        let resampled_frames = resampled[0].len();
                        let _total_resampled_samples = resampled_frames * channels;

                        // Add resampled samples to PCM data (converting back to interleaved)
                        for frame in 0..resampled_frames {
                            for channel_data in resampled.iter() {
                                let sample = channel_data[frame];
                                pcm_data.extend_from_slice(&sample.to_le_bytes());
                                sample_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Resampling error: {}", e);
                        // Fall back to original samples if resampling fails
                        for &sample in samples {
                            pcm_data.extend_from_slice(&sample.to_le_bytes());
                            sample_count += 1;
                        }
                    }
                }
            }
        } else {
            // No resampling needed, use original samples
            for &sample in samples {
                pcm_data.extend_from_slice(&sample.to_le_bytes());
                sample_count += 1;
            }
        }

        // Apply a limit to prevent excessive memory usage (equivalent to 10 minutes at 48kHz)
        if sample_count > 10 * 60 * target_sample_rate {
            break;
        }
    }

    // Print audio format info for debugging
    if let Some(spec) = last_spec {
        println!(
            "Processed audio: {} channels, {}Hz, {} samples ({:.1} seconds)",
            spec.channels.count(),
            spec.rate,
            sample_count,
            sample_count as f32 / target_sample_rate as f32
        );
    }

    Ok(pcm_data)
}

pub fn convert_to_raw_pcm_old(input_path: &str) -> Result<Vec<u8>> {
    use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType};

    let file = std::fs::File::open(input_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint to help the format registry guess what format the file is
    let mut hint = Hint::new();
    if let Some(extension) = Path::new(input_path).extension() {
        if let Some(ext_str) = extension.to_str() {
            hint.with_extension(ext_str);
        }
    }

    // Use the default options for format and metadata
    let format_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };
    let metadata_opts = MetadataOptions::default();

    // Probe the media source to determine its format
    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .context("Failed to probe media format")?;

    // Get the default track
    let track = probed
        .format
        .default_track()
        .ok_or_else(|| anyhow::anyhow!("No default track found"))?;

    // Create a decoder for the track
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create decoder")?;

    // Store the decoded PCM data
    let mut pcm_data = Vec::with_capacity(1_000_000); // Pre-allocate 1MB

    // Decode the track
    let mut sample_count = 0;
    let target_sample_rate = 48000; // Target sample rate for fingerprinting

    // Initialize resampler storage (only created if needed)
    let mut resampler = None;
    let mut last_spec = None;

    loop {
        // Get the next packet from the format reader
        let packet = match probed.format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::ResetRequired) => {
                // Reset the decoder when required
                decoder.reset();
                continue;
            }
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                // End of file reached
                break;
            }
            Err(e) => {
                // Some other error occurred
                return Err(anyhow::anyhow!("Error reading packet: {}", e));
            }
        };

        // Decode the packet
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(symphonia::core::errors::Error::IoError(_)) => {
                // Skip decoding errors
                continue;
            }
            Err(e) => {
                eprintln!("Error decoding packet: {}", e);
                continue;
            }
        };

        // Get the decoded audio buffer
        let spec = *decoded.spec();
        last_spec = Some(spec);

        // Create a buffer for the decoded audio
        let mut sample_buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);

        // Copy the decoded audio to the sample buffer
        sample_buffer.copy_interleaved_ref(decoded);
        let samples = sample_buffer.samples();

        // Check if we need to resample
        if spec.rate != target_sample_rate {
            // Create resampler if this is the first packet or if format changed
            if resampler.is_none() {
                println!(
                    "Resampling from {}Hz to {}Hz for {}",
                    spec.rate, target_sample_rate, input_path
                );

                // Calculate frames (samples per channel)
                let frames = samples.len() / spec.channels.count();

                // Create the resampler
                let resampler_result = SincFixedIn::<f32>::new(
                    target_sample_rate as f64 / spec.rate as f64,
                    2.0, // Oversampling factor
                    SincInterpolationParameters {
                        sinc_len: 256,
                        f_cutoff: 0.95,
                        interpolation: SincInterpolationType::Linear,
                        oversampling_factor: 256,
                        window: rubato::WindowFunction::Blackman,
                    },
                    frames,
                    spec.channels.count(),
                );

                match resampler_result {
                    Ok(r) => resampler = Some(r),
                    Err(e) => {
                        eprintln!("Failed to create resampler: {}", e);
                        resampler = None;
                    }
                }
            }

            // Prepare samples for resampling (convert interleaved to per-channel)
            let channels = spec.channels.count();
            let frames = samples.len() / channels;

            // Split interleaved samples into separate channel vectors
            let mut channel_samples = vec![Vec::with_capacity(frames); channels];
            for (i, &sample) in samples.iter().enumerate() {
                channel_samples[i % channels].push(sample);
            }

            // Perform resampling
            if let Some(resampler) = resampler.as_mut() {
                match resampler.process(&channel_samples, None) {
                    Ok(resampled) => {
                        // Calculate how many samples we have after resampling
                        let resampled_frames = resampled[0].len();
                        let _total_resampled_samples = resampled_frames * channels;

                        // Add resampled samples to PCM data (converting back to interleaved)
                        for frame in 0..resampled_frames {
                            for channel_data in resampled.iter() {
                                let sample = channel_data[frame];
                                pcm_data.extend_from_slice(&sample.to_le_bytes());
                                sample_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Resampling error: {}", e);
                        // Fall back to original samples if resampling fails
                        for &sample in samples {
                            pcm_data.extend_from_slice(&sample.to_le_bytes());
                            sample_count += 1;
                        }
                    }
                }
            }
        } else {
            // No resampling needed, use original samples
            for &sample in samples {
                pcm_data.extend_from_slice(&sample.to_le_bytes());
                sample_count += 1;
            }
        }

        // Apply a limit to prevent excessive memory usage (equivalent to 10 minutes at 48kHz)
        if sample_count > 10 * 60 * target_sample_rate {
            break;
        }
    }

    // Print audio format info for debugging
    if let Some(spec) = last_spec {
        println!(
            "Processed audio: {} channels, {}Hz, {} samples ({:.1} seconds)",
            spec.channels.count(),
            spec.rate,
            sample_count,
            sample_count as f32 / target_sample_rate as f32
        );
    }

    Ok(pcm_data)
}

/// Splits interleaved PCM data into multiple separate channel vectors.
pub fn split_channels(pcm_data: &[u8], num_channels: usize) -> Vec<Vec<u8>> {
    let mut channels: Vec<Vec<u8>> =
        vec![Vec::with_capacity(pcm_data.len() / num_channels); num_channels];

    for (i, &sample) in pcm_data.iter().enumerate() {
        channels[i % num_channels].push(sample);
    }

    channels
}
