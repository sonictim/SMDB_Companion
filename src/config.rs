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

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct AsyncTunnel<T> {
    #[serde(skip)]
    pub tx: mpsc::Sender<T>,
    #[serde(skip)]
    pub rx: mpsc::Receiver<T>,
    #[serde(skip)]
    pub waiting: bool,
}

impl<T> Default for AsyncTunnel<T> {
    fn default() -> Self {
        AsyncTunnel::new(1)
    }
}

impl<T> AsyncTunnel<T> {
    pub fn new(channels: usize) -> AsyncTunnel<T> {
        let (tx, rx) = mpsc::channel(channels);
        AsyncTunnel {
            tx,
            rx,
            waiting: false,
        }
    }

    // You might want to add methods to send and receive messages
    // pub async fn asend(&mut self, item: T) -> Result<(), mpsc::error::SendError<T>> {
    //     self.waiting = true;
    //     self.tx.send(item).await
    //     // if let Some(tx) = &self.tx {
    //     //     tx.send(item).await
    //     // } else {
    //     //     Err(mpsc::error::SendError(item))
    //     // }
    // }

    // pub async fn areceive(&self) -> Option<T> {
    //     if let rx = &self.rx {
    //         rx.recv().await.ok()
    //     } else {
    //         None
    //     }
    // }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]

pub struct NodeConfig {
    pub enabled: bool,
    // pub list: Vec<String>,
    // pub selected: String,
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
            // list: self.list.clone(),
            // selected: self.selected.clone(),
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
            // list: Vec::new(),
            // selected: String::new(),
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
            // list: Vec::new(),
            // selected: String::new(),
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
pub struct PreservationLogic {
    pub friendly: String,
    pub sql: String,
}

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
