use egui::accesskit::Node;

use crate::prelude::*;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Remove {
    #[serde(skip)]
    config: NodeConfig,
    enabled: bool,
    safe: bool,
    dupes_db: bool,
    remove_files: bool,
    delete_action: Delete,
}

impl Remove {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
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
                enum_combo_box(ui, &mut self.delete_action);
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
            .iter()
            .map(|record| record.id as i64)
            .collect();

        if ids.is_empty() {
            return Ok(());
        }

        let total = self.config.records.len();
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
            let progress = std::cmp::min(current_count, total);

            let _ = self
                .config
                .progress_io
                .tx
                .send(ProgressMessage::Update(progress, total))
                .await;
            let _ = self
                .config
                .status_io
                .tx
                .send(format!("Processed {} / {}", progress, total).into())
                .await;
        }

        let _ = self
            .config
            .status_io
            .tx
            .send("Cleaning Up Database".into())
            .await;
        let _result = sqlx::query("VACUUM").execute(pool).await;

        println!("VACUUM done inside delete function");

        Ok(())
    }
}
