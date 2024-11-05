use crate::prelude::*;
use claxon::FlacReader;
use hound::WavReader;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::BufReader;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Waveforms {
    pub enabled: bool,
    #[serde(skip)]
    pub config: Node,
}

impl NodeCommon for Waveforms {
    fn config(&mut self) -> &mut Node {
        &mut self.config
    }
    fn render(&mut self, ui: &mut egui::Ui, _: &Database) {
        ui.checkbox(&mut self.enabled, "Search Audio Waveforms for duplicates")
            .on_hover_text_at_pointer("Will Analyze the Audio Content to search for duplicates");
    }
    fn process(&mut self, db: &Database) {
        let db = db.clone();
        let progress_sender = self.config.progress.tx.clone();
        let status_sender = self.config.status.tx.clone();
        self.config
            .wrap_async(move || gather(db, progress_sender, status_sender))
    }
}
async fn gather(
    db: Database,
    progress: mpsc::Sender<Progress>,
    status: mpsc::Sender<Arc<str>>,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    println!("Searching for Duplicate Waveforms");

    // Using Mutex for counter
    let counter = Arc::new(Mutex::new(0));
    let wavemaps: Arc<Mutex<HashMap<String, Vec<FileRecord>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let _ = status.send("Gathering all File Records".into()).await;

    // Fetch records from the database asynchronously
    let records = db.fetch_all_filerecords().await.unwrap();
    let total = records.len();
    let _ = status
        .send(format!("Found {total} FileRecords").into())
        .await;

    // Use rayon to process records in parallel
    records.par_iter().for_each(|record| {
        let path = Path::new(&*record.path);

        if path.exists() {
            let wavemap = get_wavemap(record); // Compute wavemap for the record

            // Clone the Arc to use in the closure
            let wavemaps = wavemaps.clone();
            let counter = counter.clone(); // Clone the counter Arc

            // Lock the mutex to access wavemaps
            {
                let mut wavemaps = wavemaps.lock().unwrap();
                wavemaps.entry(wavemap).or_default().push(record.clone());
            }

            // Lock the mutex to update the counter
            let mut count = counter.lock().unwrap();
            *count += 1;

            // Send progress update
            let _ = progress.try_send(Progress {
                counter: *count,
                total,
            });
        }
    });

    // At this point, all records have been processed
    // You can also send a final progress update if needed
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

    // Count duplicates
    let wavemaps = wavemaps.lock().unwrap(); // Lock the mutex to read wavemaps
    for (_, records) in wavemaps.iter() {
        if records.len() > 1 {
            // Skip the first record and add the rest to results
            results.extend(records.iter().skip(1).cloned());

            duplicate_count += 1;
        }
    }
    println!("Found {duplicate_count} waveform duplicates");

    Ok(results)
}

fn get_wavemap(record: &FileRecord) -> String {
    hash_audio_content(&record.path).unwrap()
}
fn hash_audio_content(file_path: &str) -> Result<String> {
    // Open the file and choose handling based on extension
    let file = File::open(file_path)?;
    let extension = file_path.split('.').last().unwrap_or("");

    let audio_data = match extension.to_lowercase().as_str() {
        "flac" => read_flac_audio_data(&file)?,
        "wav" => read_wav_audio_data(&file)?,
        _ => anyhow::bail!("Unsupported file format"),
    };

    // Generate SHA-256 hash of the raw audio data
    let mut hasher = Sha256::new();
    hasher.update(&audio_data);
    let hash = hasher.finalize();

    // Convert hash to a hexadecimal string for readability
    // let hash_hex = format!("{:x}", hash);
    Ok(hex::encode_upper(hash))
}

fn read_flac_audio_data(file: &File) -> Result<Vec<u8>> {
    let mut reader = BufReader::new(file);
    let mut flac_reader = FlacReader::new(&mut reader)?;

    let mut audio_data = Vec::new();

    // Collect audio samples from the FLAC file
    for sample in flac_reader.samples() {
        let sample = sample?; // Unwrap the Result
        audio_data.extend_from_slice(&sample.to_le_bytes());
    }

    Ok(audio_data)
}

fn read_wav_audio_data(file: &File) -> Result<Vec<u8>> {
    let mut reader = BufReader::new(file);
    let wav_reader = WavReader::new(&mut reader)?;

    let mut audio_data = Vec::new();

    // Print information about the WAV file
    println!("WAV file: {:?}", wav_reader.spec());

    // Read samples as i32
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
