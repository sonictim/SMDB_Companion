use crate::prelude::*;

use once_cell::sync::Lazy;
pub use regex::Regex;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Deep {
    pub enabled: bool,
    #[serde(skip)]
    pub config: Node,
    pub ignore_extension: bool,
}

impl NodeCommon for Deep {
    fn config(&mut self) -> &mut Node {
        &mut self.config
    }
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn render(&mut self, ui: &mut egui::Ui, db: &Database) {
        ui.checkbox(&mut self.enabled, "Similar Filename Duplicate Search")
            .on_hover_text_at_pointer(
                "Filenames ending in .#, .#.#.#, or .M will be examined as possible duplicates",
            );

        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.label(
                "Multi Mono Stems ending in .1 .2 will be flagged as duplicates. Use Caution.",
            );
        });

        match db.extensions.get().len() {
            0 => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Gathering Filetypes from DB");
                    // self.clear_status();
                });
            }
            1 => {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label("All Records are of Filetype:");
                    ui.label(&db.extensions.get()[0]);
                });
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    let text = &db.extensions.get().join(", ");
                    ui.label(format! {"Database contains: {text}"});
                });
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    let text = if self.ignore_extension {"Checked: 'example.wav' and 'example.flac' will be considered duplicate filenames"}
                    else {"Unchecked: 'example.wav' and 'example.flac' will be considered unique filenames"};
                    ui.checkbox(&mut self.ignore_extension, "Ignore Filetypes").on_hover_text_at_pointer(text);
            });
            }
        }
    }

    fn process(
        &mut self,
        db: &Database,
        columns: &HashSet<String>,
        order: Arc<RwLock<OrderPanel>>,
    ) {
        let progress_sender = self.config.progress.tx.clone();
        let status_sender = self.config.status.tx.clone();
        let pool = db.pool().unwrap();
        let ignore = self.ignore_extension;
        let columns = columns.clone();
        let order = order.clone();
        self.config.wrap_async(move || {
            Self::async_gather(pool, progress_sender, status_sender, ignore, columns, order)
        })
    }
}

impl Deep {
    pub async fn async_gather(
        pool: SqlitePool,
        progress_sender: mpsc::Sender<Progress>,
        status_sender: mpsc::Sender<Arc<str>>,
        ignore_extension: bool,
        columns: HashSet<String>,
        order: Arc<RwLock<OrderPanel>>,
    ) -> Result<HashSet<FileRecord>, sqlx::Error> {
        let mut file_groups: HashMap<String, Vec<FileRecord>> = HashMap::new();
        let _ = status_sender
            .send("Gathering Duplicates with Similar Filenames".into())
            .await;

        let query = &format!(
            "SELECT {} FROM {}",
            hashset_to_query_string(&columns),
            TABLE
        );
        // let query = &format!("SELECT rowid, Filename, duration, filepath FROM {}", TABLE);

        let rows = sqlx::query(query).fetch_all(&pool).await?;
        let _ = status_sender.send("Organizing Results".into()).await;

        let total = rows.len();
        let mut counter: usize = 0;

        // Use a parallel iterator to process the rows
        let processed_records: Vec<(String, FileRecord)> = rows
            .par_iter() // Use a parallel iterator
            .map(|row| {
                let mut file_record = FileRecord::new(row);
                file_record.update_metadata(row, &columns);
                // println! {"{:?}", file_record};
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

            counter += 1;

            if counter % 100 == 0 {
                let _ = progress_sender.send(Progress { counter, total }).await;
            }
        }

        let _ = status_sender.send("Finishing up".into()).await;

        let mut file_records = HashSet::new();
        for (root, mut records) in file_groups {
            if records.len() <= 1 {
                continue;
            }
            // println!("Unsorted: {:?}", records);
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
                // println!("Root Found");
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
                let order = order.read().unwrap();

                order.sort_vec(&mut records);
                // println!("Sorted: {:?}", records);
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
