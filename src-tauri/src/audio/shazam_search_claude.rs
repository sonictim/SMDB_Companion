use ndarray::{Array2, s};
use ndarray_stats::QuantileExt;
use rustfft::{FftPlanner, num_complex::Complex, num_traits::Float};
use std::collections::{HashMap, HashSet};

/// Represents a constellation point (time-frequency point) in the audio
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ConstellationPoint {
    time_idx: u32,
    freq_idx: u16,
}

/// Represents a target zone relative to an anchor point
#[derive(Debug, Clone, Copy)]
struct TargetZone {
    time_offset_min: u32,
    time_offset_max: u32,
    freq_offset_min: i32,
    freq_offset_max: i32,
}

/// Represents a hash formed from a pair of constellation points
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct AudioHash {
    anchor_freq: u16,
    target_freq: u16,
    delta_time: u16,
}

/// Represents an audio file and its fingerprint
#[derive(Debug, Clone)]
pub struct AudioFile {
    pub id: String,
    pub duration: f32,
    raw_fingerprint: Vec<ConstellationPoint>, // The constellation map
    hashes: HashMap<AudioHash, Vec<u32>>,     // Maps hashes to time points
}

/// Manages a database of audio fingerprints
pub struct AudioDatabase {
    audio_files: HashMap<String, AudioFile>,
    hash_index: HashMap<AudioHash, Vec<(String, u32)>>, // Maps hashes to (file_id, time) pairs
}

impl AudioDatabase {
    /// Create a new audio database
    pub fn new() -> Self {
        Self {
            audio_files: HashMap::new(),
            hash_index: HashMap::new(),
        }
    }

    /// Add an audio file to the database
    pub fn add_audio_file(
        &mut self,
        id: String,
        pcm_data: &[i16],
        sample_rate: u32,
    ) -> Result<(), String> {
        // Generate fingerprint from PCM data
        let file = fingerprint_audio(id.clone(), pcm_data, sample_rate)?;

        // Index all hashes
        for (hash, time_points) in &file.hashes {
            for &time in time_points {
                self.hash_index
                    .entry(*hash)
                    .or_insert_with(Vec::new)
                    .push((id.clone(), time));
            }
        }

        // Store the file
        self.audio_files.insert(id, file);
        Ok(())
    }

    /// Find files that may contain the query audio as a subset
    pub fn subset_match(&self, query_id: &str, threshold: usize) -> Vec<MatchResult> {
        let mut results = Vec::new();
        let query = match self.audio_files.get(query_id) {
            Some(f) => f,
            None => return results,
        };

        // Count hash matches for each potential parent file
        let mut match_counts: HashMap<String, HashMap<i32, u32>> = HashMap::new();

        // Check each hash in the query
        for (hash, query_times) in &query.hashes {
            // Look up this hash in our index
            if let Some(matches) = self.hash_index.get(hash) {
                for (file_id, file_time) in matches {
                    // Don't match against itself
                    if file_id == query_id {
                        continue;
                    }

                    // For each time this hash occurs in query
                    for &query_time in query_times {
                        // Calculate time offset: if query is a subset,
                        // this offset should be consistent across multiple matches
                        let offset = *file_time as i32 - query_time as i32;

                        match_counts
                            .entry(file_id.clone())
                            .or_insert_with(HashMap::new)
                            .entry(offset)
                            .and_modify(|count| *count += 1)
                            .or_insert(1);
                    }
                }
            }
        }

        // Find the best matches
        for (file_id, offsets) in match_counts {
            // Find the offset with the most matches
            let (best_offset, match_count) = offsets
                .iter()
                .max_by_key(|&(_, count)| count)
                .unwrap_or((&0, &0));

            // Only consider it a match if it meets the threshold
            if *match_count >= threshold as u32 {
                let parent = &self.audio_files[&file_id];
                let similarity = *match_count as f32 / query.hashes.len() as f32;

                results.push(MatchResult {
                    parent_id: file_id,
                    child_id: query_id.to_string(),
                    offset: if *best_offset < 0 {
                        0
                    } else {
                        *best_offset as u32
                    },
                    match_count: *match_count,
                    similarity,
                });
            }
        }

        // Sort by match count (descending)
        results.sort_by(|a, b| b.match_count.cmp(&a.match_count));
        results
    }

