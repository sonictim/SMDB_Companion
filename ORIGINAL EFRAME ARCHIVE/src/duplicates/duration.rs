use crate::prelude::*;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Duration {
    pub enabled: bool,
    #[serde(skip)]
    pub config: Node,
    pub min_duration: String,
}

impl Default for Duration {
    fn default() -> Self {
        Self {
            enabled: false,
            config: Node::default(),
            min_duration: "0.05".to_string(),
        }
    }
}

impl NodeCommon for Duration {
    fn config(&mut self) -> &mut Node {
        &mut self.config
    }
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn render(&mut self, ui: &mut egui::Ui, _: &Database) {
        let text = "Search for Records who's Duration is less than:";
        ui.checkbox(&mut self.enabled, text)
            .on_hover_text_at_pointer(
                "Audio Files less than the specified duration will be marked for removal",
            );
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            let response = ui.text_edit_singleline(&mut self.min_duration);

            if response.lost_focus() {
                self.enabled = true;
                if self.min_duration.starts_with('.') {
                    self.min_duration = format!("0{}", self.min_duration);
                }
            }
            ui.label("seconds");
        });

        if self.min_duration.parse::<f64>().is_err() {
            ui.horizontal(|ui| {
                ui.add_space(24.0);

                ui.label("Invalid Duration");
            });
            self.enabled = false;
        }
    }

    fn process(&mut self, db: &Database, _: &HashSet<String>, _: Arc<RwLock<OrderPanel>>) {
        let progress_sender = self.config.progress.tx.clone();
        let status_sender = self.config.status.tx.clone();
        let pool = db.pool().unwrap();
        let duration = self.min_duration.parse::<f64>().unwrap_or(0.0);
        // let db = db.clone();
        self.config
            .wrap_async(move || Self::async_gather(pool, progress_sender, status_sender, duration));
    }
}

impl Duration {
    pub async fn async_gather(
        pool: SqlitePool,
        progress_sender: mpsc::Sender<Progress>,
        status_sender: mpsc::Sender<Arc<str>>,
        duration: f64,
    ) -> Result<HashSet<FileRecord>, sqlx::Error> {
        let _ = status_sender
            .send(
                format!(
                    "Searching for audio files shorter than {} seconds",
                    duration
                )
                .into(),
            )
            .await;

        let mut file_records = HashSet::new();

        // SQL query that converts MM:SS.mmm to seconds before comparison
        let query = format!(
            "SELECT rowid, filename, duration, filepath 
             FROM {TABLE} 
             WHERE (
                CAST(substr(duration, 1, instr(duration, ':') - 1) AS REAL) * 60 + 
                CAST(substr(duration, instr(duration, ':') + 1) AS REAL)
             ) < ?"
        );

        let results = sqlx::query(&query).bind(duration).fetch_all(&pool).await?;

        let total = results.len();

        for (counter, row) in results.iter().enumerate() {
            file_records.insert(FileRecord::new(row));
            if counter % 1231 == 0 {
                let _ = progress_sender.send(Progress { counter, total }).await;
            }
        }

        println!("Found files with duration < {}", duration);
        Ok(file_records)
    }

    // pub async fn async_gather(
    //     db: Database,
    //     progress_sender: mpsc::Sender<Progress>,
    //     status_sender: mpsc::Sender<Arc<str>>,
    //     duration: f64,
    // ) -> Result<HashSet<FileRecord>, sqlx::Error> {
    //     let _ = status_sender
    //         .send(
    //             format!(
    //                 "Searching for audio files shorter than {} seconds",
    //                 duration
    //             )
    //             .into(),
    //         )
    //         .await;

    //     let mut file_records: HashSet<FileRecord> = db.fetch_all_filerecords().await?;

    //     let total = file_records.len();
    //     let mut counter = 0;

    //     // Create a channel for progress updates
    //     let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    //     let progress_sender_clone = progress_sender.clone();

    //     // Spawn a task to handle progress updates
    //     tokio::spawn(async move {
    //         while let Some(count) = rx.recv().await {
    //             let _ = progress_sender_clone
    //                 .send(Progress {
    //                     counter: count,
    //                     total,
    //                 })
    //                 .await;
    //         }
    //     });

    //     file_records.retain(|record| {
    //         counter += 1;
    //         if counter % 100 == 0 {
    //             let _ = tx.try_send(counter);
    //         }

    //         if let Some((minutes, rest)) = record.duration.split_once(':') {
    //             if let (Ok(mins), Ok(secs)) = (minutes.parse::<f64>(), rest.parse::<f64>()) {
    //                 let total_seconds = (mins * 60.0) + secs;
    //                 total_seconds < duration
    //             } else {
    //                 false
    //             }
    //         } else {
    //             false
    //         }
    //     });
    //     println!("Found Short Files");
    //     Ok(file_records)
    // }

    // // let mut count = 1;
    // let mut file_records = HashSet::new();

    // let query = format!(
    //     "SELECT rowid, filename, duration, filepath
    //      FROM {TABLE}
    //      WHERE CAST(CASE
    //         WHEN duration GLOB '*[0-9].[0-9]*' THEN duration
    //         WHEN duration GLOB '*[0-9]' THEN duration || '.0'
    //         ELSE '0.0'
    //      END AS REAL) < ?"
    // );

    // let results = sqlx::query(&query).bind(duration).fetch_all(&pool).await; // Return the result (Result<Vec<sqlx::sqlite::SqliteRow>, sqlx::Error>)

    // if let Ok(results) = results {
    //     let total = results.len();

    //     for (counter, row) in results.iter().enumerate() {
    //         file_records.insert(FileRecord::new(row));
    //         if counter % 100 == 0 {
    //             let _ = progress_sender.send(Progress { counter, total }).await;
    //         }
    //     }
    // }

    // println!("Found Short Files");
    // Ok(file_records)
    // }
}
