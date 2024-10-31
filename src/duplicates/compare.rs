use crate::prelude::*;



#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Compare {
    pub config: NodeConfig,
    #[serde(skip)]
    compare_db: AsyncTunnel<Option<Database>>,

}
impl Compare {
    pub fn enabled(&self) -> bool {
        self.config.enabled
    }
    pub fn render_progress_bar(&mut self, ui: &mut egui::Ui) {
        self.config.render_progress_bar(ui);
    }
    pub fn render(&mut self, ui: &mut egui::Ui) {
        self.compare_db.recv2();
        let cdb = self.compare_db.get();

        ui.horizontal(|ui| {
            let enabled = self.config.enabled || self.compare_db.get().is_some();
            let text = enabled_text("Compare against database: ", &enabled);
            ui.checkbox(&mut self.config.enabled, text)
                .on_hover_text_at_pointer
                    ("Filenames from Target Database found in Comparison Database will be Marked for Removal");
            
            if let Some(cdb) = cdb {
                if ui.selectable_label(false, &cdb.name).clicked() {
                    let tx = self.compare_db.tx.clone();
                    tokio::spawn(async move {
                        let db = Database::open().await.unwrap();
                        let _ = tx.send(Some(db)).await;
                    });
                }
            }
            else {
                self.config.enabled = false;
                if ui.button("Select DB").clicked()  {
                    self.config.enabled = false;
                    let tx = self.compare_db.tx.clone();
                    tokio::spawn(async move {
                        let db = Database::open().await.unwrap();
                        let _ = tx.send(Some(db)).await;
                    });
                }
            }
        });
    }


    pub fn gather(&mut self, db: &Database) {
        let cdb = self.compare_db.get();
        if self.config.enabled && cdb.is_some() {
            if let Some(cdb) = &cdb {
                self.config.working = true;
                self.config.status = format!("Comparing against {}", cdb.name).into();
                let db = db.clone();
                let cdb = cdb.clone();
                let tx = self.config.records_io.tx.clone();
                let handle = tokio::spawn(async move {
                    println!("tokio spawn compare");
                    let results = Self::async_gather(&db, &cdb).await;
                    if (tx.send(results.expect("error on compare db")).await).is_err() {
                        eprintln!("Failed to send db");
                    }
                });
                self.config.handle = Some(handle);
                
            }
        }
        
    }

    pub async fn async_gather(
        db: &Database,
        cdb: &Database
 
    ) -> Result<HashSet<FileRecord>, sqlx::Error> {
       
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