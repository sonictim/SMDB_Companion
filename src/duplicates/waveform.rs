use crate::prelude::*;
use anyhow::Context;
use claxon::FlacReader;
use hound::WavReader;
use minimp3::{Decoder, Frame};
use order::OrderPanel;

use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use std::sync::RwLock;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Waveforms {
    pub enabled: bool,
    #[serde(skip)]
    pub config: Node,
    pub ignore_filetype: bool,
}

impl NodeCommon for Waveforms {
    fn config(&mut self) -> &mut Node {
        &mut self.config
    }
    fn enabled(&self) -> bool {
        self.enabled
    }
    fn render(&mut self, ui: &mut egui::Ui, _: &Database) {
        ui.checkbox(&mut self.enabled, "Audio Content Duplicate Search (slow)")
            .on_hover_text_at_pointer("Will match identical audio content with different filenames.\nRecommend running other searches first and this one on an already thinned database");
        // ui.label("This search is slower than the others");
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.checkbox(&mut self.ignore_filetype, "Ignore Filetypes");
        });
    }
    fn process(
        &mut self,
        db: &Database,
        metadata: &HashSet<String>,
        order: Arc<RwLock<OrderPanel>>,
    ) {
        let db = db.clone();
        let progress_sender = self.config.progress.tx.clone();
        let status_sender = self.config.status.tx.clone();
        let ignore = self.ignore_filetype;
        let order = order.clone();
        let metadata = metadata.clone();

        self.config
            .wrap_async(move || gather(db, progress_sender, status_sender, ignore, metadata, order))
    }
}
async fn gather(
    db: Database,
    progress: mpsc::Sender<Progress>,
    status: mpsc::Sender<Arc<str>>,
    ignore_filetypes: bool,
    metadata: HashSet<String>,
    order: Arc<RwLock<OrderPanel>>,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    println!("Searching for Duplicate Waveforms");

    // Using Mutex for counter
    let counter = Arc::new(Mutex::new(0));
    let wavemaps: Arc<Mutex<HashMap<String, Vec<FileRecord>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let _ = status.send("Gathering all File Records".into()).await;

    let query = &format!(
        "SELECT {} FROM {}",
        hashset_to_query_string(&metadata),
        TABLE
    );
    let rows = db.fetch(query).await;
    let mut records = HashSet::new();

    for row in rows {
        let mut file_record = FileRecord::new(&row);
        file_record.update_metadata(&row, &metadata);
        records.insert(file_record);
    }

    let total = records.len();
    let _ = status
        .send(format!("Found {total} FileRecords").into())
        .await;

    // Use rayon to process records in parallel
    records.par_iter().for_each(|record| {
        let counter = counter.clone(); // Clone the counter Arc
        let mut count = counter.lock().unwrap();
        *count += 1;

        let path = Path::new(record.data.get("FilePath").unwrap());
        if path.exists() {
            if let Ok(wavemap) = hash_audio_content(path.to_str().unwrap(), ignore_filetypes) {
                // Clone the Arc to use in the closure
                let wavemaps = wavemaps.clone();

                // Lock the mutex to update the counter

                // Send progress update
                let _ = status.try_send(format!("{}", path.display()).into());
                let _ = progress.try_send(Progress {
                    counter: *count,
                    total,
                });
                // Lock the mutex to access wavemaps
                {
                    let mut wavemaps = wavemaps.lock().unwrap();
                    wavemaps.entry(wavemap).or_default().push(record.clone());
                }
            };
        }
    });

    let final_count = *counter.lock().unwrap();
    let _ = progress
        .send(Progress {
            counter: final_count,
            total,
        })
        .await;

    let mut results = HashSet::new(); // Populate results as necessary

    println!("Counting total duplicates found");
    let mut duplicate_count = 0;

    let log_file_path = "waveform_log.txt";
    let mut file = File::create(log_file_path).unwrap();

    // Count duplicates
    let mut wavemaps = wavemaps.lock().unwrap(); // Lock the mutex to read wavemaps
    let order = order.read().unwrap();
    for (key, records) in &mut wavemaps.iter_mut() {
        if records.len() > 1 {
            let _ = writeln!(file, "Key: {key}");

            // Now we can safely mutate `records`
            order.sort_vec(records);

            // Use a reference to `records` instead of taking ownership
            for r in &*records {
                // Borrow records immutably
                let _ = writeln!(
                    file,
                    "{}",
                    r.data.get("FilePath").unwrap_or(&"".to_string())
                );
            }

            // Skip the first record and add the rest to results
            results.extend(records.iter().skip(1).cloned());

            duplicate_count += 1;
        }
    }
    println!("Found {duplicate_count} waveform duplicates");

    Ok(results)
}

