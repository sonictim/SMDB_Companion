pub use eframe::egui::{self, RichText, Ui};
pub use rayon::prelude::*;
pub use serde::Deserialize;

pub use sqlx::sqlite::SqliteRow;
pub use sqlx::{sqlite::SqlitePool, Row};

pub use std::collections::{HashMap, HashSet};
pub use std::path::Path;
pub use std::result::Result;
pub use std::sync::{Arc, Mutex};
pub use tokio::sync::mpsc;

pub use dirs::home_dir;
pub use rfd::FileDialog;
pub use std::fs::{self};
pub use std::hash::Hash;

pub use crate::assets::*;
pub use crate::duplicates::*;
pub use crate::find_replace::FindPanel;

pub const TABLE: &str = "justinmetadata";

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct FileRecord {
    pub id: usize,
    pub filename: Arc<str>,
    pub duration: Arc<str>,
    pub path: Arc<str>,
}

impl FileRecord {
    pub fn new(row: &SqliteRow) -> Self {
        let id: u32 = row.get(0);
        let filename: &str = row.get(1);
        let duration = row.try_get(2).unwrap_or("");
        let path: &str = row.get(3);
        Self {
            id: id as usize,
            filename: filename.into(),
            duration: duration.into(),
            path: path.into(),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct AsyncTunnel<T: Default> {
    pub data: T,
    #[serde(skip)]
    pub tx: mpsc::Sender<T>,
    #[serde(skip)]
    pub rx: Arc<Mutex<mpsc::Receiver<T>>>,
    #[serde(skip)]
    pub waiting: bool,
}

impl<T: Default> Default for AsyncTunnel<T> {
    fn default() -> Self {
        AsyncTunnel::new(1)
    }
}

impl<T: Clone + std::default::Default> Clone for AsyncTunnel<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            tx: self.tx.clone(),
            rx: self.rx.clone(),
            waiting: self.waiting,
        }
    }
}

impl<T: Default> AsyncTunnel<T> {
    pub fn new(channels: usize) -> AsyncTunnel<T> {
        let (tx, rx) = mpsc::channel(channels);
        AsyncTunnel {
            data: T::default(),
            tx,
            rx: Arc::new(Mutex::new(rx)),
            waiting: false,
        }
    }
    pub fn clear(&mut self) {
        self.data = T::default();
        self.waiting = false;
    }
    pub fn get(&self) -> &T {
        &self.data
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.data
    }

    pub fn set(&mut self, data: T) {
        self.data = data;
    }

    pub async fn send(&mut self, item: T) -> Result<(), mpsc::error::SendError<T>> {
        self.waiting = true;
        self.tx.send(item).await
    }

    pub fn recv(&mut self) -> Option<T> {
        self.rx.lock().unwrap().try_recv().ok()
    }
    pub fn recv2(&mut self) -> bool {
        if let Ok(result) = self.rx.lock().unwrap().try_recv() {
            self.data = result;
            self.waiting = false;
            return true;
        }
        false
    }
}

#[derive(Clone, Default)]
pub struct Database {
    pub path: String,
    pool: Option<SqlitePool>,
    pub name: String,
    pub size: usize,
    pub columns: Vec<String>,
    // pub extensions: Arc<[String]>,
    pub extensions: AsyncTunnel<Vec<String>>,
}

impl Database {
    pub async fn get_pool(&self) -> SqlitePool {
        SqlitePool::connect(&self.path).await.unwrap()
    }

