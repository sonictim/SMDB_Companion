pub use crate::prelude::*;

use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub fn decode_audio_for_fingerprint(path: &Path) -> Vec<i16> {
    resample_interleaved(decode_interleaved(path), 48000)
        .samples
        .iter()
        .map(|&float| (float * 32767.0) as i16)
        .collect()
}

pub struct DecodedAudioInterleaved {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

pub fn decode_interleaved(path: &Path) -> DecodedAudioInterleaved {
    // Create a media source. Note that the MediaSource trait is automatically implemented for File,
    // among other types.
    let file = Box::new(File::open(path).unwrap());

    // Create the media source stream using the boxed media source from above.
    let mss = MediaSourceStream::new(file, Default::default());

    // Create a hint to help the format registry guess what format reader is appropriate. In this
    // example we'll leave it empty.
    let hint = Hint::new();

    // Use the default options when reading and decoding.
    let format_opts: FormatOptions = Default::default();
    let metadata_opts: MetadataOptions = Default::default();
    let decoder_opts: DecoderOptions = Default::default();

    // Probe the media source stream for a format.
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .unwrap();

    // Get the format reader yielded by the probe operation.
    let mut format = probed.format;

    // Get the default track.
    let track = format.default_track().unwrap();

    // Create a decoder for the track.
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .unwrap();

    // Store the track identifier, we'll use it to filter packets.
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(48000);
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(1);

    let mut sample_count = 0;
    let mut sample_buf = None;
    let mut decoded_samples = Vec::new();

    loop {
        // Get the next packet from the format reader.
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(err) => {
                if ignore_end_of_stream_error(Err(err)).is_ok() {
                    break;
                } else {
                    continue;
                }
            }
        };

        // If the packet does not belong to the selected track, skip it.
        if packet.track_id() != track_id {
            continue;
        }

        // Decode the packet into audio samples, ignoring any decode errors.
        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                // The decoded audio samples may now be accessed via the audio buffer if per-channel
                // slices of samples in their native decoded format is desired. Use-cases where
                // the samples need to be accessed in an interleaved order or converted into
                // another sample format, or a byte buffer is required, are covered by copying the
                // audio buffer into a sample buffer or raw sample buffer, respectively. In the
                // example below, we will copy the audio buffer into a sample buffer in an
                // interleaved order while also converting to a f32 sample format.

                // If this is the *first* decoded packet, create a sample buffer matching the
                // decoded audio buffer format.
                if sample_buf.is_none() {
                    // Get the audio buffer specification.
                    let spec = *audio_buf.spec();

                    // Get the capacity of the decoded buffer. Note: This is capacity, not length!
                    let duration = audio_buf.capacity() as u64;

                    // Create the f32 sample buffer.
                    sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                }

                // Copy the decoded audio buffer into the sample buffer in an interleaved format.
                if let Some(buf) = &mut sample_buf {
                    buf.copy_interleaved_ref(audio_buf);
                    decoded_samples.extend_from_slice(buf.samples());

                    // The samples may now be access via the `samples()` function.
                    sample_count += buf.samples().len();
                    print!("\rDecoded {} samples", sample_count);
                }
            }
            Err(Error::DecodeError(_)) => (),
            Err(err) => {
                if ignore_end_of_stream_error(Err(err)).is_ok() {
                    break;
                } else {
                    continue;
                }
            }
        }
    }
    DecodedAudioInterleaved {
        samples: decoded_samples,
        sample_rate,
        channels,
    }
}

fn ignore_end_of_stream_error(result: Result<(), Error>) -> Result<(), Error> {
    match result {
        Err(Error::IoError(err))
            if err.kind() == std::io::ErrorKind::UnexpectedEof
                && err.to_string() == "end of stream" =>
        {
            // Do not treat "end of stream" as a fatal error. It's the currently only way a
            // format reader can indicate the media is complete.
            Ok(())
        }
        _ => result,
    }
}

pub struct DecodedAudioSeparated {
    pub channels_samples: Vec<Vec<f32>>, // Each inner Vec represents samples for one channel
    pub sample_rate: u32,
    pub channels: u16,
    pub metadata: HashMap<String, String>,
}

