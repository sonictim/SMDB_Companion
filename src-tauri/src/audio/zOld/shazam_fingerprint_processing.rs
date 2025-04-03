use crate::shazam_search::*;
use hound::{SampleFormat, WavReader};
use rustfft::{FftPlanner, num_complex::Complex};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

const TARGET_SAMPLE_RATE: u32 = 8000; // Downsample to 8kHz for efficiency
const FFT_SIZE: usize = 1024;
const WINDOW_SIZE: usize = FFT_SIZE;
const HOP_SIZE: usize = FFT_SIZE / 2; // 50% overlap
const MIN_FREQ_BIN: usize = 10; // Approx 78Hz at 8kHz sample rate
const MAX_FREQ_BIN: usize = FFT_SIZE / 2; // Nyquist limit
const TARGET_POINTS_PER_SEC: usize = 20; // Target landmark density
const PAIR_TIME_WINDOW: f32 = 0.5; // Look for pairs within this window
const MAX_PAIRS_PER_ANCHOR: usize = 5; // Limit pairs per anchor point

pub fn generate_landmarks(audio_path: &str) -> Vec<LandmarkPair> {
    // Get mono audio data at target sample rate
    let (samples, sample_rate) = load_and_process_audio(audio_path);

    if samples.is_empty() {
        println!("Warning: No audio data loaded from {}", audio_path);
        return Vec::new();
    }

    // Create spectrogram
    let spectrogram = create_spectrogram(&samples, sample_rate);

    // Find spectral peaks
    let landmarks = find_spectral_peaks(&spectrogram, sample_rate);

    // Create pairs of landmarks
    create_landmark_pairs(&landmarks)
}

fn load_and_process_audio(audio_path: &str) -> (Vec<f32>, u32) {
    // Try to handle mp3, wav, flac, etc. using symphonia
    if let Ok(audio) = load_audio_with_symphonia(audio_path) {
        return audio;
    }

    // Fallback to WAV reading with hound
    match load_wav_file(audio_path) {
        Ok(audio) => audio,
        Err(_) => {
            println!("Failed to load audio from {}", audio_path);
            (Vec::new(), TARGET_SAMPLE_RATE)
        }
    }
}

fn load_audio_with_symphonia(audio_path: &str) -> Result<(Vec<f32>, u32), String> {
    let file = std::fs::File::open(audio_path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = Hint::new();
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();
    let decoder_opts = DecoderOptions::default();

    // Probe the media source
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| format!("Error probing format: {:?}", e))?;

    let mut format = probed.format;
    let track = format.default_track().unwrap();
    let track_id = track.id;

    // Get the decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .map_err(|e| format!("Error creating decoder: {:?}", e))?;

    // Get track info
    let track_info = format!("{:?}", track.codec_params.codec);

    println!("Track codec: {}", track_info);
    println!("Sample rate: {:?}", track.codec_params.sample_rate);

    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let mut samples = Vec::new();

    // Decode frames
    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = decoded.spec();
                let num_channels = spec.channels.count();
                let frames = decoded.frames(); // Store frames count before decoded is moved
                let mut sample_buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *spec);

                sample_buffer.copy_interleaved_ref(decoded);

                // Convert to mono by averaging channels if needed
                if num_channels > 1 {
                    // num_channels is already defined above
                    let samples_slice = sample_buffer.samples();
                    for i in 0..frames {
                        let mut sum = 0.0;
                        for ch in 0..num_channels {
                            let index = i * num_channels + ch;
                            sum += samples_slice[index];
                        }
                        samples.push(sum / num_channels as f32);
                    }
                } else {
                    samples.extend_from_slice(sample_buffer.samples());
                }
            }
            Err(e) => {
                println!("Error decoding frame: {:?}", e);
                break;
            }
        }
    }

    // Resample if needed
    let resampled = if sample_rate != TARGET_SAMPLE_RATE {
        resample(&samples, sample_rate, TARGET_SAMPLE_RATE)
    } else {
        samples
    };

    Ok((resampled, TARGET_SAMPLE_RATE))
}

fn load_wav_file(path: &str) -> Result<(Vec<f32>, u32), String> {
    let mut reader = WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;

    let mut samples = Vec::new();

    match spec.sample_format {
        SampleFormat::Int => match spec.bits_per_sample {
            8 => {
                for sample in reader.samples::<i8>() {
                    let sample = sample.map_err(|e| e.to_string())?;
                    samples.push(sample as f32 / 128.0);
                }
            }
            16 => {
                for sample in reader.samples::<i16>() {
                    let sample = sample.map_err(|e| e.to_string())?;
                    samples.push(sample as f32 / 32768.0);
                }
            }
            24 | 32 => {
                for sample in reader.samples::<i32>() {
                    let sample = sample.map_err(|e| e.to_string())?;
                    samples.push(sample as f32 / 2147483648.0);
                }
            }
            _ => {
                return Err(format!(
                    "Unsupported bits per sample: {}",
                    spec.bits_per_sample
                ));
            }
        },
        SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                let sample = sample.map_err(|e| e.to_string())?;
                samples.push(sample);
            }
        }
    }

    // Convert to mono if needed by averaging channels
    let channels = spec.channels as usize;
    if channels > 1 {
        let mono_samples = (0..samples.len() / channels)
            .map(|i| {
                let mut sum = 0.0;
                for ch in 0..channels {
                    sum += samples[i * channels + ch];
                }
                sum / channels as f32
            })
            .collect();
        samples = mono_samples;
    }

    // Resample if needed
    let resampled = if sample_rate != TARGET_SAMPLE_RATE {
        resample(&samples, sample_rate, TARGET_SAMPLE_RATE)
    } else {
        samples
    };

    Ok((resampled, TARGET_SAMPLE_RATE))
}

