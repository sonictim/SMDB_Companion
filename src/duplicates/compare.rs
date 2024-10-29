use crate::prelude::*;



#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Compare {
    config: NodeConfig,
    #[serde(skip)]
    compare_db: Option<Database>,
    #[serde(skip)]
    cdb_io: AsyncTunnel<Database>,
}
impl Compare {
    pub fn enabled(&self) -> bool {
        self.config.enabled
    }
    pub fn render(&mut self, ui: &mut egui::Ui) {
        self.compare_db = self.cdb_io.recv();

        ui.horizontal(|ui| {
            let enabled = self.config.enabled || self.compare_db.is_some();
            let text = enabled_text("Compare against database: ", &enabled);
            ui.checkbox(&mut self.config.enabled, text)
                .on_hover_text_at_pointer
                    ("Filenames from Target Database found in Comparison Database will be Marked for Removal");
            
            if let Some(cdb) = &self.compare_db {
                if ui.selectable_label(false, &cdb.name).clicked() {
                    let tx = self.cdb_io.tx.clone();
                    tokio::spawn(async move {
                        let db = Database::open().await.unwrap();
                        let _ = tx.send(db).await;
                    });
                }
            }
            else {
                self.config.enabled = false;
                if ui.button("Select DB").clicked()  {
                    self.config.enabled = false;
                    let tx = self.cdb_io.tx.clone();
                    tokio::spawn(async move {
                        let db = Database::open().await.unwrap();
                        let _ = tx.send(db).await;
                    });
                }
            }
        });
    }
    pub async fn gather(&self,
        db: &Database,
        // target_pool: &SqlitePool,
        // compare_pool: &SqlitePool,
    ) -> Result<HashSet<FileRecord>, sqlx::Error> {
        let cdb = self.compare_db.as_ref().unwrap();
        let compare_records: HashSet<FileRecord> = cdb.fetch_all_filerecords().await?;
        let filenames_to_check = extract_filenames_set_from_records(&compare_records);
    
        let mut matching_records = db.fetch_all_filerecords().await?;
        println!("Comparing filenames between databases");
    
        matching_records.retain(|record| filenames_to_check.contains(&*record.filename));
    
        if matching_records.is_empty() {
            println!("NO OVERLAPPING FILE RECORDS FOUND!");
        } else {
            println!("Found {} overlapping file records.", matching_records.len());
        }
    
        Ok(matching_records.into_iter().collect())
    }


}


fn extract_filenames_set_from_records(file_records: &HashSet<FileRecord>) -> HashSet<Arc<str>> {
    file_records
        .iter()
        .map(|record| record.filename.clone())
        .collect()
}