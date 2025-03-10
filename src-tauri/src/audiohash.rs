use std::env;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use chromaprint::Chromaprint;
use claxon::FlacReader;
use crossbeam_channel::{bounded, unbounded};
use hound::WavReader;
use minimp3::{Decoder, Frame};
use parking_lot::RwLock;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::process::{Command, Stdio};

// Process pool size for FFmpeg conversions
const MAX_FFMPEG_PROCESSES: usize = 4;
// Downsampling factor for large audio files
const DOWNSAMPLE_FACTOR: usize = 4;
// Threshold for large files (in bytes)
const LARGE_FILE_THRESHOLD: u64 = 100_000_000; // 100MB

// Cached audio data with timestamp for invalidation
#[derive(Clone)]
struct CachedAudioData {
    data: Vec<u8>,
    timestamp: std::time::SystemTime,
}

// Thread pool for FFmpeg processes
lazy_static::lazy_static! {
    // Replace mutex with a counting semaphore implementation
    static ref FFMPEG_SEMAPHORE: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    static ref PROCESS_CACHE: Arc<RwLock<lru::LruCache<String, CachedAudioData>>> =
        Arc::new(RwLock::new(lru::LruCache::new(std::num::NonZeroUsize::new(100).unwrap())));
    static ref FFMPEG_PATH: Arc<String> = Arc::new(get_ffmpeg_path());
}

pub fn process_files_in_parallel(
    file_paths: &[String],
    ignore_filetype: bool,
) -> Vec<(String, Result<String>)> {
    // Create a work queue with bounded capacity
    let (sender, receiver) = bounded::<(String, bool)>(32);
    let (result_sender, result_receiver) = unbounded::<(String, Result<String>)>();

    // Spawn worker threads based on CPU count
    let num_workers = num_cpus::get();
    let mut handles = Vec::with_capacity(num_workers);

    for _ in 0..num_workers {
        let receiver = receiver.clone();
        let result_sender = result_sender.clone();
        let handle = thread::spawn(move || {
            while let Ok((path, ignore)) = receiver.recv() {
                let hash_result = hash_audio_content(&path, ignore);
                result_sender.send((path.to_string(), hash_result)).unwrap();
            }
        });
        handles.push(handle);
    }

    // Send work to the queue
    for path in file_paths {
        sender.send((path.clone(), ignore_filetype)).unwrap();
    }
    drop(sender); // Close sender to signal no more work
    drop(result_sender); // Close original sender

    // Collect results
    let mut results = Vec::with_capacity(file_paths.len());
    while let Ok(result) = result_receiver.recv() {
        results.push(result);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    results
}

pub fn hash_audio_content(file_path: &str, ignore_filetypes: bool) -> Result<String> {
    // Check if file exists and get its modified time
    let file_timestamp = match std::fs::metadata(file_path) {
        Ok(metadata) => metadata.modified().unwrap_or(std::time::SystemTime::now()),
        Err(_) => return Err(anyhow::anyhow!("File does not exist: {}", file_path)),
    };

    // Check cache first, with timestamp validation
    {
        let cache = PROCESS_CACHE.read();
        if let Some(cached_data) = cache.peek(file_path) {
            // Only use cache if file hasn't been modified
            if cached_data.timestamp >= file_timestamp {
                return hash_audio_bytes(&cached_data.data);
            }
        }
    }

    let audio_data = read_audio_data(file_path, ignore_filetypes)
        .context(format!("Failed to read audio data from {}", file_path))?;

    // Cache the result for future use with timestamp
    {
        let mut cache = PROCESS_CACHE.write();
        cache.put(
            file_path.to_string(),
            CachedAudioData {
                data: audio_data.clone(),
                timestamp: file_timestamp,
            },
        );
    }

    hash_audio_bytes(&audio_data)
}

fn hash_audio_bytes(audio_data: &[u8]) -> Result<String> {
    // For very large files, use downsampled hashing
    if (audio_data.len() > 50_000_000) {
        // 50MB threshold
        return hash_downsampled_audio(audio_data);
    }

    // For large files, use chunked parallel hashing
    if (audio_data.len() > 10_000_000) {
        // 10MB threshold
        return hash_large_audio_content(audio_data);
    }

    // For medium files, use rayon's par_chunks
    if (audio_data.len() > 1_000_000) {
        // 1MB threshold
        return hash_medium_audio_content(audio_data);
    }

    // For smaller files, use regular hashing
    let mut hasher = Sha256::new();
    hasher.update(audio_data);
    let hash = hasher.finalize();
    Ok(hex::encode(hash))
}

// New function to handle extremely large files
fn hash_downsampled_audio(audio_data: &[u8]) -> Result<String> {
    // Create a downsampled version by taking every Nth sample
    let downsampled: Vec<u8> = audio_data
        .chunks(4) // Group by 4 bytes (assuming 32-bit samples)
        .step_by(DOWNSAMPLE_FACTOR)
        .flat_map(|chunk| chunk.iter().copied())
        .collect();

    // Hash the downsampled data
    let mut hasher = Sha256::new();
    hasher.update(&downsampled);
    Ok(hex::encode(hasher.finalize()))
}

// Optimized for medium-sized files
fn hash_medium_audio_content(audio_data: &[u8]) -> Result<String> {
    const CHUNK_SIZE: usize = 262_144; // 256KB chunks

    let hash = audio_data
        .par_chunks(CHUNK_SIZE)
        .fold(Sha256::new, |mut hasher, chunk| {
            hasher.update(chunk);
            hasher
        })
        .reduce(Sha256::new, |mut acc, partial| {
            acc.update(partial.finalize());
            acc
        });

    Ok(hex::encode(hash.finalize()))
}

// Optimized for large files
fn hash_large_audio_content(audio_data: &[u8]) -> Result<String> {
    const CHUNK_SIZE: usize = 1_000_000; // 1MB chunks

    let chunks: Vec<&[u8]> = audio_data.chunks(CHUNK_SIZE).collect();
    let chunk_hashes: Vec<Vec<u8>> = chunks
        .par_iter()
        .map(|chunk| {
            let mut chunk_hasher = Sha256::new();
            chunk_hasher.update(chunk);
            chunk_hasher.finalize().to_vec()
        })
        .collect();

    // Combine chunk hashes
    let mut final_hasher = Sha256::new();
    for hash in chunk_hashes {
        final_hasher.update(hash);
    }

    let final_hash = final_hasher.finalize();
    Ok(hex::encode(final_hash))
}

fn get_ffmpeg_path() -> String {
    // Try to get ffmpeg from system path first
    if let Ok(output) = Command::new("which").arg("ffmpeg").output() {
        if !output.stdout.is_empty() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }

    // Fall back to local path
    let current_dir = env::current_dir().expect("Failed to get current dir");
    let ffmpeg_path = current_dir.join("assets").join("ffmpeg").join("ffmpeg");
    ffmpeg_path.to_string_lossy().to_string()
}

fn read_audio_data(file_path: &str, ignore_filetypes: bool) -> Result<Vec<u8>> {
    // Skip processing if file doesn't exist
    if !Path::new(file_path).exists() {
        return Err(anyhow::anyhow!("File does not exist: {}", file_path));
    }

    // Check file size - use FFmpeg directly for large files regardless of type
    if let Ok(metadata) = std::fs::metadata(file_path) {
        if metadata.len() > LARGE_FILE_THRESHOLD {
            return convert_to_raw_pcm(file_path);
        }
    }

    if ignore_filetypes {
        return convert_to_raw_pcm(file_path);
    }

    let extension = Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "flac" => {
            let file = File::open(file_path)?;
            read_flac_audio_data(&file)
        }
        "wav" => {
            let file = File::open(file_path)?;
            read_wav_audio_data(&file)
        }
        "mp3" => {
            let file = File::open(file_path)?;
            read_mp3_audio_data(&file)
        }
        _ => convert_to_raw_pcm(file_path),
    }
}

