use crate::prelude::*;

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
        tokio::spawn(async move {
            gather(db, progress_sender, status_sender).await;
            // let _ = tx.send(db).await;
        });
        println!("Waveform not implemented yet");
    }
}

async fn gather(
    db: Database,
    progress_sender: mpsc::Sender<Progress>,
    status_sender: mpsc::Sender<Arc<str>>,
) {
    println!("Searching for Duplicate Waveforms");
    let mut counter = 1;
    let mut wavemaps: HashMap<String, Vec<FileRecord>> = HashMap::new();
    let records = db.fetch_all_filerecords().await.unwrap();
    let total = records.len();
    for record in records {
        wavemaps
            .entry(get_wavemap(&record))
            .or_default()
            .push(record);
        let _ = status_sender
            .send(format!("Processed File {counter} / {total}").into())
            .await;
        let _ = progress_sender
            .send(Progress {
                count: counter,
                total,
            })
            .await;
        counter += 1;
    }
    println!("counting total duplicates found");
    counter = 0;
    for (_, records) in wavemaps {
        if records.len() > 1 {
            counter += 1;
        }
    }
    println!("Found {counter} waveform duplicates");
}

fn get_wavemap(record: &FileRecord) -> String {
    "Jimbo".to_string()
}
