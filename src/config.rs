use crate::assets::*;
use crate::processing::*;
// use egui::Order;
// use clipboard::{ClipboardContext, ClipboardProvider};
use eframe::egui::{self, RichText};
use rayon::prelude::*;
use serde::Deserialize;
use sqlx::sqlite::SqlitePool;
use std::collections::HashSet;
use std::fs::{self};
use std::hash::Hash;
use std::path::Path;
use tokio::sync::mpsc;

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
#[serde(default)]
pub struct Registration {
    pub name: String,
    pub email: String,
    pub key: String,
    #[serde(skip)]
    pub valid: Option<bool>,
}

impl Registration {
    pub fn validate(&mut self) {
        if generate_license_key(&self.name, &self.email) == self.key {
            self.valid = Some(true);
        } else {
            self.valid = Some(false);
        }
    }
    pub fn clear(&mut self) {
        self.name.clear();
        self.email.clear();
        self.key.clear();
        self.valid = Some(false);
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct AsyncTunnel<T> {
    #[serde(skip)]
    pub tx: mpsc::Sender<T>,
    #[serde(skip)]
    pub rx: mpsc::Receiver<T>,
}

impl<T> Default for AsyncTunnel<T> {
    fn default() -> Self {
        AsyncTunnel::new(1)
    }
}

impl<T> AsyncTunnel<T> {
    // Make `new` an associated function, and use `Self` for the return type
    pub fn new(channels: usize) -> AsyncTunnel<T> {
        let (tx, rx) = mpsc::channel(channels);
        AsyncTunnel { tx, rx }
    }

    // You might want to add methods to send and receive messages
    // pub async fn send(&self, item: T) -> Result<(), mpsc::error::SendError<T>> {
    //     if let Some(tx) = &self.tx {
    //         tx.send(item).await
    //     } else {
    //         Err(mpsc::error::SendError(item))
    //     }
    // }

    // pub async fn receive(&self) -> Option<T> {
    //     if let Some(rx) = &self.rx {
    //         rx.recv().await.ok()
    //     } else {
    //         None
    //     }
    // }
}

// #[derive(serde::Deserialize, serde::Serialize, Default)]
// #[serde(default)]
// pub struct FindReplaceConfig {
//     pub column: String,
//     pub find: String,
//     pub find_buf: String,
//     pub replace: String,
//     pub replace_buf: String,
//     pub search_replace_path: bool,
//     pub dirty: bool,
//     pub case_sensitive: bool,
//     #[serde(skip)]
//     pub find_io: AsyncTunnel<usize>,
//     #[serde(skip)]
//     pub replace_io: AsyncTunnel<HashSet<FileRecord>>,
//     #[serde(skip)]
//     pub replace_safety: bool,
//     #[serde(skip)]
//     pub count: usize,
// }

// impl FindReplaceConfig {
//     pub fn render(&mut self, ui: &mut egui::Ui, db: &Option<Database>, registered: bool) {
//         if let Some(db) = db {
//             if db.size == 0 {
//                 ui.heading("No Records in Database");
//                 return;
//             }
//             ui.heading(RichText::new("Find and Replace").strong());

//             empty_line(ui);
//             ui.horizontal(|ui| {
//                 // ui.add_space(68.0);
//                 let mut text = RichText::new("Case Sensitive").size(14.0);
//                 if self.case_sensitive {
//                     text = text.color(egui::Color32::from_rgb(255, 0, 0)).strong()
//                 }
//                 ui.checkbox(&mut self.case_sensitive, text);
//             });
//             empty_line(ui);
//             // ui.separator();
//             ui.horizontal(|ui| {
//                 ui.label("Find Text: ");
//                 ui.text_edit_singleline(&mut self.find);
//             });
//             ui.horizontal(|ui| {
//                 ui.label("Replace: ");
//                 ui.add_space(8.0);
//                 ui.text_edit_singleline(&mut self.replace);
//             });
//             ui.horizontal(|ui| {
//                 ui.label("in Column: ");
//                 ui.radio_value(&mut self.search_replace_path, true, "FilePath");
//                 ui.radio_value(&mut self.search_replace_path, false, "Other");
//                 let filtered_columns: Vec<_> = db
//                     .columns
//                     .iter()
//                     .filter(|col| {
//                         col.as_str() != "FilePath"
//                             && col.as_str() != "Pathname"
//                             && col.as_str() != "Filename"
//                     })
//                     .collect();
//                 egui::ComboBox::from_id_salt("find_column")
//                     .selected_text(&self.column)
//                     .show_ui(ui, |ui| {
//                         for item in filtered_columns {
//                             ui.selectable_value(&mut self.column, item.clone(), item);
//                         }
//                     });
//             });
//             empty_line(ui);
//             ui.separator();
//             empty_line(ui);
//             if !self.search_replace_path {
//                 ui.checkbox(&mut self.dirty, "Mark Records as Dirty?");
//                 ui.label("Dirty Records are audio files with metadata that is not embedded");
//                 empty_line(ui);
//                 ui.separator();
//                 empty_line(ui);
//             }

//             if self.find.is_empty() {
//                 return;
//             }
//             if ui
//                 .button(RichText::new("Find Records").size(16.0))
//                 .clicked()
//             {
//                 self.replace_safety = true;
//                 if self.search_replace_path {
//                     self.column = "FilePath".to_string()
//                 }
//                 let tx = self.find_io.tx.clone().expect("tx channel exists");
//                 let pool = db.pool.clone();
//                 let mut find = self.find.clone();
//                 let mut column = self.column.clone();
//                 let case_sensitive = self.case_sensitive;
//                 tokio::spawn(async move {
//                     println!("Inside Find Async");
//                     let count = smreplace_get(&pool, &mut find, &mut column, case_sensitive)
//                         .await
//                         .unwrap();
//                     let _ = tx.send(count).await;
//                 });
//             }
//             empty_line(ui);
//             if let Some(rx) = self.find_io.rx.as_mut() {
//                 if let Ok(count) = rx.try_recv() {
//                     self.count = count;
//                 }
//             }
//             if self.find != self.find_buf || self.replace != self.replace_buf {
//                 self.replace_safety = false;
//                 self.find_buf = self.find.clone();
//                 self.replace_buf = self.replace.clone();
//             }
//             if self.replace_safety {
//                 ui.label(
//                     RichText::new(format!(
//                         "Found {} records matching '{}' in {} of SM database: {}",
//                         self.count, self.find, self.column, db.name
//                     ))
//                     .strong(),
//                 );
//                 if self.count == 0 {
//                     return;
//                 }

//                 if registered {
//                     ui.label(
//                         RichText::new(
//                             "\nUNREGISTERED!\nPlease Register to Continue with Replacement",
//                         )
//                         .strong(),
//                     );
//                     return;
//                 }
//                 ui.label(format!("Replace with \"{}\" ?", self.replace));
//                 ui.horizontal(|ui| {
//                     ui.label("This is");
//                     ui.label(RichText::new("NOT").strong());
//                     ui.label("undoable");
//                 });
//                 if self.search_replace_path {
//                     ui.label("This does not alter your file system.");
//                 }
//                 ui.separator();
//                 ui.horizontal(|ui| {
//                     if ui
//                         .button(RichText::new("Replace Records").size(16.0))
//                         .clicked()
//                     {
//                         // let tx = self.find_tx.clone().expect("tx channel exists");
//                         let pool = db.pool.clone();
//                         let mut find = self.find.clone();
//                         let mut replace = self.replace.clone();
//                         let mut column = self.column.clone();
//                         let dirty = self.dirty;
//                         let filepath = self.search_replace_path;
//                         let case_sensitive = self.case_sensitive;
//                         tokio::spawn(async move {
//                             smreplace_process(
//                                 &pool,
//                                 &mut find,
//                                 &mut replace,
//                                 &mut column,
//                                 dirty,
//                                 filepath,
//                                 case_sensitive,
//                             )
//                             .await;
//                         });
//                         self.replace_safety = false;
//                     }
//                     if ui.button(RichText::new("Cancel").size(16.0)).clicked() {
//                         self.count = 0;
//                         self.replace_safety = false;
//                     }
//                 });
//             } else if self.count > 0 && registered {
//                 ui.label(format!("{} records replaced", self.count));
//             }
//         } else {
//             ui.heading(RichText::new("No Open Database").weak());
//         }
//     }
// }

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]

pub struct NodeConfig {
    pub enabled: bool,
    pub list: Vec<String>,
    pub selected: String,
    #[serde(skip)]
    pub status: String,
    #[serde(skip)]
    pub status_io: AsyncTunnel<String>,
    #[serde(skip)]
    pub records: HashSet<FileRecord>,
    #[serde(skip)]
    pub working: bool,

