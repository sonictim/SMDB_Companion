use crate::prelude::*;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Remove {
    #[serde(skip)]
    pub enabled: bool,
    #[serde(skip)]
    pub run: bool,
    #[serde(skip)]
    pub config: Node,
    safe: bool,
    pub dupes_db: bool,
    remove_files: bool,
    delete_action: Delete,
}

impl Default for Remove {
    fn default() -> Self {
        Self {
            enabled: false,
            run: false,
            config: Node::default(),
            safe: true,
            dupes_db: true,
            remove_files: false,
            delete_action: Delete::Trash,
        }
    }
}

impl Remove {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn render_options(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Remove Options").strong());
        let mut text = RichText::new("Create New Safety Database of Thinned Records");
        if !self.safe {
            text = text.strong().color(egui::Color32::from_rgb(255, 100, 100))
        }
        ui.checkbox(&mut self.safe, text);
        if !&self.safe {
            ui.horizontal(|ui| {
                ui.label(red_text("UNSAFE!"));
                ui.label(RichText::new("Will remove records from current database").strong());
            });
        }
        ui.checkbox(
            &mut self.dupes_db,
            "Create New Database of Duplicate Records",
        );
        ui.horizontal_wrapped(|ui| {
            let mut text = RichText::new("Remove Duplicate Files From Disk ");
            if self.remove_files {
                text = text
                    .strong()
                    .size(14.0)
                    .color(egui::Color32::from_rgb(255, 100, 100))
            }
            ui.checkbox(&mut self.remove_files, text);

            if self.remove_files {
                enum_combo_box(ui, &mut self.delete_action, "delete action");
                if self.remove_files && self.delete_action == Delete::Permanent {
                    ui.label(
                        RichText::new("UNSAFE!")
                            .color(egui::Color32::from_rgb(255, 0, 0))
                            .strong(),
                    );
                    ui.label(RichText::new("This is NOT undoable").strong());
                }
            }
        });
    }

    pub fn process(&mut self, db: &Database, registration: Option<bool>) {
        if registration == Some(false) {
            self.config.records.clear();
            self.config
                .status
                .set("Unregistered!\nPlease Register to Remove Duplicates".into());
            return;
        }

        let mut work_db_path: Option<String> = Some(db.path.clone());
        let mut duplicate_db_path: Option<String> = None;
        let records = self.config.records.get().clone();

        self.config.working = true;
        if self.safe {
            self.config.status.set("Creating Safety Database".into());
            let path = format!("{}_thinned.sqlite", &db.path.trim_end_matches(".sqlite"));
            let _result = fs::copy(&db.path, &path);
            work_db_path = Some(path);
        }
        if self.dupes_db {
            self.config
                .status
                .set("Creating Database of Duplicates".into());
            let path = format!("{}_dupes.sqlite", &db.path.trim_end_matches(".sqlite"));
            let _result = fs::copy(&db.path, &path);
            duplicate_db_path = Some(path);
        }

        let progress_sender = self.config.progress.tx.clone();
        let status_sender = self.config.status.tx.clone();
        self.config.wrap_async(move || {
            remove_duplicates_go(
                records,
                work_db_path,
                duplicate_db_path,
                progress_sender,
                status_sender,
            )
        });

        if self.remove_files {
            println!("Removing Files");
            let files: HashSet<&str> = self
                .config
                .records
                .get()
                .par_iter()
                .map(|record| &*record.path)
                .collect();

            let _ = self.delete_action.delete_files(files);
        }
        self.enabled = false;
        self.run = false;
    }

    pub async fn delete_file_records(
        &self,
        pool: &SqlitePool,
        // records: &HashSet<FileRecord>,
        // progress_sender: mpsc::Sender<ProgressMessage>,
        // status_sender: mpsc::Sender<Arc<str>>,
    ) -> Result<(), sqlx::Error> {
        const CHUNK_SIZE: usize = 12321;

        let ids: Vec<i64> = self
            .config
            .records
            .get()
            .iter()
            .map(|record| record.id as i64)
            .collect();

        if ids.is_empty() {
            return Ok(());
        }

        let total = self.config.records.get().len();
        let mut current_count = 0;

        for chunk in ids.chunks(CHUNK_SIZE) {
            let chunk: Vec<i64> = chunk.to_vec(); // Clone the chunk for the query

            // Construct the SQL query with placeholders for the chunk
            let query = format!(
                "DELETE FROM {} WHERE rowid IN ({})",
                TABLE,
                chunk.iter().map(|_| "?").collect::<Vec<&str>>().join(", ")
            );

            // Prepare the query with bound parameters
            let mut query = sqlx::query(&query);
            for id in &chunk {
                query = query.bind(*id);
            }

            // Execute the query
            match query.execute(pool).await {
                Ok(result) => {
                    let rows_deleted = result.rows_affected();
                    println!("Deleted {} records", rows_deleted);
                }
                Err(err) => {
                    eprintln!("Failed to delete records: {:?}", err);
                }
            }

            current_count += chunk.len();
            let counter = std::cmp::min(current_count, total);

            let _ = self
                .config
                .progress
                .tx
                .send(Progress { counter, total })
                .await;
            let _ = self
                .config
                .status
                .tx
                .send(format!("Processed {} / {}", counter, total).into())
                .await;
        }

        let _ = self
            .config
            .status
            .tx
            .send("Cleaning Up Database".into())
            .await;
        let _result = sqlx::query("VACUUM").execute(pool).await;

        println!("VACUUM done inside delete function");

        Ok(())
    }
}

pub async fn remove_duplicates_go(
    records: HashSet<FileRecord>,
    main_db_path: Option<String>,
    dupe_db_path: Option<String>,
    progress_sender: mpsc::Sender<Progress>,
    status_sender: mpsc::Sender<Arc<str>>,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let _ = status_sender.send("Performing Record Removal".into()).await;
    if let Some(main_path) = &main_db_path {
        let main_db = Database::init(main_path).await;
        let _ = main_db
            .delete_file_records(&records, progress_sender.clone(), status_sender.clone())
            .await;

        if let Some(path) = dupe_db_path {
            let mut dupes_db = Database::init(&path).await;
            let _ = dupes_db
                .keep_file_records(&records, progress_sender, status_sender)
                .await;
        }
    }
    Ok(records)
}

// pub async fn create_duplicates_db(
//     db: &Database,
//     // pool: &SqlitePool,
//     dupe_records_to_keep: &HashSet<FileRecord>,
//     progress_sender: mpsc::Sender<ProgressMessage>,
//     status_sender: mpsc::Sender<Arc<str>>,
// ) -> Result<(), sqlx::Error> {
//     println!("Generating Duplicates Only Database. This can take a while.");
//     let _ = status_sender
//         .send("Creating Duplicates Only Database. This can be slow.".into())
//         .await;

//     if let Ok(all_records) = db.fetch_all_filerecords().await {
//         // Use a parallel iterator to process records
//         let dupe_records_to_delete: HashSet<FileRecord> = all_records
//             .par_iter() // Parallel iterator
//             .filter(|record| !dupe_records_to_keep.contains(record)) // Filter out records to keep
//             .cloned() // Clone the records to create a new HashSet
//             .collect(); // Collect into a HashSet

//         let _result = delete_file_records(
//             pool,
//             &dupe_records_to_delete,
//             progress_sender,
//             status_sender,
//         )
//         .await;
//     }

//     Ok(())
// }