fn convert_to_raw_pcm(input_path: &str) -> Result<Vec<u8>> {
    // Implement proper semaphore with MAX_FFMPEG_PROCESSES limit
    {
        let mut count = FFMPEG_SEMAPHORE.lock().unwrap();
        while *count >= MAX_FFMPEG_PROCESSES {
            // Release lock and wait before retrying
            drop(count);
            thread::sleep(Duration::from_millis(50));
            count = FFMPEG_SEMAPHORE.lock().unwrap();
        }
        *count += 1;
    }

    // Ensure we decrement the counter when done
    struct SemaphoreGuard;
    impl Drop for SemaphoreGuard {
        fn drop(&mut self) {
            let mut count = FFMPEG_SEMAPHORE.lock().unwrap();
            if *count > 0 {
                *count -= 1;
            }
        }
    }
    let _guard = SemaphoreGuard;

    let ffmpeg_binary_path = FFMPEG_PATH.as_str();
    let ffmpeg_command = Command::new(ffmpeg_binary_path)
        .arg("-i")
        .arg(input_path)
        .arg("-ar")
        .arg("48000")
        .arg("-ac")
        .arg("1")
        .arg("-f")
        .arg("f32le")
        .arg("-vn")
        .arg("-map_metadata")
        .arg("-1")
        // Add thread count limit to avoid overloading system
        .arg("-threads")
        .arg("2")
        // Add timeout for stuck processes
        .arg("-t")
        .arg("300") // 5-minute maximum processing time
        .arg("-")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to start FFmpeg")?;

    let mut ffmpeg_process = ffmpeg_command
        .stdout
        .ok_or_else(|| anyhow::anyhow!("Failed to open FFmpeg stdout"))?;

    let mut pcm_data = Vec::with_capacity(1_000_000); // Pre-allocate 1MB
    ffmpeg_process.read_to_end(&mut pcm_data)?;

    Ok(pcm_data)
}