    /// Find all subset relationships in the entire database
    pub fn find_all_subset_matches(&self, threshold: usize) -> Vec<MatchResult> {
        let mut all_results = Vec::new();

        // Process files in order of increasing duration
        let mut ids: Vec<String> = self.audio_files.keys().cloned().collect();
        ids.sort_by(|a, b| {
            let dur_a = self.audio_files[a].duration;
            let dur_b = self.audio_files[b].duration;
            dur_a.partial_cmp(&dur_b).unwrap()
        });

        // For each file (starting with shortest), find potential parents
        for id in ids {
            let results = self.subset_match(&id, threshold);
            all_results.extend(results);
        }

        all_results
    }
}

/// Result of a match operation
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub parent_id: String,
    pub child_id: String,
    pub offset: u32,      // Time offset in the parent file (in spectrogram frames)
    pub match_count: u32, // Number of matching hashes
    pub similarity: f32,  // Ratio of matching hashes to total hashes in query
}

/// Generate a fingerprint from PCM audio data
fn fingerprint_audio(id: String, pcm_data: &[i16], sample_rate: u32) -> Result<AudioFile, String> {
    // Spectrogram parameters
    let window_size = 1024;
    let hop_size = 512;
    let num_frames = (pcm_data.len() - window_size) / hop_size + 1;
    let duration = pcm_data.len() as f32 / sample_rate as f32;

    // Compute spectrogram
    let spectrogram = compute_spectrogram(pcm_data, window_size, hop_size)?;

    // Find peaks in the spectrogram (constellation points)
    let constellation_points = find_peaks(&spectrogram);

    // Generate hashes from constellation points
    let (hashes, _) = generate_hashes(&constellation_points);

    Ok(AudioFile {
        id,
        duration,
        raw_fingerprint: constellation_points,
        hashes,
    })
}

/// Compute the spectrogram of an audio signal
fn compute_spectrogram(
    pcm_data: &[i16],
    window_size: usize,
    hop_size: usize,
) -> Result<Array2<f32>, String> {
    if pcm_data.len() < window_size {
        return Err("Audio too short for analysis".to_string());
    }

    let num_frames = (pcm_data.len() - window_size) / hop_size + 1;
    let mut spectrogram = Array2::zeros((num_frames, window_size / 2 + 1));

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);

    let hann_window: Vec<f32> = (0..window_size)
        .map(|i| {
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (window_size as f32 - 1.0)).cos())
        })
        .collect();

    for frame in 0..num_frames {
        let start = frame * hop_size;
        let end = start + window_size;

        // Apply window and convert to complex
        let mut buffer: Vec<Complex<f32>> = pcm_data[start..end]
            .iter()
            .enumerate()
            .map(|(i, &s)| Complex {
                re: s as f32 * hann_window[i] / 32768.0,
                im: 0.0,
            })
            .collect();

        // Perform FFT
        fft.process(&mut buffer);

        // Store magnitude spectrum (keep only the first half + DC)
        for bin in 0..=window_size / 2 {
            let magnitude = (buffer[bin].re.powi(2) + buffer[bin].im.powi(2)).sqrt();
            spectrogram[[frame, bin]] = magnitude;
        }
    }

    Ok(spectrogram)
}

