use rustfft::{FftPlanner, num_complex::Complex};
use std::{convert, f32::EPSILON};

pub fn compute_fft(signal: &[f32]) -> Vec<Complex<f32>> {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(signal.len());

    let mut buffer: Vec<Complex<f32>> = signal.iter().map(|&s| Complex::new(s, 0.0)).collect();
    fft.process(&mut buffer);

    buffer // Returns the frequency-domain representation
}

pub fn compare_fft(left_fft: &[Complex<f32>], right_fft: &[Complex<f32>]) -> bool {
    let threshold = 1e-6; // Small tolerance for floating-point precision

    left_fft
        .iter()
        .zip(right_fft.iter())
        .all(|(l, r)| (l - r).norm() < threshold)
}

// pub fn are_channels_identical(pcm_data: &[u8]) -> bool {
//     let pcm_data = convert_pcm_to_i16(pcm_data);

//     let (left, right) = split_stereo_channels(&pcm_data);

//     // Choose a power of 2 FFT size (e.g., 1024 samples)
//     let fft_size = 1024.min(left.len());

//     let left_fft = compute_fft(&left[..fft_size]);
//     let right_fft = compute_fft(&right[..fft_size]);

//     compare_fft(&left_fft, &right_fft)
// }

fn split_stereo_channels(pcm_data: &[i16]) -> (Vec<f32>, Vec<f32>) {
    let mut left_channel = Vec::with_capacity(pcm_data.len() / 2);
    let mut right_channel = Vec::with_capacity(pcm_data.len() / 2);

    for chunk in pcm_data.chunks_exact(2) {
        left_channel.push(chunk[0] as f32);
        right_channel.push(chunk[1] as f32);
    }

    (left_channel, right_channel)
}

pub fn convert_pcm_to_i16(pcm_data: &[u8]) -> Vec<i16> {
    pcm_data
        .chunks(4)
        .filter_map(|chunk| {
            if chunk.len() == 4 {
                let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
                let float = f32::from_le_bytes(bytes);
                Some((float * 32767.0) as i16)
            } else {
                None
            }
        })
        .collect()
}

pub fn are_channels_identical(pcm_data: &[u8]) -> bool {
    // First, extract bitdepth info from file or assume 16-bit
    let bits_per_sample = 16; // Should be from record.bitdepth

    // Print some debug info
    println!("PCM data length: {} bytes", pcm_data.len());
    println!(
        "First few bytes: {:?}",
        &pcm_data[0..16.min(pcm_data.len())]
    );

    // Convert based on actual bit depth
    let pcm_i16 = match bits_per_sample {
        16 => {
            // Try both endianness
            let mut samples_le = Vec::with_capacity(pcm_data.len() / 2);
            let mut samples_be = Vec::with_capacity(pcm_data.len() / 2);

            for chunk in pcm_data.chunks_exact(2) {
                samples_le.push(i16::from_le_bytes([chunk[0], chunk[1]]));
                samples_be.push(i16::from_be_bytes([chunk[0], chunk[1]]));
            }

            // Use whichever has more variation (real audio)
            let var_le = variance(&samples_le);
            let var_be = variance(&samples_be);
            println!("Variance LE: {}, BE: {}", var_le, var_be);

            if var_le > var_be {
                samples_le
            } else {
                samples_be
            }
        }
        24 => {
            // Handle 24-bit audio
            let mut samples = Vec::with_capacity(pcm_data.len() / 3);

            for chunk in pcm_data.chunks_exact(3) {
                // Try both endianness
                let sample_le =
                    ((chunk[0] as i32) | ((chunk[1] as i32) << 8) | ((chunk[2] as i32) << 16)) >> 8;
                let sample_be =
                    ((chunk[2] as i32) | ((chunk[1] as i32) << 8) | ((chunk[0] as i32) << 16)) >> 8;

                // Use the one with larger magnitude (likely correct)
                if sample_le.abs() > sample_be.abs() {
                    samples.push(sample_le as i16);
                } else {
                    samples.push(sample_be as i16);
                }
            }

            samples
        }
        32 => {
            // Handle 32-bit float
            let mut samples = Vec::with_capacity(pcm_data.len() / 4);

            for chunk in pcm_data.chunks_exact(4) {
                if chunk.len() == 4 {
                    let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
                    let float = f32::from_le_bytes(bytes);
                    samples.push((float * 32767.0) as i16);
                }
            }

            samples
        }
        _ => {
            println!("Unsupported bit depth: {}", bits_per_sample);
            return false;
        }
    };

    if pcm_i16.is_empty() {
        println!("No samples after conversion");
        return false;
    }

    // Split into actual channels (assuming stereo)
    let num_channels = 2; // Should come from record.channels
    let mut channels: Vec<Vec<f32>> =
        vec![Vec::with_capacity(pcm_i16.len() / num_channels); num_channels];

    for (i, &sample) in pcm_i16.iter().enumerate() {
        let channel_idx = i % num_channels;
        channels[channel_idx].push(sample as f32);
    }

    // Simple channel comparison without FFT
    // FFTs can mask differences and are too permissive
    if channels.len() < 2 {
        return false;
    }

    let reference = &channels[0];
    let mut matching_samples = 0;
    let mut total_samples = 0;

    for other_channel in channels.iter().skip(1) {
        if reference.len() != other_channel.len() {
            println!("Channel length mismatch");
            return false;
        }

        // Only check every 100th sample for speed
        for i in (0..reference.len()).step_by(100) {
            total_samples += 1;

            // Allow for minor differences (quantization, etc)
            if (reference[i] - other_channel[i]).abs() < 0.1 {
                matching_samples += 1;
            }
        }
    }

    let match_percentage = (matching_samples as f32 / total_samples as f32) * 100.0;
    println!("Channel similarity: {:.1}%", match_percentage);

    // More strict threshold: 95% similarity required
    match_percentage > 95.0
}

fn variance(samples: &[i16]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }

    let mean = samples.iter().map(|&s| s as f64).sum::<f64>() / samples.len() as f64;
    samples
        .iter()
        .map(|&s| (s as f64 - mean).powi(2))
        .sum::<f64>()
        / samples.len() as f64
}