    pub async fn init(db_path: &str) -> Self {
        let pool = SqlitePool::connect(db_path)
            .await
            .expect("Pool did not open");
        let size = Database::get_size(&pool).await.expect("get db size");
        let columns = Database::get_columns(&pool).await.expect("get columns");

        let mut db = Self {
            path: db_path.to_string(),
            pool: Some(pool),
            name: db_path
                .split('/')
                .last()
                .expect("Name From Pathname")
                .to_string(),
            size,
            columns,
            // extensions: get_audio_file_types(&pool2).await.unwrap().into(),
            extensions: AsyncTunnel::default(),
            // io: AsyncTunnel::new(1),
        };
        let tx = db.extensions.tx.clone();
        db.fetch_extensions(tx);
        db
    }
    async fn get_columns(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
        // Query for table info using PRAGMA
        let columns = sqlx::query(&format!("PRAGMA table_info({});", TABLE))
            .fetch_all(pool)
            .await?
            .into_iter()
            .filter_map(|row| {
                let column_name: String = row.try_get("name").ok()?; // Extract "name" column
                if !column_name.starts_with('_') {
                    Some(column_name)
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();

        // Sort the column names
        let mut sorted_columns = columns;
        sorted_columns.sort();
        Ok(sorted_columns)
    }

    async fn get_size(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
        // let pool = self.pool.as_ref().unwrap();
        let count: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", TABLE))
            .fetch_one(pool)
            .await?;

        Ok(count.0 as usize)
    }

    pub fn fetch_extensions(&mut self, tx: mpsc::Sender<Vec<String>>) {
        println!("Fetching extensions");
        let Some(pool) = self.pool() else {
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

    pub fn receive_extensions(&mut self) {}

    pub fn pool(&self) -> Option<SqlitePool> {
        self.pool.clone()
    }

    pub async fn fetch(&self, query: &str) -> Vec<SqliteRow> {
        let pool = self.get_pool().await;
        sqlx::query(query).fetch_all(&pool).await.unwrap()
    }

    pub async fn fetch_filerecords(&self, query: &str) -> Result<HashSet<FileRecord>, sqlx::Error> {
        let mut file_records = HashSet::new();
        let rows = self.fetch(query).await;
        for row in rows {
            file_records.insert(FileRecord::new(&row));
        }
        Ok(file_records)
    }
    pub async fn fetch_all_filerecords(&self) -> Result<HashSet<FileRecord>, sqlx::Error> {
        println!("Gathering all records from database");
        self.fetch_filerecords(&format!(
            "SELECT rowid, filename, duration, filepath FROM {}",
            TABLE
        ))
        .await
    }

    pub async fn open() -> Option<Self> {
        let home_dir = home_dir();
        match home_dir {
            Some(home_dir) => {
                println!("Found SMDB dir");
                let db_dir = home_dir.join("Library/Application Support/SoundminerV6/Databases");
                if let Some(path) = FileDialog::new()
                    .add_filter("SQLite Database", &["sqlite"])
                    .set_directory(db_dir)
                    .pick_file()
                {
                    let db_path = path.display().to_string();
                    if db_path.ends_with(".sqlite") {
                        println!("Opening Database {}", db_path);
                        let db = Self::init(&db_path).await;
                        return Some(db);
                    }
                }
            }
            None => {
                println!("did not find SMDB dir");
                if let Some(path) = FileDialog::new()
                    .add_filter("SQLite Database", &["sqlite"])
                    .pick_file()
                {
                    let db_path = path.display().to_string();
                    if db_path.ends_with(".sqlite") {
                        println!("Opening Database {}", db_path);
                        let db = Self::init(&db_path).await;
                        return Some(db);
                    }
                }
            }
        }
        None
    }

    pub async fn keep_file_records(
        &mut self,
        dupe_records_to_keep: &HashSet<FileRecord>,
        progress_sender: mpsc::Sender<Progress>,
        status_sender: mpsc::Sender<Arc<str>>,
    ) -> Result<(), sqlx::Error> {
        println!("Generating Duplicates Only Database. This can take a while.");
        let _ = status_sender
            .send("Creating Duplicates Only Database. This can be slow.".into())
            .await;

        if let Ok(all_records) = self.fetch_all_filerecords().await {
            // Use a parallel iterator to process records
            let dupe_records_to_delete: HashSet<FileRecord> = all_records
                .par_iter() // Parallel iterator
                .filter(|record| !dupe_records_to_keep.contains(record)) // Filter out records to keep
                .cloned() // Clone the records to create a new HashSet
                .collect(); // Collect into a HashSet

            let _result = self
                .delete_file_records(&dupe_records_to_delete, progress_sender, status_sender)
                .await;
        }

        Ok(())
    }

    pub async fn delete_file_records(
        &self,
        // pool: &SqlitePool,
        records: &HashSet<FileRecord>,
        progress_sender: mpsc::Sender<Progress>,
        status_sender: mpsc::Sender<Arc<str>>,
    ) -> Result<(), sqlx::Error> {
        const CHUNK_SIZE: usize = 12321;

        let ids: Vec<i64> = records.iter().map(|record| record.id as i64).collect();

        if ids.is_empty() {
            return Ok(());
        }

        let total = records.len();
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
            match query.execute(self.pool.clone().as_ref().unwrap()).await {
                Ok(result) => {
                    let rows_deleted = result.rows_affected();
                    println!("Deleted {} records", rows_deleted);
                }
                Err(err) => {
                    eprintln!("Failed to delete records: {:?}", err);
                }
            }

            // Update the current count and send progress
            current_count += chunk.len();
            let count = std::cmp::min(current_count, total);

            let _ = progress_sender.send(Progress { count, total }).await;
            let _ = status_sender
                .send(format!("Processed {} / {}", count, total).into())
                .await;
        }

        // After all deletions, perform the cleanup
        let _ = status_sender.send("Cleaning Up Database".into()).await;
        let _result = sqlx::query("VACUUM")
            .execute(self.pool.clone().as_ref().unwrap())
            .await;

        println!("VACUUM done inside delete function");

        Ok(())
    }
}

pub async fn get_audio_file_types(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query("SELECT DISTINCT AudioFileType FROM justinmetadata")
        .fetch_all(pool)
        .await?;

    let audio_file_types: Vec<String> = rows
        .iter()
        .filter_map(|row| row.get::<Option<String>, _>("AudioFileType")) // Access the column directly
        .collect();

    Ok(audio_file_types)
}