fn read_audio_data(file_path: &str, ignore_filetypes: bool) -> Result<Vec<u8>> {
    if ignore_filetypes {
        return convert_to_raw_pcm(file_path);
    }

    let file = File::open(file_path)?;
    let extension = file_path.split('.').last().unwrap_or("");

    match extension.to_lowercase().as_str() {
        "flac" => read_flac_audio_data(&file),
        "wav" => read_wav_audio_data(&file),
        "mp3" => read_mp3_audio_data(&file),
        _ => convert_to_raw_pcm(file_path),
        // println!("{file_path} is not supported yet");
        // return Err(anyhow::anyhow!("Unsupported file format: {extension}"));
    }
}

fn hash_audio_content(file_path: &str, ignore_filetypes: bool) -> Result<String> {
    let Ok(audio_data) = read_audio_data(file_path, ignore_filetypes) else {
        return Err(anyhow::anyhow!("Could not get audio"));
    };
    // Generate SHA-256 hash of the raw audio data
    let mut hasher = Sha256::new();
    hasher.update(&audio_data);
    let hash = hasher.finalize();
    Ok(hex::encode(hash))
}

fn get_ffmpeg_path() -> String {
    // Get the path to the ffmpeg binary depending on platform (Linux/macOS/Windows)
    let current_dir = env::current_dir().expect("Failed to get current dir");
    let ffmpeg_path = current_dir.join("assets").join("ffmpeg").join("ffmpeg");

    ffmpeg_path.to_string_lossy().to_string()
}

fn convert_to_raw_pcm(input_path: &str) -> Result<Vec<u8>> {
    let ffmpeg_binary_path = get_ffmpeg_path();
    let ffmpeg_command = Command::new(ffmpeg_binary_path)
        .arg("-i")
        .arg(input_path)
        // Standardize to 16kHz mono - reduces data while preserving essential content
        .arg("-ar")
        .arg("16000")
        .arg("-ac")
        .arg("1")
        // // Apply audio filters to normalize the content
        // .arg("-af")
        // .arg("lowpass=f=7000,volume=volume=1.0:precision=double,dcshift=shift=0")
        // // This filter chain:
        // // 1. Applies a lowpass filter to remove high frequencies that differ between sample rates
        // // 2. Normalizes volume to ensure consistent levels
        // // 3. Removes DC offset which can vary between encodings
        .arg("-f")
        .arg("f32le")
        .arg("-vn")
        .arg("-map_metadata")
        .arg("-1")
        .arg("-")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to start FFmpeg")?;

    let mut ffmpeg_process = ffmpeg_command
        .stdout
        .ok_or_else(|| anyhow::anyhow!("Failed to open FFmpeg stdout"))?;

    let mut pcm_data = Vec::new();
    ffmpeg_process.read_to_end(&mut pcm_data)?;

    Ok(pcm_data)
}

fn read_flac_audio_data(file: &File) -> Result<Vec<u8>> {
    let mut reader = BufReader::new(file);
    let mut flac_reader = FlacReader::new(&mut reader)?;

    let mut audio_data = Vec::new();

    for sample in flac_reader.samples() {
        let sample = sample?; // Unwrap the Result
        audio_data.extend_from_slice(&sample.to_le_bytes());
    }

    Ok(audio_data)
}

fn read_wav_audio_data(file: &File) -> Result<Vec<u8>> {
    let mut reader = BufReader::new(file);
    let wav_reader = WavReader::new(&mut reader)?;
    // println!("WAV file: {:?}", wav_reader.spec());

    let mut audio_data = Vec::new();

    for sample in wav_reader.into_samples::<i32>() {
        match sample {
            Ok(s) => {
                // Convert to bytes and add to audio_data
                audio_data.extend_from_slice(&s.to_le_bytes());
            }
            Err(e) => {
                eprintln!("Error reading sample: {:?}", e);
                break; // Exit the loop on error
            }
        }
    }

    Ok(audio_data)
}

fn read_mp3_audio_data(file: &File) -> Result<Vec<u8>> {
    let mut reader = BufReader::new(file);
    let mut decoder = Decoder::new(&mut reader);
    let mut audio_data: Vec<u8> = Vec::new();

    loop {
        match decoder.next_frame() {
            Ok(Frame { data, .. }) => {
                // The frame data is raw PCM samples, so we can directly extend it
                for sample in data {
                    audio_data.extend_from_slice(&sample.to_le_bytes()); // Convert to little-endian bytes
                }
            }
            Err(e) => {
                eprintln!("Error decoding MP3 frame: {:?}", e);
                break; // Exit the loop on error
            }
        }
    }
    Ok(audio_data)
}