pub fn decode_separated(path: &Path) -> DecodedAudioSeparated {
    let file = Box::new(File::open(path).unwrap());
    let mss = MediaSourceStream::new(file, Default::default());
    let hint = Hint::new();
    let format_opts: FormatOptions = Default::default();
    let metadata_opts: MetadataOptions = Default::default();
    let decoder_opts: DecoderOptions = Default::default();
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .unwrap();
    let mut format = probed.format;
    let track = format.default_track().unwrap();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .unwrap();
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(48000);
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(1);
    let mut metadata = HashMap::new();

    if let Some(data) = format.metadata().current() {
        for tag in data.tags() {
            let key = tag.key.to_string();
            let value = tag.value.to_string();
            metadata.insert(key, value);
        }
    }

    // Initialize a vector of vectors, one for each channel
    let mut channels_samples: Vec<Vec<f32>> = (0..channels).map(|_| Vec::new()).collect();

    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(err) => {
                if ignore_end_of_stream_error(Err(err)).is_ok() {
                    break;
                } else {
                    continue;
                }
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                if sample_buf.is_none() {
                    let spec = *audio_buf.spec();
                    let duration = audio_buf.capacity() as u64;
                    sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                }

                if let Some(buf) = &mut sample_buf {
                    buf.copy_interleaved_ref(audio_buf);

                    // De-interleave the samples into separate channel vectors
                    let samples = buf.samples();
                    let num_channels = channels as usize;

                    for (i, &sample) in samples.iter().enumerate() {
                        let channel_idx = i % num_channels;
                        channels_samples[channel_idx].push(sample);
                    }
                }
            }
            Err(Error::DecodeError(_)) => continue,
            Err(err) => {
                if ignore_end_of_stream_error(Err(err)).is_ok() {
                    break;
                } else {
                    continue;
                }
            }
        }
    }

    DecodedAudioSeparated {
        channels_samples,
        sample_rate,
        channels,
        metadata,
    }
}

pub fn are_channels_identical(path: &Path) -> bool {
    let file = match File::open(path) {
        Ok(f) => Box::new(f),
        Err(_) => return false, // Handle errors gracefully
    };
    let mss = MediaSourceStream::new(file, Default::default());
    let hint = Hint::new();
    let format_opts: FormatOptions = Default::default();
    let metadata_opts: MetadataOptions = Default::default();
    let decoder_opts: DecoderOptions = Default::default();
    let probed =
        match symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts) {
            Ok(p) => p,
            Err(_) => return false,
        };
    let mut format = probed.format;
    let track = format.default_track().unwrap();
    let mut decoder =
        match symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts) {
            Ok(d) => d,
            Err(_) => return false,
        };
    let track_id = track.id;
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .unwrap_or(1);

    // If there's only one channel, return false immediately
    if channels == 1 {
        return false;
    }

    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    // Used to track if we've found any differences between channels
    let mut all_channels_identical = true;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(err) => {
                if ignore_end_of_stream_error(Err(err)).is_ok() {
                    break;
                } else {
                    continue;
                }
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                if sample_buf.is_none() {
                    let spec = *audio_buf.spec();
                    let duration = audio_buf.capacity() as u64;
                    sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                }

                if let Some(buf) = &mut sample_buf {
                    buf.copy_interleaved_ref(audio_buf);

                    // Check if channels are identical in this buffer
                    let samples = buf.samples();
                    let num_channels = channels as usize;
                    let frames = samples.len() / num_channels;

                    for frame in 0..frames {
                        let base_sample = samples[frame * num_channels]; // First channel's sample

                        // Compare first channel with all other channels
                        for ch in 1..num_channels {
                            // Use a small epsilon for floating point comparison
                            let epsilon = 1e-6;
                            if (samples[frame * num_channels + ch] - base_sample).abs() > epsilon {
                                all_channels_identical = false;
                                break;
                            }
                        }

                        if !all_channels_identical {
                            // Once we find channels are different, we can return early
                            return false;
                        }
                    }
                }
            }
            Err(Error::DecodeError(_)) => continue,
            Err(err) => {
                if ignore_end_of_stream_error(Err(err)).is_ok() {
                    break;
                } else {
                    continue;
                }
            }
        }
    }

    all_channels_identical
}