/// Find the prominent peaks in a spectrogram
fn find_peaks(spectrogram: &Array2<f32>) -> Vec<ConstellationPoint> {
    let (n_frames, n_bins) = spectrogram.dim();
    let mut peaks = Vec::new();

    // Parameters
    let freq_min = 30; // Minimum frequency bin to consider
    let freq_max = 300; // Maximum frequency bin (helps avoid high-freq noise)
    let time_window = 5; // Time window for local maximum check
    let freq_window = 5; // Frequency window for local maximum check
    let threshold = 0.05; // Threshold relative to local maximum

    // Find local maxima
    for t in time_window..(n_frames - time_window) {
        // We'll find the top maxima in each time frame
        let mut frame_peaks = Vec::new();

        for f in freq_min..freq_max {
            let val = spectrogram[[t, f]];
            if val < threshold {
                continue;
            }

            // Check if this is a local maximum in both time and frequency
            let mut is_peak = true;

            for dt in -1..=1 {
                for df in -1..=1 {
                    if dt == 0 && df == 0 {
                        continue;
                    }

                    let t2 = (t as i32 + dt) as usize;
                    let f2 = (f as i32 + df) as usize;

                    if spectrogram[[t2, f2]] >= val {
                        is_peak = false;
                        break;
                    }
                }
                if !is_peak {
                    break;
                }
            }

            if is_peak {
                frame_peaks.push((f, val));
            }
        }

        // Sort by magnitude and take the top N peaks
        frame_peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Keep only the top 5 peaks per frame (or fewer if we found fewer)
        let num_to_keep = std::cmp::min(5, frame_peaks.len());
        for i in 0..num_to_keep {
            peaks.push(ConstellationPoint {
                time_idx: t as u32,
                freq_idx: frame_peaks[i].0 as u16,
            });
        }
    }

    peaks
}

/// Generate hashes from constellation points
fn generate_hashes(
    points: &[ConstellationPoint],
) -> (HashMap<AudioHash, Vec<u32>>, Vec<(AudioHash, u32)>) {
    let mut hashes = HashMap::new();
    let mut hash_list = Vec::new();

    // Target zones relative to anchor points
    let target_zones = vec![
        TargetZone {
            time_offset_min: 1,
            time_offset_max: 3,
            freq_offset_min: -30,
            freq_offset_max: 30,
        },
        TargetZone {
            time_offset_min: 3,
            time_offset_max: 6,
            freq_offset_min: -30,
            freq_offset_max: 30,
        },
        TargetZone {
            time_offset_min: 6,
            time_offset_max: 10,
            freq_offset_min: -30,
            freq_offset_max: 30,
        },
    ];

    // For each point, treat it as an anchor and find target points in target zones
    for (i, &anchor) in points.iter().enumerate() {
        for &zone in &target_zones {
            // Find points that fall within this target zone
            for &target in &points[i + 1..] {
                let time_delta = target.time_idx - anchor.time_idx;
                let freq_delta = target.freq_idx as i32 - anchor.freq_idx as i32;

                // Check if the point is in the target zone
                if time_delta >= zone.time_offset_min
                    && time_delta <= zone.time_offset_max
                    && freq_delta >= zone.freq_offset_min
                    && freq_delta <= zone.freq_offset_max
                {
                    // Create a hash from the anchor-target pair
                    let hash = AudioHash {
                        anchor_freq: anchor.freq_idx,
                        target_freq: target.freq_idx,
                        delta_time: time_delta as u16,
                    };

                    // Store the hash along with the time of the anchor point
                    hashes
                        .entry(hash)
                        .or_insert_with(Vec::new)
                        .push(anchor.time_idx);

                    hash_list.push((hash, anchor.time_idx));
                }
            }
        }
    }

    (hashes, hash_list)
}