fn read_flac_audio_data(file: &File) -> Result<Vec<u8>> {
    let mut reader = BufReader::with_capacity(65536, file); // 64KB buffer
    let mut flac_reader = FlacReader::new(&mut reader)?;
    let stream_info = flac_reader.streaminfo();
    let estimated_size =
        (stream_info.samples.unwrap_or(0) * (stream_info.bits_per_sample as u64) / 8) as usize;

    let mut audio_data = Vec::with_capacity(estimated_size.max(1024));

    // Use blocking approach for better performance
    let block_size = 4096;
    let mut buffer = Vec::with_capacity(block_size);

    for sample in flac_reader.samples() {
        let sample = sample?;
        buffer.extend_from_slice(&sample.to_le_bytes());

        if buffer.len() >= block_size {
            audio_data.append(&mut buffer);
            buffer = Vec::with_capacity(block_size);
        }
    }

    // Add any remaining data
    if !buffer.is_empty() {
        audio_data.append(&mut buffer);
    }

    Ok(audio_data)
}

fn read_wav_audio_data(file: &File) -> Result<Vec<u8>> {
    let mut reader = BufReader::with_capacity(65536, file); // 64KB buffer
    let wav_reader = WavReader::new(&mut reader)?;
    let spec = wav_reader.spec();
    let estimated_size = (wav_reader.duration() * (spec.bits_per_sample as u32) / 8) as usize;

    let mut audio_data = Vec::with_capacity(estimated_size.max(1024));

    // Process in blocks for better memory efficiency
    const BLOCK_SIZE: usize = 4096;
    let mut buffer = Vec::with_capacity(BLOCK_SIZE);
    let mut count = 0;

    for sample in wav_reader.into_samples::<i32>() {
        match sample {
            Ok(s) => {
                buffer.extend_from_slice(&s.to_le_bytes());
                count += 1;

                if count % BLOCK_SIZE == 0 {
                    audio_data.append(&mut buffer);
                    buffer = Vec::with_capacity(BLOCK_SIZE);
                }
            }
            Err(e) => {
                eprintln!("Error reading sample: {:?}", e);
                break;
            }
        }
    }

    // Add any remaining data
    if !buffer.is_empty() {
        audio_data.append(&mut buffer);
    }

    Ok(audio_data)
}

fn read_mp3_audio_data(file: &File) -> Result<Vec<u8>> {
    let mut reader = BufReader::with_capacity(65536, file); // 64KB buffer
    let mut decoder = Decoder::new(&mut reader);
    let mut audio_data = Vec::with_capacity(1_000_000); // Pre-allocate 1MB

    // Process MP3 frames in batches
    let mut batch = Vec::with_capacity(65536);
    let mut frame_count = 0;

    loop {
        match decoder.next_frame() {
            Ok(Frame { data, .. }) => {
                for sample in data {
                    batch.extend_from_slice(&sample.to_le_bytes());
                }

                frame_count += 1;

                // Every 20 frames, append to main data vector
                if frame_count % 20 == 0 {
                    audio_data.append(&mut batch);
                    batch = Vec::with_capacity(65536);
                }
            }
            Err(e) => {
                // Check if we're at the end of the file
                if format!("{:?}", e).contains("EOF") {
                    break;
                }
                eprintln!("Error decoding MP3 frame: {:?}", e);
                break;
            }
        }
    }

    // Add any remaining data
    if !batch.is_empty() {
        audio_data.append(&mut batch);
    }

    Ok(audio_data)
}

// Utility function to measure performance
pub fn measure_performance<F, T>(func: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = std::time::Instant::now();
    let result = func();
    let duration = start.elapsed();
    (result, duration)
}

pub fn get_chromaprint_fingerprint<P: AsRef<Path>>(file_path: P) -> Result<(String, String)> {
    let path_str = file_path.as_ref().to_string_lossy().to_string();
    println!("Generating Chromaprint fingerprint for: {}", path_str);

    // Get PCM data (reuse your existing function)
    let pcm_data = convert_to_raw_pcm(&path_str)?;
    println!("Got PCM data, length: {} bytes", pcm_data.len());

    // Convert to i16 samples
    let samples: Vec<i16> = pcm_data
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
        .collect();

    println!("Converted to {} i16 samples", samples.len());

    let mut c = Chromaprint::new();
    c.start(16000, 1);
    c.feed(&samples);
    c.finish();

    // Get both fingerprint formats
    let text_fingerprint = c.fingerprint();
    let raw_fingerprint = c.raw_fingerprint();

    // Base64-encode the raw fingerprint (more efficient for database storage)
    let encoded = raw_fingerprint.as_ref().map_or(String::new(), |v| {
        // Convert Vec<i32> to bytes before encoding
        let bytes: Vec<u8> = v.iter().flat_map(|&x| x.to_le_bytes()).collect();
        general_purpose::STANDARD.encode(bytes)
    });

    // println!(
    //     "Fingerprint generated: {}",
    //     text_fingerprint
    //         .as_ref()
    //         .unwrap_or(&"did not unwrap fingerprint".to_string())
    // );
    // println!(
    //     "  Length (raw): {} bytes",
    //     raw_fingerprint.as_ref().map_or(0, |v| v.len())
    // );
    // println!("  Encoded (B64): {} chars", encoded.len());

    // Return the encoded version for database storage
    Ok((text_fingerprint.unwrap_or_default(), encoded))
}