/// Resamples interleaved audio data to a new sample rate
///
/// # Arguments
/// * `data` - The `DecodedAudioInterleaved` containing audio samples and metadata
/// * `target_sample_rate` - The desired output sample rate
///
/// # Returns
/// A new `DecodedAudioInterleaved` with resampled audio at the target sample rate
pub fn resample_interleaved(
    data: DecodedAudioInterleaved,
    target_sample_rate: u32,
) -> DecodedAudioInterleaved {
    // If the sample rate is already the target rate, return the data unchanged
    if data.sample_rate == target_sample_rate {
        return data;
    }

    let channels = data.channels as usize;
    let source_sample_rate = data.sample_rate;

    // Calculate the number of frames (samples per channel)
    let frames = data.samples.len() / channels;

    // Create deinterleaved data (Rubato requires channel-separated format)
    let mut input_frames = vec![Vec::with_capacity(frames); channels];

    // Deinterleave the audio data
    for frame in 0..frames {
        for (channel, channel_frames) in input_frames.iter_mut().enumerate() {
            let sample_index = frame * channels + channel;
            if sample_index < data.samples.len() {
                channel_frames.push(data.samples[sample_index]);
            }
        }
    }

    // Configure the resampler
    // Using SincFixedIn for higher quality resampling with a configurable sinc interpolation
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        target_sample_rate as f64 / source_sample_rate as f64,
        2.0,
        params,
        frames,
        channels,
        // Set to true for continuous audio stream processing
    )
    .unwrap();

    // Process the audio frames
    let output_frames = resampler.process(&input_frames, None).unwrap();

    // Re-interleave the resampled audio data
    let resampled_len = output_frames[0].len();
    let mut resampled_samples = Vec::with_capacity(resampled_len * channels);

    for frame in 0..resampled_len {
        for channel in 0..channels {
            resampled_samples.push(output_frames[channel][frame]);
        }
    }

    // Return the resampled audio
    DecodedAudioInterleaved {
        samples: resampled_samples,
        sample_rate: target_sample_rate,
        channels: data.channels,
    }
}

/// Resamples channel-separated audio data to a new sample rate
///
/// # Arguments
/// * `data` - The `DecodedAudio` containing channel-separated audio samples and metadata
/// * `target_sample_rate` - The desired output sample rate
///
/// # Returns
/// A new `DecodedAudio` with resampled audio at the target sample rate
pub fn resample_separated(
    data: DecodedAudioSeparated,
    target_sample_rate: u32,
) -> DecodedAudioSeparated {
    // If the sample rate is already the target rate, return the data unchanged
    if data.sample_rate == target_sample_rate {
        return data;
    }

    let source_sample_rate = data.sample_rate;
    let channels = data.channels as usize;
    let frames = if !data.channels_samples.is_empty() {
        data.channels_samples[0].len()
    } else {
        return data; // Return original data if there are no samples
    };

    // Clone the data to ensure we have ownership for processing
    // We're already in the right format for Rubato (channel-separated)
    let input_frames: Vec<Vec<f32>> = data.channels_samples.clone();

    // Configure the resampler
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        target_sample_rate as f64 / source_sample_rate as f64,
        2.0,
        params,
        frames,
        channels,
    )
    .unwrap();

    // Process the audio frames
    // Since the data is already channel-separated, we can pass it directly to the resampler
    let output_frames = resampler.process(&input_frames, None).unwrap();

    // Return the resampled audio
    DecodedAudioSeparated {
        channels_samples: output_frames,
        sample_rate: target_sample_rate,
        channels: data.channels,
        metadata: data.metadata,
    }
}

impl FileRecord {
    pub fn get_raw_pcm(&self) -> Result<Vec<u8>> {
        use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType};

        let file = std::fs::File::open(self.get_filepath())?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a hint to help the format registry guess what format the file is
        let mut hint = Hint::new();
        hint.with_extension(self.get_extension());

        // Use the default options for format and metadata
        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let metadata_opts = MetadataOptions::default();

        // Probe the media source to determine its format
        let mut probed = ::symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .context("Failed to probe media format")?;

        // Get the default track
        let track = probed
            .format
            .default_track()
            .ok_or_else(|| anyhow::anyhow!("No default track found"))?;

        // Create a decoder for the track
        let mut decoder = ::symphonia::default::get_codecs()
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
                Err(::symphonia::core::errors::Error::ResetRequired) => {
                    // Reset the decoder when required
                    decoder.reset();
                    continue;
                }
                Err(::symphonia::core::errors::Error::IoError(ref e))
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
                Err(::symphonia::core::errors::Error::IoError(_)) => {
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
                        spec.rate,
                        target_sample_rate,
                        self.get_filename()
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