    #[serde(skip)]
    pub records_io: AsyncTunnel<HashSet<FileRecord>>,
    #[serde(skip)]
    pub progress_io: AsyncTunnel<ProgressMessage>,
    #[serde(skip)]
    pub progress: (f32, f32),
    #[serde(skip)]
    pub handle: Option<tokio::task::JoinHandle<()>>,
}

impl Clone for NodeConfig {
    fn clone(&self) -> Self {
        NodeConfig {
            enabled: self.enabled,
            list: self.list.clone(),
            selected: self.selected.clone(),
            status: self.status.clone(),
            status_io: AsyncTunnel::new(1),
            records: self.records.clone(),
            working: self.working,
            records_io: AsyncTunnel::new(1),
            progress_io: AsyncTunnel::new(32),
            progress: self.progress,
            handle: None, // JoinHandle does not implement Clone
        }
    }
}
impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            list: Vec::new(),
            selected: String::new(),
            status: String::new(),
            status_io: AsyncTunnel::new(1),
            records: HashSet::new(),
            working: false,
            records_io: AsyncTunnel::new(1),
            progress_io: AsyncTunnel::new(32),
            progress: (0.0, 0.0),
            handle: None,
        }
    }
}

impl NodeConfig {
    pub fn new(on: bool) -> Self {
        Self {
            enabled: on,
            list: Vec::new(),
            selected: String::new(),
            status: String::new(),
            status_io: AsyncTunnel::new(1),
            records: HashSet::new(),
            working: false,
            records_io: AsyncTunnel::new(1),
            progress_io: AsyncTunnel::new(32),
            progress: (0.0, 0.0),
            handle: None,
        }
    }
    pub fn new_option(on: bool, o: &str) -> Self {
        Self {
            enabled: on,
            list: Vec::new(),
            selected: o.to_string(),
            status: String::new(),
            status_io: AsyncTunnel::new(1),
            records: HashSet::new(),
            working: false,
            records_io: AsyncTunnel::new(1),
            progress_io: AsyncTunnel::new(32),
            progress: (0.0, 0.0),
            handle: None,
        }
    }
    pub fn abort(&mut self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
        self.handle = None;
        self.working = false;
        self.records.clear();
        self.status.clear();
        self.progress = (0.0, 0.0);
    }

