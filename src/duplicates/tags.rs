
use crate::prelude::*;

use futures::stream::{self, StreamExt};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Tags {
    pub enabled: bool,
    pub list: SelectableList,
    #[serde(skip)]
    pub config: NodeC,
}

impl Default for Tags {
    fn default() -> Self {
        let mut default = Self {
            enabled: false,
            list: SelectableList::default(),
            config: NodeC::default(),
        };
        default.list.set(default_tags());
        default
    }
}

impl Tags {

    pub fn render_progress_bar(&mut self, ui: &mut egui::Ui) {
        self.config.render(ui);
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let enabled = !self.list().is_empty();
        let text = enabled_text(
            "Search for Records with AudioSuite Tags in Filename",
            &enabled,
        );
        ui.checkbox(&mut self.enabled, text)
            .on_hover_text_at_pointer(
                "Filenames with Common Protools AudioSuite Tags will be marked for removal",
            );


    }

    pub fn gather(&mut self, db: &Database) {
        if self.enabled {
            let progress_sender = self.config.progress.tx.clone();
            let status_sender = self.config.status.tx.clone();
            let pool = db.pool().unwrap();
            let tags = self.list.get().to_vec();
            self.config.wrap_async(
                move || Self::async_gather(pool, progress_sender, status_sender, tags),
            );
        }
    }



    pub async fn async_gather(
        pool: SqlitePool,
        progress_sender: mpsc::Sender<Progress>,
        status_sender: mpsc::Sender<Arc<str>>,
        tags: Vec<String>,
    ) -> Result<HashSet<FileRecord>, sqlx::Error> {
        let _ = status_sender.send("Searching for Filenames with Specified Tags".into()).await;
    
        let total = tags.len();
        // let mut count = 1;
        let mut file_records = HashSet::new();
        let max_concurrency = 10; // Adjust based on your system's capacity and connection pool size
    
        // Process each tag concurrently with a controlled level of concurrency
        let results = stream::iter(tags.into_iter())
            .enumerate()
            .map(|(count, tag)| {
                
                let pool = pool.clone();
                let progress_sender = progress_sender.clone();
                let status_sender = status_sender.clone();
                async move {
                    let query = format!(
                        "SELECT rowid, filename, duration, filepath FROM {TABLE} WHERE filename LIKE '%' || ? || '%'"
                        
                    );
    
                    
                    let result = sqlx::query(&query).bind(&tag).fetch_all(&pool).await; // Return the result (Result<Vec<sqlx::sqlite::SqliteRow>, sqlx::Error>)
                    let _ = status_sender.send((format!["Searching for tag: {}", &tag]).into()).await;
                    let _ = progress_sender
                        .send(Progress{count, total})
                        .await;
                    
                    result
                }
               
            })
            .buffer_unordered(max_concurrency) // Control the level of concurrency
            .collect::<Vec<Result<Vec<sqlx::sqlite::SqliteRow>, sqlx::Error>>>()
            .await;
    
        // Iterate over the results and insert the file records
        for result in results {
            match result {
                Ok(rows) => {
                    for row in rows {
                        file_records.insert(FileRecord::new(&row));
                    }
                }
                Err(err) => {
                    return Err(err); // Return early if an error occurs
                }
            }
    
        }
    
        println!("Found Tags");
        Ok(file_records)
    }


    pub fn render_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Tag Editor").strong());
        ui.label("Protools Audiosuite Tags use the following format:  -example_");
        ui.label("You can enter any string of text and if it is a match, the file will be marked for removal");
        empty_line(ui);
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.list.render(ui, 6, "tags editor", false);
            if !self.list().is_empty() {
                ui.separator();
            }
            empty_line(ui);
            self.list.add_text_input(ui);

            if !self.list.get().is_empty() && ui.button("Remove Selected Tags").clicked() {
                self.list.remove_selected();
            }
        });
    }
    pub fn list(&self) -> &[String] {
        self.list.get()
    }
}
