use crate::prelude::*;

use once_cell::sync::Lazy;
pub use regex::Regex;
use std::collections::HashMap;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Deep {
    pub enabled: bool,
    #[serde(skip)]
    pub config: Node,
    extensions: AsyncTunnel<Vec<String>>,
    pub ignore_extension: bool,
}

impl Deep {
    pub fn abort(&mut self) {
        self.config.abort();
    }

    pub fn render(&mut self, ui: &mut egui::Ui, db: &Database) {
        self.extensions.recv2();
        // if let Some(ext) = self.extensions_io.recv() {
        //     self.extensions = ext;
        // }

        ui.checkbox(&mut self.enabled, "Similar Filename Duplicates Search")
            .on_hover_text_at_pointer(
                "Filenames ending in .#, .#.#.#, or .M will be examined as possible duplicates",
            );

        if self.extensions.get().is_empty() && !self.extensions.waiting() {
            // self.extensions_io.waiting = true;

            let Some(pool) = db.pool() else {
                return;
            };
            let tx = self.extensions.tx.clone();

            let _handle = tokio::spawn(async move {
                let results = get_audio_file_types(&pool).await;

                if let Ok(results) = results {
                    let _ = tx.send(results).await;
                }
            });

            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Gathering Filetypes from DB");
                // self.clear_status();
            });
        } else {
            ui.horizontal(|ui| {
                ui.add_space(24.0);

                if self.extensions.get().len() > 1 {
                    let text = if self.ignore_extension {"Checked: 'example.wav' and 'example.flac' will be considered duplicate filenames"}
                    else {"Unchecked: 'example.wav' and 'example.flac' will be considered unique filenames"};
                    ui.checkbox(&mut self.ignore_extension, "Ignore Filetypes").on_hover_text_at_pointer(text);

                } else {
                    ui.label("All Records are of Filetype:");
                    ui.label(&self.extensions.get()[0]);
                }
            });
        }
    }

    pub fn process(&mut self, db: &Database) {
        if self.enabled {
            let progress_sender = self.config.progress.tx.clone();
            let status_sender = self.config.status.tx.clone();
            let pool = db.pool().unwrap();
            let ignore = self.ignore_extension;
            self.config.wrap_async(move || {
                Self::async_gather(pool, progress_sender, status_sender, ignore)
            })
        }
    }
}

impl Deep {
    pub async fn async_gather(
        pool: SqlitePool,
        progress_sender: mpsc::Sender<Progress>,
        status_sender: mpsc::Sender<Arc<str>>,
        ignore_extension: bool,
    ) -> Result<HashSet<FileRecord>, sqlx::Error> {
        let mut file_groups: HashMap<String, Vec<FileRecord>> = HashMap::new();
        let _ = status_sender
            .send("Gathering Duplicates with Similar Filenames".into())
            .await;

        let query = &format!("SELECT rowid, filename, duration, filepath FROM {}", TABLE);

        let rows = sqlx::query(query).fetch_all(&pool).await?;
        let _ = status_sender.send("Organizing Results".into()).await;

        let total = rows.len();
        let mut count: usize = 0;

        // Use a parallel iterator to process the rows
        let processed_records: Vec<(String, FileRecord)> = rows
            .par_iter() // Use a parallel iterator
            .map(|row| {
                let file_record = FileRecord::new(row);
                let base_filename = get_root_filename(&file_record.filename, ignore_extension)
                    .unwrap_or_else(|| file_record.filename.to_string());
                (base_filename, file_record)
            })
            .collect();

        let _ = status_sender.send("Processing Records".into()).await;
        for (base_filename, file_record) in processed_records {
            file_groups
                .entry(base_filename)
                .or_default()
                .push(file_record);

            count += 1;

            if count % 100 == 0 {
                let _ = progress_sender.send(Progress { count, total }).await;
            }
        }

        let _ = status_sender.send("Finishing up".into()).await;

        let mut file_records = HashSet::new();
        for (root, records) in file_groups {
            if records.len() <= 1 {
                continue;
            }

            let root_found = records.iter().any(|record| {
                if ignore_extension {
                    let name = Path::new(&*record.filename)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap();
                    return name == root;
                }

                *record.filename == root
            });
            if root_found {
                file_records.extend(records.into_iter().filter(|record| {
                    if ignore_extension {
                        let name = Path::new(&*record.filename)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap();
                        return name != root;
                    }
                    *record.filename != root
                }));
            } else {
                file_records.extend(records.into_iter().skip(1));
            }
        }
        let _ = status_sender
            .send(format!("Found {} duplicate records", file_records.len()).into())
            .await;

        Ok(file_records)
    }
}

static FILENAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?P<base>.+?)(?:\.(?:\d+|M))*$").unwrap());

fn get_root_filename(filename: &str, ignore_extension: bool) -> Option<String> {
    let path = Path::new(filename);
    let mut name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    // Use the regex to capture the base name
    if let Some(caps) = FILENAME_REGEX.captures(&name) {
        name = caps["base"].to_string();
    } else {
        println!("{} Did not match Regex", filename);
    }

    if ignore_extension {
        return Some(name);
    }

    // Reattach the extension if it's not being ignored
    Some(format!("{name}.{extension}"))
}

pub async fn get_audio_file_types(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query("SELECT DISTINCT AudioFileType FROM justinmetadata")
        .fetch_all(pool)
        .await?;

    let audio_file_types: Vec<String> = rows
        .iter()
        .filter_map(|row| row.get::<Option<String>, _>("AudioFileType")) // Access the column directly
        .collect();

    Ok(audio_file_types)
}