    pub fn receive_hashset(&mut self) -> Option<HashSet<FileRecord>> {
        let rx = &mut self.records_io.rx;
        if let Ok(records) = rx.try_recv() {
            self.records = records.clone();
            self.handle = None;
            self.working = false;
            self.progress = (0.0, 0.0);
            self.status = format! {"Found {} duplicate records", self.records.len()};
            return Some(records);
        }

        None
    }
    pub fn receive_progress(&mut self) {
        let progress_receiver = &mut self.progress_io.rx;
        while let Ok(message) = progress_receiver.try_recv() {
            let ProgressMessage::Update(current, total) = message;
            self.progress = (current as f32, total as f32);
        }
    }
    pub fn receive_status(&mut self) {
        let status_receiver = &mut self.status_io.rx;
        while let Ok(message) = status_receiver.try_recv() {
            self.status = message;
        }
    }
    // pub fn clear_status(&mut self) {
    //     self.status.clear()
    // }

    // pub fn render<F>(&mut self, ui: &mut egui::Ui, text: &str, hint: &str, action: Option<F>)
    // where
    //     F: FnOnce(),
    // {
    //     ui.checkbox(&mut self.enabled, text)
    //         .on_hover_text_at_pointer(hint);
    //     if let Some(action) = action {
    //         action();
    //     }
    //     self.progress_bar(ui);
    // }

    pub fn progress_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.working {
                ui.spinner();
            } else {
                ui.add_space(24.0)
            }
            ui.label(RichText::new(&self.status).strong());
            if self.working {
                ui.label(format!(
                    "Progress: {} / {}",
                    self.progress.0, self.progress.1
                ));
            }
        });

        if self.working {
            ui.add(
                egui::ProgressBar::new(self.progress.0 / self.progress.1)
                    // .text("progress")
                    .desired_height(4.0),
            );
        } else {
            // ui.separator();
        }
        empty_line(ui);
    }
}

// #[derive(Default)]
pub struct Database {
    pub path: String,
    pub pool: Option<SqlitePool>,
    pub name: String,
    pub size: usize,
    pub columns: Vec<String>,
    pub file_extensions: Vec<String>,
    // pub io: AsyncTunnel<Database>,
    //     pub tx: Option<mpsc::Sender<Database>>,