// Simple linear interpolation resampling
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    let ratio = from_rate as f64 / to_rate as f64;
    let new_len = (samples.len() as f64 / ratio).ceil() as usize;
    let mut resampled = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let pos = i as f64 * ratio;
        let pos_floor = pos.floor() as usize;
        let pos_ceil = (pos_floor + 1).min(samples.len() - 1);
        let fraction = pos - pos_floor as f64;

        let value =
            samples[pos_floor] * (1.0 - fraction as f32) + samples[pos_ceil] * fraction as f32;
        resampled.push(value);
    }

    resampled
}

// Create a spectrogram from audio samples
fn create_spectrogram(samples: &[f32], sample_rate: u32) -> Vec<Vec<f32>> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);

    let num_frames = (samples.len() - WINDOW_SIZE) / HOP_SIZE + 1;
    let mut spectrogram = Vec::with_capacity(num_frames);

    // Prepare hann window for better frequency resolution
    let hann_window: Vec<f32> = (0..WINDOW_SIZE)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / WINDOW_SIZE as f32).cos()))
        .collect();

    for frame_idx in 0..num_frames {
        let start = frame_idx * HOP_SIZE;
        let end = start + WINDOW_SIZE;

        if end > samples.len() {
            break;
        }

        // Apply window and prepare FFT input
        let mut fft_buffer: Vec<Complex<f32>> = samples[start..end]
            .iter()
            .zip(hann_window.iter())
            .map(|(&s, &w)| Complex { re: s * w, im: 0.0 })
            .collect();

        // Pad if needed
        fft_buffer.resize(FFT_SIZE, Complex { re: 0.0, im: 0.0 });

        // Perform FFT in-place
        fft.process(&mut fft_buffer);

        // Compute magnitude spectrum (only need first half due to symmetry)
        let magnitudes: Vec<f32> = fft_buffer[..FFT_SIZE / 2]
            .iter()
            .map(|c| (c.re * c.re + c.im * c.im).sqrt())
            .collect();

        spectrogram.push(magnitudes);
    }

    spectrogram
}

// Find spectral peaks in the spectrogram
fn find_spectral_peaks(spectrogram: &[Vec<f32>], sample_rate: u32) -> Vec<AudioLandmark> {
    let mut landmarks = Vec::new();

    // Target density of points
    let frames_per_sec = sample_rate as f32 / HOP_SIZE as f32;
    let target_points =
        (spectrogram.len() as f32 * TARGET_POINTS_PER_SEC as f32 / frames_per_sec) as usize;

    // Find local maxima in the spectrogram
    let mut peak_values = Vec::new();

    for (t, frame) in spectrogram.iter().enumerate() {
        for f in MIN_FREQ_BIN..MAX_FREQ_BIN.min(frame.len()) {
            // Check if this is a local maximum in time and frequency
            let is_local_max = is_local_maximum(spectrogram, t, f, 3, 3);

            if is_local_max {
                let time = t as f32 * HOP_SIZE as f32 / sample_rate as f32;
                peak_values.push((frame[f], AudioLandmark::new(f as u16, time, frame[f])));
            }
        }
    }

    // Sort peaks by amplitude (descending)
    peak_values.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Take the strongest peaks
    landmarks = peak_values
        .into_iter()
        .take(target_points)
        .map(|(_, landmark)| landmark)
        .collect();

    // Sort by time for pairing
    landmarks.sort_by(|a, b| {
        a.time_offset
            .partial_cmp(&b.time_offset)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    landmarks
}

// Check if a point is a local maximum in the spectrogram
fn is_local_maximum(
    spectrogram: &[Vec<f32>],
    t: usize,
    f: usize,
    time_radius: usize,
    freq_radius: usize,
) -> bool {
    let value = spectrogram[t][f];

    // If value is too small, it's not interesting
    if value < 0.01 {
        return false;
    }

    let t_start = t.saturating_sub(time_radius);
    let t_end = (t + time_radius + 1).min(spectrogram.len());
    let f_start = f.saturating_sub(freq_radius);
    let f_end = (f + freq_radius + 1).min(spectrogram[t].len());

    for t2 in t_start..t_end {
        for f2 in f_start..f_end {
            // Skip comparing to self
            if t2 == t && f2 == f {
                continue;
            }

            if spectrogram[t2][f2] > value {
                return false;
            }
        }
    }

    true
}

// Create landmark pairs from spectral peaks
fn create_landmark_pairs(landmarks: &[AudioLandmark]) -> Vec<LandmarkPair> {
    let mut pairs = Vec::new();

    // For each landmark, find other landmarks within the target window
    for i in 0..landmarks.len() {
        let anchor = landmarks[i];
        let anchor_time = anchor.time_offset;

        // Keep track of pairs for this anchor to limit their number
        let mut anchor_pairs = 0;

        // Look ahead to find target points
        for j in (i + 1)..landmarks.len() {
            let target = landmarks[j];
            let target_time = target.time_offset;

            // Check if within time window
            if target_time - anchor_time > PAIR_TIME_WINDOW {
                break; // Too far ahead
            }

            // Create a landmark pair
            pairs.push(LandmarkPair::new(anchor, target));

            anchor_pairs += 1;
            if anchor_pairs >= MAX_PAIRS_PER_ANCHOR {
                break; // Enough pairs for this anchor
            }
        }
    }

    pairs
}
