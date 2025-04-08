use crate::prelude::*;
use rustfft::{FftPlanner, num_complex::Complex};
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::get_probe;

const MIN_MATCHES: usize = 5; // Minimum number of matches required to consider a result
const MATCH_THRESHOLD: f32 = 0.5; // Minimum score threshold for a match to be considered valid

struct AudioFingerprint {
    file_id: u64,
    hash_points: Vec<(u64, u32)>, // (hash, time_offset)
}

fn compute_spectrogram(
    audio_samples: &[f32],
    window_size: usize,
    hop_size: usize,
) -> Vec<Vec<f32>> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(window_size);

    let mut spectrogram = Vec::new();
    let mut window = vec![Complex::new(0.0, 0.0); window_size];

    for start in (0..audio_samples.len()).step_by(hop_size) {
        // Fill the window with audio samples, zero-padding if necessary
        for i in 0..window_size {
            window[i] = if start + i < audio_samples.len() {
                Complex::new(audio_samples[start + i], 0.0)
            } else {
                Complex::new(0.0, 0.0)
            };
        }

        // Perform FFT
        fft.process(&mut window);

        // Compute magnitude spectrum
        let magnitude_spectrum: Vec<f32> = window.iter().map(|c| c.norm()).collect();

        spectrogram.push(magnitude_spectrum);
    }

    spectrogram
}

fn generate_fingerprint(audio_samples: &[f32], file_id: u64) -> AudioFingerprint {
    // 1. Apply Short-Time Fourier Transform (STFT)
    let window_size = 1024;
    let hop_size = 512;
    let spectrogram = compute_spectrogram(audio_samples, window_size, hop_size);

    // 2. Find peak points in the spectrogram (local maxima)
    let peaks = find_peak_points(&spectrogram);

    // 3. Create constellation map by forming hash pairs from peaks
    let mut hash_points = Vec::new();
    let anchor_points = 5; // Number of points to pair with each anchor

    for window in peaks.windows(anchor_points + 1) {
        let anchor = &window[0];
        for target in &window[1..=anchor_points] {
            // Create hash from frequency and time differences
            let hash = compute_hash(anchor.freq, anchor.time, target.freq, target.time);
            hash_points.push((hash, anchor.time));
        }
    }

    AudioFingerprint {
        file_id,
        hash_points,
    }
}

fn compute_hash(freq1: u32, time1: u32, freq2: u32, time2: u32) -> u64 {
    // Combine frequency and time differences into a single hash value
    let freq_diff = freq2 as i64 - freq1 as i64;
    let time_diff = time2 as i64 - time1 as i64;

    // Use a simple hash combining function
    let hash = ((freq_diff & 0xFFFF) as u64) << 32 | ((time_diff & 0xFFFF) as u64);
    hash
}

fn find_peak_points(spectrogram: &[Vec<f32>]) -> Vec<PeakPoint> {
    let mut peaks = Vec::new();

    for (time, spectrum) in spectrogram.iter().enumerate() {
        for (freq, &magnitude) in spectrum.iter().enumerate() {
            // Check if the current point is a local maximum
            if is_local_maximum(spectrum, freq) {
                peaks.push(PeakPoint {
                    time: time as u32,
                    freq: freq as u32,
                    magnitude,
                });
            }
        }
    }

    peaks
}

fn is_local_maximum(spectrum: &[f32], index: usize) -> bool {
    let left = spectrum.get(index.wrapping_sub(1)).unwrap_or(&f32::MIN);
    let right = spectrum.get(index + 1).unwrap_or(&f32::MIN);
    spectrum[index] > *left && spectrum[index] > *right
}

struct PeakPoint {
    time: u32,
    freq: u32,
    magnitude: f32,
}

fn preprocess_audio(file_path: &str) -> Result<Vec<f32>, String> {
    // Open the audio file
    let file = File::open(file_path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(BufReader::new(file)), Default::default());

    // Probe the media source
    let probed = get_probe()
        .format(&FormatOptions::default(), &MetadataOptions::default(), mss)
        .map_err(|e| e.to_string())?;

    let mut format = probed.format;

    // Get the default track
    let track = format
        .default_track()
        .ok_or_else(|| "No default track found".to_string())?;

    // Create a decoder for the track
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| e.to_string())?;

    let mut samples = Vec::new();

    // Decode packets
    while let Ok(packet) = format.next_packet() {
        if let Ok(decoded) = decoder.decode(&packet) {
            for sample in decoded.samples() {
                samples.push(sample as f32 / i16::MAX as f32);
            }
        }
    }

    // Normalize audio
    let max_amplitude = samples.iter().cloned().fold(0.0, f32::max);
    let normalized: Vec<f32> = samples.into_iter().map(|s| s / max_amplitude).collect();

    Ok(normalized)
}