    //     pub rx: Option<mpsc::Receiver<Database>>,
}

impl Database {
    pub async fn open(db_path: &str) -> Self {
        let db_pool = SqlitePool::connect(db_path)
            .await
            .expect("Pool did not open");
        let db_size = get_db_size(&db_pool).await.expect("get db size");
        let db_columns = get_columns(&db_pool).await.expect("get columns");

        Self {
            path: db_path.to_string(),
            pool: Some(db_pool),
            name: db_path
                .split('/')
                .last()
                .expect("Name From Pathname")
                .to_string(),
            size: db_size,
            columns: db_columns,
            file_extensions: Vec::new(),
            // io: AsyncTunnel::new(1),
        }
    }
    // pub async fn pool(&self) -> SqlitePool {
    //     SqlitePool::connect(&self.path)
    //         .await
    //         .expect("Pool did not open")
    // }

    pub fn get_extensions(&mut self, tx: mpsc::Sender<Vec<String>>) {
        let Some(pool) = self.pool.clone() else {
            return;
        };
        let tx = tx.clone();
        let _handle = tokio::spawn(async move {
            let results = get_audio_file_types(&pool).await;

            if (tx.send(results.expect("Tokio Results Error HashSet")).await).is_err() {
                eprintln!("Failed to send db while gathering extensions");
            }
        });
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct FileRecord {
    pub id: usize,
    pub filename: String,
    pub duration: String,
    pub path: String,
}

pub enum ProgressMessage {
    Update(usize, usize), // (current, total)
}

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy)]
pub enum Panel {
    Duplicates,
    NewDuplicates,
    Order,
    Tags,
    Find,
    KeyGen,
}

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy, Default)]
pub enum OrderOperator {
    Largest,
    Smallest,
    #[default]
    Contains,
    DoesNotContain,
    Is,
    IsNot,
    IsEmpty,
    IsNotEmpty,
}

impl EnumComboBox for OrderOperator {
    fn as_str(&self) -> &'static str {
        match self {
            OrderOperator::Largest => "Largest",
            OrderOperator::Smallest => "Smallest",
            OrderOperator::Is => "is",
            OrderOperator::IsNot => "is NOT",
            OrderOperator::Contains => "Contains",
            OrderOperator::DoesNotContain => "Does NOT Contain",
            OrderOperator::IsEmpty => "Is Empty",
            OrderOperator::IsNotEmpty => "Is NOT Empty",
        }
    }

    fn variants() -> &'static [OrderOperator] {
        &[
            OrderOperator::Largest,
            OrderOperator::Smallest,
            OrderOperator::Contains,
            OrderOperator::DoesNotContain,
            OrderOperator::Is,
            OrderOperator::IsNot,
            OrderOperator::IsEmpty,
            OrderOperator::IsNotEmpty,
        ]
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
// #[serde(default)]

pub struct PreservationLogic {
    pub friendly: String,
    pub sql: String,
}

// pub fn extract_sql(logics: &Vec<PreservationLogic>) -> Vec<String> {
//     logics.iter().map(|logic| logic.sql.clone()).collect()
// }

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy, Default)]
pub enum Delete {
    #[default]
    Trash,
    Permanent,
}

impl EnumComboBox for Delete {
    fn as_str(&self) -> &'static str {
        match self {
            Delete::Trash => "Move to Trash",
            Delete::Permanent => "Permanently Delete",
        }
    }
    fn variants() -> &'static [Delete] {
        &[Delete::Trash, Delete::Permanent]
    }
}
impl Delete {
    pub fn delete_files(&self, files: HashSet<&str>) -> Result<(), Box<dyn std::error::Error>> {
        println!("Removing Files");

        // Filter valid files directly and collect them into a Vec
        let valid_files: Vec<&str> = files
            .par_iter()
            .filter(|&&file| Path::new(file).exists())
            .cloned() // Convert &str to str for collection
            .collect();

        match self {
            Delete::Trash => {
                if !valid_files.is_empty() {
                    trash::delete_all(&valid_files).map_err(|e| {
                        eprintln!("Move to Trash Failed: {}", e);
                        e
                    })?;
                }
            }
            Delete::Permanent => {
                for file in valid_files {
                    fs::remove_file(file).map_err(|e| {
                        eprintln!("Failed to remove file {}: {}", file, e);
                        e
                    })?;
                }
            }
        }

        Ok(())
    }
}