// Modified peak-finding for sound effects and ambient audio
fn find_peaks_for_sound_effects(spectrogram: &Array2<f32>) -> Vec<ConstellationPoint> {
    let (n_frames, n_bins) = spectrogram.dim();
    let mut peaks = Vec::new();

    // Parameters - adjusted for sound effects
    let freq_min = 10; // Include lower frequencies
    let freq_max = 512; // Include more high frequencies
    let time_window = 3; // Smaller time window
    let freq_window = 3; // Smaller frequency window

    // For ambient sounds, use adaptive thresholding
    for t in time_window..(n_frames - time_window) {
        // Calculate local statistics in a larger window
        let mut local_mean = 0.0;
        let mut local_max = 0.0;
        let window_size = 15; // Larger context window

        // Compute local statistics
        for dt in -window_size..=window_size {
            for df in -window_size..=window_size {
                let t2 = ((t as i32 + dt).max(0).min(n_frames as i32 - 1)) as usize;
                let f2 = (freq_min as i32 + df).max(0).min(freq_max as i32 - 1) as usize;

                local_mean += spectrogram[[t2, f2]];
                local_max = local_max.max(spectrogram[[t2, f2]]);
            }
        }

        local_mean /= ((2 * window_size + 1) * (2 * window_size + 1)) as f32;

        // Adaptive threshold based on local statistics
        let adaptive_threshold = local_mean + 0.3 * (local_max - local_mean);

        // Find multiple bands of interest for each time frame
        let bands = [
            (20, 80),     // Low frequencies
            (80, 250),    // Low-mid frequencies
            (250, 500),   // Mid frequencies
            (500, 1000),  // High-mid frequencies
            (1000, 4000), // High frequencies
        ];

        // Find peaks in each band
        for &(band_min, band_max) in &bands {
            let freq_min_bin = (band_min.min(n_bins - 1)) as usize;
            let freq_max_bin = (band_max.min(n_bins - 1)) as usize;

            // Find the local maximum in this frequency band
            let mut max_val = 0.0;
            let mut max_freq = freq_min_bin;

            for f in freq_min_bin..freq_max_bin {
                if spectrogram[[t, f]] > max_val {
                    max_val = spectrogram[[t, f]];
                    max_freq = f;
                }
            }

            // Only keep it if it exceeds our adaptive threshold
            if max_val > adaptive_threshold {
                peaks.push(ConstellationPoint {
                    time_idx: t as u32,
                    freq_idx: max_freq as u16,
                });
            }
        }

        // For truly random noise, also track changes in energy over time
        let energy_t_minus_1 = spectrogram.slice(s![t - 1, freq_min..freq_max]).sum();
        let energy_t = spectrogram.slice(s![t, freq_min..freq_max]).sum();
        let energy_t_plus_1 = spectrogram.slice(s![t + 1, freq_min..freq_max]).sum();

        // Detect significant changes in energy (transients in the noise)
        let delta_prev = (energy_t - energy_t_minus_1).abs();
        let delta_next = (energy_t_plus_1 - energy_t).abs();

        if delta_prev > 0.2 * energy_t || delta_next > 0.2 * energy_t {
            // Add a point for significant energy changes
            let dominant_freq = spectrogram
                .slice(s![t, freq_min..freq_max])
                .argmax()
                .unwrap()
                + freq_min;

            peaks.push(ConstellationPoint {
                time_idx: t as u32,
                freq_idx: dominant_freq as u16,
            });
        }
    }

    peaks
}

/// Example usage function
pub fn demo_usage() -> Result<(), String> {
    println!("Starting demo usage of audio fingerprinting...");
    let mut db = AudioDatabase::new();

    // Add audio files to the database (normally you would load PCM data from files)
    let file1_pcm = vec![0i16; 44100 * 10]; // 10 seconds of silence
    let file2_pcm = vec![0i16; 44100 * 3]; // 3 seconds of silence

    db.add_audio_file("file1".to_string(), &file1_pcm, 44100)?;
    db.add_audio_file("file2".to_string(), &file2_pcm, 44100)?;

    // Find subset matches for a specific file
    let matches = db.subset_match("file2", 5); // At least 5 matching hashes

    for m in matches {
        println!(
            "File {} is a subset of {} (similarity: {:.2}%, offset: {})",
            m.child_id,
            m.parent_id,
            m.similarity * 100.0,
            m.offset
        );
    }

    // Find all subset relationships
    let all_matches = db.find_all_subset_matches(5);
    println!("Found {} subset relationships", all_matches.len());

    Ok(())
}