fn load_audio(file_path: &str) -> Result<Vec<f32>, String> {
    // Open the audio file
    let file = File::open(file_path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(BufReader::new(file)), Default::default());

    // Probe the media source
    let probed = get_probe()
        .format(&FormatOptions::default(), &MetadataOptions::default(), mss)
        .map_err(|e| e.to_string())?;

    let mut format = probed.format;

    // Get the default track
    let track = format
        .default_track()
        .ok_or_else(|| "No default track found".to_string())?;

    // Create a decoder for the track
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| e.to_string())?;

    let mut samples = Vec::new();

    // Decode packets
    while let Ok(packet) = format.next_packet() {
        if let Ok(decoded) = decoder.decode(&packet) {
            for sample in decoded.samples() {
                samples.push(sample as f32 / i16::MAX as f32);
            }
        }
    }

    Ok(samples)
}

struct FingerprintDatabase {
    pool: Pool<Sqlite>,
}

impl FingerprintDatabase {
    async fn new(path: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite://{}", path))
            .await?;

        // Create the fingerprints table if it doesn't exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS fingerprints (
                hash INTEGER PRIMARY KEY,
                file_id INTEGER NOT NULL,
                time_offset INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    async fn insert_fingerprint(&self, fingerprint: &AudioFingerprint) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        for (hash, time) in &fingerprint.hash_points {
            sqlx::query(
                "INSERT INTO fingerprints (hash, file_id, time_offset) VALUES (?1, ?2, ?3)",
            )
            .bind(hash)
            .bind(fingerprint.file_id)
            .bind(time)
            .execute(&mut tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn query(
        &self,
        query_fingerprint: &AudioFingerprint,
    ) -> Result<Vec<(u64, u32)>, sqlx::Error> {
        let mut results = Vec::new();

        for (hash, _) in &query_fingerprint.hash_points {
            let rows = sqlx::query_as::<_, (u64, u32)>(
                "SELECT file_id, time_offset FROM fingerprints WHERE hash = ?1",
            )
            .bind(hash)
            .fetch_all(&self.pool)
            .await?;

            results.extend(rows);
        }

        Ok(results)
    }
}

struct MatchResult {
    file_id: u64,
    score: f32,
    offset_matches: Vec<(u32, u32)>, // (query_time, target_time)
}

fn find_matches(
    query_fingerprint: &AudioFingerprint,
    db: &FingerprintDatabase,
) -> Vec<MatchResult> {
    let mut results = Vec::new();

    let mut match_counts: HashMap<u64, Vec<(u32, u32)>> = HashMap::new();

    // Query the database for each hash point
    for (hash, query_time) in &query_fingerprint.hash_points {
        if let Ok(matches) = db.query(query_fingerprint) {
            for (file_id, target_time) in matches {
                match_counts
                    .entry(file_id)
                    .or_default()
                    .push((*query_time, target_time));
            }
        }
    }

    // Process matches to find alignment
    for (file_id, matches) in match_counts {
        if matches.len() < MIN_MATCHES {
            continue;
        }

        // Find the most common time difference (histogram approach)
        let time_diffs = compute_time_differences(&matches);
        let (best_diff, count) = find_most_common_diff(time_diffs);

        // Calculate alignment score
        let score = count as f32 / query_fingerprint.hash_points.len() as f32;

        if score > MATCH_THRESHOLD {
            results.push(MatchResult {
                file_id,
                score,
                offset_matches: filter_matches_by_diff(&matches, best_diff),
            });
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results
}

fn generate_high_res_fingerprint(audio: &[f32]) -> AudioFingerprint {
    generate_fingerprint_high_res(audio)
}

fn generate_spectral_fingerprint(audio: &[f32]) -> AudioFingerprint {
    generate_spectral_fingerprint(audio)
}

fn process_fingerprints(file_records: &[FileRecord]) -> Vec<AudioFingerprint> {
    file_records
        .par_iter()
        .map(|record| generate_fingerprint(&preprocess_audio(record).unwrap(), record.id))
        .collect()
}

#[tauri::command]
fn search_similar_sounds(query_file_path: &str) -> Result<Vec<SimilarSoundResult>, String> {
    // 1. Load and preprocess audio file
    let audio = load_audio(query_file_path).map_err(|e| e.to_string())?;
    let preprocessed = preprocess_audio(&audio);

    // 2. Generate fingerprint
    let fingerprint = generate_fingerprint(&preprocessed, 0);

    // 3. Search database
    let matches = find_matches(&fingerprint, &FINGERPRINT_DB);

    // 4. Convert to result format for frontend
    Ok(matches.into_iter().map(convert_to_result).collect())
}
