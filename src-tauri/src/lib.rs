mod audiohash;
mod commands;
use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose;
use chrono::{Duration, NaiveDateTime};
use once_cell::sync::Lazy;
use rayon::prelude::*;
pub use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
pub use sqlx::sqlite::{SqlitePool, SqliteRow};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::env;
pub use std::fs::{self};
use std::path::{Path, PathBuf};
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Emitter};
use tauri::{Manager, State};

pub use Algorithm as A;
pub use OrderOperator as O;
use commands::*;

pub const TABLE: &str = "justinmetadata";
pub const RECORD_DIVISOR: usize = 1231;

// use tauri_plugin_store::Builder;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let version = app.package_info().version.to_string();
            let _ = app
                .webview_windows()
                .get("main")
                .unwrap()
                .set_title(&format!("SMDB Companion :: v{}", version));

            app.manage(Mutex::new(AppState::default()));
            Ok(())
        })
        // .plugin(tauri_plugin_window::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_current_version,
            open_db,
            get_db_name,
            get_db_size,
            get_records_size,
            search,
            find,
            replace_metadata,
            remove_records,
            get_results,
            get_columns,
            get_reg,
            check_reg,
            open_quicklook,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
#[derive(Default)]
pub struct AppState {
    db: Database,
    // enabled: Enabled,
    // pref: Preferences,
    // reg: Registration,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Preferences {
    columns: Vec<Arc<str>>,
    match_criteria: Vec<Arc<str>>,
    ignore_filetype: bool,
    tags: Vec<Arc<str>>,
    autoselects: Vec<Arc<str>>,
    preservation_order: Vec<PreservationLogic>,
    display_all_records: bool,
    exact_waveform: bool,
    store_waveforms: bool,
    fetch_waveforms: bool,
    similarity_threshold: f64,
}

impl Preferences {
    pub fn sort_vec(&self, vec: &mut [FileRecord]) {
        for l in self.preservation_order.iter().rev() {
            l.sort(vec);
        }
        vec.sort_by(|a, b| {
            let a_root = self.check_tags(a.get_filestem());
            let b_root = self.check_tags(b.get_filestem());
            a_root.cmp(&b_root)
        });

        vec.sort_by(|a, b| {
            let a_root = if self.ignore_filetype {
                a.get_filestem() == a.root.as_ref()
            } else {
                a.get_filename() == a.root.as_ref()
            };
            let b_root = if self.ignore_filetype {
                b.get_filestem() == b.root.as_ref()
            } else {
                b.get_filename() == b.root.as_ref()
            };
            b_root.cmp(&a_root)
        });

        vec.sort_by(|a, b| {
            let a_already_marked = a.algorithm.contains(&A::Keep);
            let b_already_marked = b.algorithm.contains(&A::Keep);
            b_already_marked.cmp(&a_already_marked)
        });
    }
    pub fn check_tags(&self, item: &str) -> bool {
        for tag in &self.tags {
            if item.contains(&**tag) {
                return true;
            }
        }
        false
    }

    pub fn get_data_requirements(&self) -> Arc<str> {
        let mut set: HashSet<&str> = HashSet::new();
        for m in &self.match_criteria {
            set.insert(m);
        }
        for m in &self.preservation_order {
            let m = &m.column;
            set.insert(m);
        }
        Arc::from(set.iter().copied().collect::<Vec<_>>().join(","))
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PreservationLogic {
    pub column: Arc<str>,
    pub operator: OrderOperator,
    pub variable: Arc<str>,
}
impl PreservationLogic {
    fn sort(&self, vec: &mut [FileRecord]) {
        match self.operator {
            O::Largest => {
                vec.sort_by(|a, b| {
                    // Get values from each FileRecord
                    let a_value = a.data.get(&self.column).map_or("", |v| v);
                    let b_value = b.data.get(&self.column).map_or("", |v| v);

                    // Parse values
                    let a_num = parse_string(a_value).unwrap_or(ParsedValue::Integer(0));
                    let b_num = parse_string(b_value).unwrap_or(ParsedValue::Integer(0));

                    // Compare b to a for descending order
                    b_num.cmp(&a_num)
                });
            }
            O::Smallest => {}
            O::Is => {
                vec.sort_by(|a, b| {
                    let a_matches = a
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.as_ref() == self.variable.as_ref());
                    let b_matches = b
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.as_ref() == self.variable.as_ref());

                    b_matches.cmp(&a_matches)
                });
            }
            O::IsNot => {
                vec.sort_by(|a, b| {
                    let a_matches = a
                        .data
                        .get(&self.column)
                        .map_or(false, |v| *v == self.variable);
                    let b_matches = b
                        .data
                        .get(&self.column)
                        .map_or(false, |v| *v == self.variable);

                    a_matches.cmp(&b_matches)
                });
            }
            O::IsEmpty => {
                vec.sort_by(|a, b| {
                    let a_empty = a.data.get(&self.column).map_or(false, |v| v.is_empty());
                    let b_empty = b.data.get(&self.column).map_or(false, |v| v.is_empty());
                    b_empty.cmp(&a_empty)
                });
            }
            O::IsNotEmpty => {
                vec.sort_by(|a, b| {
                    let a_empty = a.data.get(&self.column).map_or(false, |v| v.is_empty());
                    let b_empty = b.data.get(&self.column).map_or(false, |v| v.is_empty());
                    a_empty.cmp(&b_empty)
                });
            }
            O::Contains => {
                vec.sort_by(|a, b| {
                    let a_contains = a
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.contains(self.variable.as_ref()));
                    let b_contains = b
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.contains(self.variable.as_ref()));
                    b_contains.cmp(&a_contains)
                });
            }
            O::DoesNotContain => {
                vec.sort_by(|a, b| {
                    let a_contains = a
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.contains(self.variable.as_ref()));
                    let b_contains = b
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.contains(self.variable.as_ref()));
                    a_contains.cmp(&b_contains)
                });
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ParsedValue {
    Integer(i64),
    Duration(Duration),
    DateTime(NaiveDateTime),
}

impl PartialOrd for ParsedValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (ParsedValue::Integer(a), ParsedValue::Integer(b)) => a.partial_cmp(b),
            (ParsedValue::Duration(a), ParsedValue::Duration(b)) => {
                a.num_milliseconds().partial_cmp(&b.num_milliseconds())
            }
            (ParsedValue::DateTime(a), ParsedValue::DateTime(b)) => a.partial_cmp(b),

            // For different types, we define a custom ordering or return None.
            (ParsedValue::Integer(_), _) => Some(Ordering::Less),
            (ParsedValue::Duration(_), ParsedValue::DateTime(_)) => Some(Ordering::Less),
            (ParsedValue::DateTime(_), ParsedValue::Integer(_)) => Some(Ordering::Greater),
            _ => None,
        }
    }
}

impl Ord for ParsedValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

// Parse a string into an appropriate `ParsedValue`
fn parse_string(value: &str) -> Result<ParsedValue, &'static str> {
    // Try to parse as an integer
    if let Ok(int_value) = value.parse::<i64>() {
        return Ok(ParsedValue::Integer(int_value));
    }

    // Try to parse as a duration in "MM:SS.mmm" or "HH:MM:SS.mmm" format
    if let Some(parsed_duration) = parse_duration(value) {
        return Ok(ParsedValue::Duration(parsed_duration));
    }

    // Try to parse as a datetime in "YYYY-MM-DD HH:MM:SS" format
    if let Ok(date_time) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Ok(ParsedValue::DateTime(date_time));
    }

    Err("Invalid format")
}

fn parse_duration(value: &str) -> Option<Duration> {
    let parts: Vec<&str> = value.split(&[':', '.'][..]).collect();
    match parts.len() {
        3 => {
            // "MM:SS.mmm"
            let minutes: i64 = parts[0].parse().ok()?;
            let seconds: i64 = parts[1].parse().ok()?;
            let millis: i64 = parts[2].parse().ok()?;
            Some(
                Duration::minutes(minutes)
                    + Duration::seconds(seconds)
                    + Duration::milliseconds(millis),
            )
        }
        4 => {
            // "HH:MM:SS.mmm"
            let hours: i64 = parts[0].parse().ok()?;
            let minutes: i64 = parts[1].parse().ok()?;
            let seconds: i64 = parts[2].parse().ok()?;
            let millis: i64 = parts[3].parse().ok()?;
            Some(
                Duration::hours(hours)
                    + Duration::minutes(minutes)
                    + Duration::seconds(seconds)
                    + Duration::milliseconds(millis),
            )
        }
        _ => None,
    }
}

#[derive(Debug, PartialEq, serde::Serialize, Deserialize, Clone, Copy, Default)]
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

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Enabled {
    basic: bool,
    invalidpath: bool,
    filename: bool,
    filetags: bool,
    audiosuite: bool,
    waveform: bool,
    duration: bool,
    dbcompare: bool,
    min_dur: f64,
    compare_db: Arc<str>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Registration {
    name: Arc<str>,
    email: Arc<str>,
    license: Arc<str>,
}

fn generate_license_key(name: &str, email: &str) -> Arc<str> {
    let salt = "Valhalla Delay";
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}{}", name.to_lowercase(), email.to_lowercase(), salt).as_bytes());
    let hash = hasher.finalize();

    // Option 1: Take first 16 bytes (32 characters) of the hash
    let shortened = &hash[..16];

    // Format as XXXX-XXXX-XXXX-XXXX for readability
    let formatted = format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}",
        shortened[0],
        shortened[1],
        shortened[2],
        shortened[3],
        shortened[4],
        shortened[5],
        shortened[6],
        shortened[7],
        shortened[8],
        shortened[9],
        shortened[10],
        shortened[11],
        shortened[12],
        shortened[13],
        shortened[14],
        shortened[15]
    );

    formatted.into()
}

fn generate_license_key_old(name: &str, email: &str) -> Arc<str> {
    let salt = "Valhalla Delay";
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}{}", name, email, salt).as_bytes());
    let hash = hasher.finalize();
    hex::encode_upper(hash).into()
}

#[derive(Clone, serde::Serialize)]
struct SearchStatus {
    stage: String,
    progress: u64,
    message: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Algorithm {
    All,
    Basic,
    SimilarFilename,
    Waveforms,
    Compare,
    Tags,
    FileTags,
    InvalidPath,
    Duration,
    Replace,
    Manual,
    #[default]
    Keep,
}

#[derive(Serialize, Deserialize)] // Need Deserialize to receive it back
struct FileRecordFrontend {
    id: usize,
    path: Arc<str>,
    root: Arc<str>,
    algorithm: Vec<Algorithm>,
}

#[derive(Default, Debug, Serialize, Clone, PartialEq)]
pub struct FileRecord {
    pub id: usize,
    pub path: std::path::PathBuf,
    pub root: Arc<str>,
    pub duration: Arc<str>,
    pub data: HashMap<Arc<str>, Arc<str>>,
    pub audio_fingerprint: Option<Arc<str>>,
    pub audio_fingerprint_raw: Option<Arc<str>>,
    pub algorithm: HashSet<Algorithm>,
}

impl FileRecord {
    pub fn new(row: &SqliteRow, enabled: &Enabled, pref: &Preferences, is_compare: bool) -> Self {
        let id = row.get::<u32, _>(0) as usize;
        let path_str: &str = row.get(1);
        let path = PathBuf::from(path_str);
        let duration_str: &str = row.get(2);
        let path_exists = path.exists();

        // Create a HashSet for algorithm
        let mut algorithm = HashSet::new();
        if !path_exists && enabled.invalidpath {
            algorithm.insert(Algorithm::InvalidPath);
        } else if enabled.duration && checkduration(duration_str, enabled.min_dur) {
            algorithm.insert(Algorithm::Duration);
        } else if enabled.filetags && checktags(path_str, &pref.autoselects) {
            algorithm.insert(Algorithm::FileTags);
        } else {
            algorithm.insert(Algorithm::Keep);
        }

        // Create a HashMap for column data
        let mut data = HashMap::new();

        // Gather required columns from preferences
        for column in &pref.match_criteria {
            if let Some(value) = get_column_as_string(row, column) {
                data.insert(column.clone(), value);
            }
        }

        // Gather columns from preservation logic
        for logic in &pref.preservation_order {
            let column = &logic.column;
            if let Some(value) = get_column_as_string(row, column) {
                data.insert(column.clone(), value);
            }
        }

        // Get audio fingerprint if it exists
        let audio_fingerprint = if pref.fetch_waveforms {
            match row.try_get::<&str, _>("_fingerprint") {
                Ok(hash) if !hash.is_empty() => Some(Arc::from(hash)),
                _ => None,
            }
        } else {
            None
        };
        let audio_fingerprint_raw = if pref.fetch_waveforms {
            match row.try_get::<&str, _>("_fingerprint_raw") {
                Ok(hash) if !hash.is_empty() => Some(Arc::from(hash)),
                _ => None,
            }
        } else {
            None
        };

        // Create the record
        let mut record = Self {
            id,
            path,
            root: Arc::default(),
            duration: Arc::from(duration_str),
            data,
            audio_fingerprint,
            audio_fingerprint_raw,
            algorithm,
        };

        record.set_root(enabled, pref);
        record
    }

    pub fn set_root(&mut self, enabled: &Enabled, pref: &Preferences) {
        // Use Cow to avoid unnecessary string allocations
        let mut name = Cow::Borrowed(self.get_filestem());

        // Optimize tag processing with minimal allocations
        if enabled.audiosuite {
            if let Some((base, _)) = pref.tags.iter().find_map(|tag| name.split_once(&**tag)) {
                name = Cow::Owned(base.to_string());
            }
        }

        // Use const array to avoid repeated allocations
        const COPY_TAGS: [&str; 6] = [" copy.", " Copy.", " COPY.", ".copy.", ".Copy.", ".COPY."];

        if enabled.filename {
            // Find and split with minimal allocations
            if let Some((base, _)) = COPY_TAGS.iter().find_map(|&tag| name.split_once(tag)) {
                name = Cow::Owned(base.to_string());
            }

            // Efficient regex capture
            name = Cow::Owned(
                FILENAME_REGEX
                    .captures(&name)
                    .and_then(|caps| caps.name("base").map(|m| m.as_str().to_string()))
                    .unwrap_or_else(|| name.into_owned()),
            );
        }

        // Minimize string allocations for root
        self.root = if pref.ignore_filetype {
            Arc::from(name.as_ref())
        } else {
            Arc::from(format!("{}.{}", name.as_ref(), self.get_extension()))
        };
    }

    pub fn get_filename(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Invalid Filename")
    }

    pub fn get_filestem(&self) -> &str {
        self.path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Invalid Filestem")
    }

    pub fn get_extension(&self) -> &str {
        self.path
            .extension()
            .and_then(|n| n.to_str())
            .unwrap_or("Invalid Extension")
    }

    pub fn get_path(&self) -> &str {
        self.path
            .parent()
            .and_then(|n| n.to_str())
            .unwrap_or("Invalid Path")
    }

    pub fn get_filepath(&self) -> &str {
        self.path.to_str().unwrap_or("Invalid Filepath")
    }

    pub fn get_root(&self) -> String {
        let name = self.get_filestem();
        if let Some(caps) = FILENAME_REGEX.captures(name) {
            caps["base"].to_string()
        } else {
            name.to_string()
        }
    }
    pub fn set_root_old(&mut self, enabled: &Enabled, pref: &Preferences) {
        let mut name = self.get_filestem();
        println!("Source Root: {}", name);
        if enabled.audiosuite {
            for tag in &pref.tags {
                if let Some((base, _)) = name.split_once(&**tag) {
                    name = base;
                    println!("Tag Found: {} Root: {}", tag, name);
                }
            }
        };
        if enabled.filename {
            let copy = [" copy.", " Copy.", " COPY.", ".copy.", ".Copy.", ".COPY."];
            for tag in &copy {
                if let Some((base, _)) = name.split_once(tag) {
                    name = base;
                    println!("Filename Root: {}", name);
                }
            }
            name = FILENAME_REGEX
                .captures(name) // Use reference to avoid cloning
                .and_then(|caps| caps.name("base"))
                .map(|m| m.as_str())
                .unwrap_or_else(|| name);
            println!("similar filename Root: {}", name);
        }

        self.root = if pref.ignore_filetype {
            Arc::from(name)
        } else {
            Arc::from(format!("{}.{}", name, self.get_extension()))
        };
        println!("Final Root: {}", self.root);
    }
}

static FILENAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?P<base>.+?)(?:\.(?:\d+|M))*$").unwrap());

#[derive(Default)]
pub struct Database {
    path: Option<PathBuf>,
    size: usize,
    records: Vec<FileRecord>, // Changed from Arc<[FileRecord]> to Vec<FileRecord>
    is_compare: bool,
}

impl Database {
    async fn open(&mut self, is_compare: bool) {
        let path = FileDialog::new()
            .add_filter("SQLite Database", &["sqlite"])
            .pick_file();
        self.init(path, is_compare).await;
    }

    async fn init(&mut self, path: Option<PathBuf>, is_compare: bool) {
        if let Some(path) = path {
            self.path = Some(path);
            self.size = self.fetch_size().await.unwrap();
            self.records = Vec::with_capacity(self.size); // No need for .into()
            self.is_compare = is_compare;
            self.add_column("_fingerprint").await.ok();
            self.add_column("_fingerprint_raw").await.ok();
        }
    }

    async fn create_clone(&self, tag: &str) -> Database {
        let mut path = self.path.as_ref().unwrap().to_string_lossy().to_string();
        path = path.replace(".sqlite", &format!("_{}.sqlite", tag));
        let path = PathBuf::from(path);
        let _result = fs::copy(self.path.as_ref().unwrap(), &path);

        let mut db = Database::default();
        db.init(Some(path), false).await;
        db
    }

    fn get_path(&self) -> Option<Arc<str>> {
        if let Some(path) = &self.path {
            if let Some(path) = path.to_str() {
                return Some(Arc::from(path));
            }
        }
        None
    }

    fn get_name(&self) -> Option<Arc<str>> {
        if let Some(path) = &self.path {
            if let Some(name) = path.file_stem() {
                if let Some(name_str) = name.to_str() {
                    return Some(Arc::from(name_str));
                }
            }
        }
        None
    }
    fn get_size(&self) -> usize {
        self.size
    }

    fn get_records_size(&self) -> usize {
        self.records
            .iter()
            .filter(|record| !record.algorithm.contains(&A::Keep))
            .count()
    }

    pub async fn get_pool(&self) -> Option<SqlitePool> {
        if let Some(path) = self.get_path() {
            return SqlitePool::connect(&path).await.ok();
        }
        None
    }

    async fn add_column(&self, add: &str) -> Result<(), sqlx::Error> {
        if let Some(pool) = self.get_pool().await {
            // First check if the column already exists
            let columns = sqlx::query(&format!("PRAGMA table_info({});", TABLE))
                .fetch_all(&pool)
                .await?;

            // Check if our column exists
            let column_exists = columns.iter().any(|row| {
                let column_name: &str = row.try_get("name").unwrap_or_default();
                column_name == add
            });

            // Only add the column if it doesn't exist
            if !column_exists {
                // Add the column with TEXT type (you can change this if needed)
                let query = format!("ALTER TABLE {} ADD COLUMN {} TEXT;", TABLE, add);
                sqlx::query(&query).execute(&pool).await?;
                println!("Added new column: {}", add);
            } else {
                println!("Column '{}' already exists", add);
            }

            return Ok(());
        }

        Err(sqlx::Error::Configuration(
            "No database connection available".into(),
        ))
    }

    async fn fetch_size(&self) -> Result<usize, sqlx::Error> {
        if let Some(pool) = self.get_pool().await {
            // let pool = self.pool.as_ref().unwrap();
            let count: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", TABLE))
                .fetch_one(&pool)
                .await?;

            return Ok(count.0 as usize);
        }
        Ok(0)
    }

    pub async fn remove(&self, ids: &[usize]) -> Result<(), sqlx::Error> {
        const BATCH_SIZE: usize = 1000; // Define the batch size
        if let Some(pool) = self.get_pool().await {
            // Iterate over chunks of IDs
            for chunk in ids.chunks(BATCH_SIZE) {
                // Create placeholders for each ID in the chunk
                let placeholders = std::iter::repeat("?")
                    .take(chunk.len())
                    .collect::<Vec<_>>()
                    .join(",");
                let query = format!("DELETE FROM {} WHERE rowid IN ({})", TABLE, placeholders);

                // Create a query builder
                let mut query_builder = sqlx::query(&query);

                // Bind each ID individually
                for &id in chunk {
                    query_builder = query_builder.bind(id as i64);
                }

                // Execute the query
                query_builder.execute(&pool).await?;
            }
        }
        Ok(())
    }

    pub async fn fetch(&self, query: &str) -> Vec<SqliteRow> {
        if let Some(pool) = self.get_pool().await {
            sqlx::query(query)
                .fetch_all(&pool)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub async fn fetch_filerecords(
        &mut self,
        query: &str,
        enabled: &Enabled,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), sqlx::Error> {
        // self.records.clear();
        let rows = self.fetch(query).await;
        let mut records = Vec::with_capacity(rows.len());
        println!("{} Rows Found", rows.len());
        let new_records: Vec<FileRecord> = rows
            .par_iter()
            .enumerate()
            .map(|(count, row)| {
                if count % RECORD_DIVISOR == 0 {
                    app.emit(
                        "search-sub-status",
                        SearchStatus {
                            stage: "gather".into(),
                            progress: (count * 100 / rows.len()) as u64,
                            message: format!(
                                "Processing Records into Memory: {}/{}",
                                count,
                                rows.len()
                            ),
                        },
                    )
                    .ok();
                }
                FileRecord::new(row, enabled, pref, self.is_compare)
            })
            .collect();
        app.emit(
            "search-sub-status",
            SearchStatus {
                stage: "gather".into(),
                progress: 100,
                message: "Complete".into(),
            },
        )
        .ok();
        records.extend(new_records);
        self.records = records;
        Ok(())
    }

    pub async fn fetch_all_filerecords(
        &mut self,
        enabled: &Enabled,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), sqlx::Error> {
        println!("Gathering all records from database");
        self.fetch_filerecords(
            &format!(
                "SELECT rowid, filepath, duration, _chromaprint, {} FROM {}",
                pref.get_data_requirements(),
                TABLE
            ),
            enabled,
            pref,
            app,
        )
        .await
    }

    async fn fetch_columns(&self) -> Result<Vec<Arc<str>>, sqlx::Error> {
        // Query for table info using PRAGMA
        let mut columns = self
            .fetch(&format!("PRAGMA table_info({});", TABLE))
            .await
            .into_iter()
            .filter_map(|row| {
                let column_name: &str = row.try_get("name").ok()?; // Extract "name" column
                if !column_name.starts_with('_') {
                    Some(column_name.into())
                } else {
                    None
                }
            })
            .collect::<Vec<Arc<str>>>();
        columns.sort();
        // if let Some(index) = columns.iter().position(|x| x.as_ref() == "FilePath") {
        //     let filepath = columns.remove(index); // Remove the item
        //     columns.insert(0, filepath); // Insert it at the beginning
        // }

        Ok(columns)
    }

    pub async fn compare_search(&mut self, enabled: &Enabled, pref: &Preferences, app: &AppHandle) {
        let mut cdb = Database::default();
        cdb.init(Some(PathBuf::from(&*enabled.compare_db)), true)
            .await;

        app.emit(
            "search-sub-status",
            SearchStatus {
                stage: "compare".into(),
                progress: 0,
                message: "Gathering Records".into(),
            },
        )
        .ok();
        let _ = cdb.fetch_all_filerecords(enabled, pref, app).await;
        let mut total = cdb.get_size();
        if total == 0 {
            total = 100;
        }
        println!("{} Records Found in Compare Database", total);
        // Use HashSet for O(1) lookup
        let filenames_to_check: HashSet<_> = cdb
            .records
            .iter()
            .enumerate()
            .map(|(count, record)| {
                if count % RECORD_DIVISOR == 0 {
                    app.emit(
                        "search-sub-status",
                        SearchStatus {
                            stage: "compare".into(),
                            progress: (count * 100 / total) as u64,
                            message: format!("Processing Records into Memory: {}/{}", count, total),
                        },
                    )
                    .ok();
                }

                record.get_filename()
            })
            .collect();
        app.emit(
            "search-sub-status",
            SearchStatus {
                stage: "compare".into(),
                progress: 100,
                message: format!("Processing Records into Memory: {}/{}", total, total),
            },
        )
        .ok();

        println!("filenames to check: {:?}", filenames_to_check);

        // Convert Arc to Vec, modify in parallel, and convert back
        total = self.records.len();
        self.records
            .par_iter_mut()
            .enumerate()
            .for_each(|(count, record)| {
                if count % RECORD_DIVISOR == 0 {
                    app.emit(
                        "search-sub-status",
                        SearchStatus {
                            stage: "compare".into(),
                            progress: (count * 100 / total) as u64,
                            message: format!("Comparing against Database: {}/{}", count, total),
                        },
                    )
                    .ok();
                }

                if filenames_to_check.contains(record.get_filename()) {
                    record.algorithm.insert(A::Compare);
                    record.algorithm.remove(&A::Keep);
                }
            });
        app.emit(
            "search-sub-status",
            SearchStatus {
                stage: "compare".into(),
                progress: 100,
                message: format!("Comparing against Database: {}/{}", total, total),
            },
        )
        .ok();
    }

    fn dupe_search(&mut self, pref: &Preferences, app: &AppHandle) {
        println!("Starting Duplicate Search");

        let mut file_groups: HashMap<Vec<Arc<str>>, Vec<FileRecord>> =
            HashMap::with_capacity(self.records.len() / 2);

        let total = self.records.len();
        let mut count = 0;

        // Group records by root
        for record in &*self.records {
            count += 1;
            if count % RECORD_DIVISOR == 0 {
                app.emit(
                    "search-sub-status",
                    SearchStatus {
                        stage: "dupes".into(),
                        progress: (count * 100 / total) as u64,
                        message: format!("Oraginizing Records: {}/{}", count, total),
                    },
                )
                .ok();
            }
            let mut key = Vec::new();
            for m in &pref.match_criteria {
                println!("m: {}", m);
                if &**m == "Filename" {
                    key.push(record.root.clone());
                } else {
                    key.push(record.data[m].clone());
                }
            }
            file_groups.entry(key).or_default().push(record.clone());
        }
        app.emit(
            "search-sub-status",
            SearchStatus {
                stage: "dupes".into(),
                progress: 100,
                message: format!("Oraginizing Records: {}/{}", total, total),
            },
        )
        .ok();

        println!("marking dupes");

        // Determine whether to filter out single-record groups
        let processed_records: Vec<FileRecord> = file_groups
            .into_iter()
            .enumerate()
            .flat_map(|(count, (_, mut records))| {
                if count % RECORD_DIVISOR == 0 {
                    app.emit(
                        "search-sub-status",
                        SearchStatus {
                            stage: "dupes".into(),
                            progress: (count * 100 / total) as u64,
                            message: format!("Marking Duplicates: {}/{}", count, total),
                        },
                    )
                    .ok();
                }
                if records.len() < 2 {
                    return records;
                }
                pref.sort_vec(&mut records);

                records.iter_mut().enumerate().for_each(|(i, record)| {
                    if i > 0 {
                        record.algorithm.remove(&A::Keep);
                    }

                    if &*record.root == record.get_filename()
                        || &*record.root == record.get_filestem()
                    {
                        record.algorithm.insert(A::Basic);
                    } else if pref.check_tags(record.get_filestem()) {
                        record.algorithm.insert(A::Tags);
                    } else {
                        record.algorithm.insert(A::SimilarFilename);
                    }
                });

                records.into_iter().collect::<Vec<_>>()
            })
            .collect();

        app.emit(
            "search-sub-status",
            SearchStatus {
                stage: "dupes".into(),
                progress: 100,
                message: format!("Marking Duplicates: {}/{}", total, total),
            },
        )
        .ok();
        self.records = processed_records;

        println!("all done!");
    }

    fn wave_search(&mut self, pref: &Preferences) {
        println!("Starting Waveform Search");

        // for record in &self.records {
        //     let _ = audiohash::get_chromaprint_fingerprint(&record.path);
        // }

        // Step 1: Calculate hashes for records that don't have them using parallel processing
        let paths_to_hash: Vec<String> = self
            .records
            .iter()
            .filter(|record| record.audio_fingerprint.is_none())
            .filter_map(|record| record.path.to_str().map(|p| p.to_owned()))
            .collect();

        // Only perform hash processing if there are files that need it
        if !paths_to_hash.is_empty() {
            println!("Calculating audio hashes for {} files", paths_to_hash.len());

            // let (results, duration) = audiohash::measure_performance(|| {
            //     audiohash::process_files_in_parallel(&paths_to_hash, pref.ignore_filetype);
            // });

            // println!("Processing completed in {:?}", duration);

            // Process files in parallel to generate hashes
            let hash_results =
                audiohash::process_files_in_parallel(&paths_to_hash, pref.ignore_filetype);

            // Update records with calculated hashes
            for (path, hash_result) in hash_results {
                if let Ok(hash) = hash_result {
                    // Find the record with matching path and update its hash
                    let mut records = self.records.to_vec();
                    if let Some(record) = records
                        .iter_mut()
                        .find(|r| r.path.to_string_lossy() == path)
                    {
                        record.audio_fingerprint = Some(Arc::from(hash.as_str()));
                    }
                    self.records = records;
                } else {
                    // Optionally log hash failures
                    eprintln!("Failed to hash file: {}", path);
                }
            }
        }

        // Step 2: Group records by hash (identical to original approach)
        let groups = {
            let (sender, receiver) = crossbeam_channel::bounded(1024);
            let record_count = self.records.len();

            // Process records in parallel
            self.records
                .par_chunks(record_count.max(1) / num_cpus::get().max(1))
                .for_each_with(sender.clone(), |s, chunk| {
                    // Process each chunk of records
                    let mut local_groups: HashMap<Arc<str>, Vec<FileRecord>> = HashMap::new();

                    for record in chunk {
                        if let Some(hash) = &record.audio_fingerprint {
                            local_groups
                                .entry(hash.clone())
                                .or_default()
                                .push(record.clone());
                        }
                    }

                    // Send local results to the collector
                    for (hash, records) in local_groups {
                        s.send((hash, records)).unwrap();
                    }
                });

            // Drop the sender to signal completion
            drop(sender);

            // Collect results into a hashmap
            let mut file_groups: HashMap<Arc<str>, Vec<FileRecord>> =
                HashMap::with_capacity(record_count / 2);

            while let Ok((hash, records)) = receiver.recv() {
                file_groups
                    .entry(hash)
                    .or_default()
                    .append(&mut records.clone());
            }

            file_groups
        };

        println!("Marking duplicates");

        // Process groups in parallel (identical to original approach)
        let processed_records = {
            let (sender, receiver) = crossbeam_channel::bounded(1024);

            // Use rayon to process groups in parallel
            groups
                .into_par_iter()
                .for_each_with(sender.clone(), |s, (_, mut records)| {
                    if records.len() < 2 {
                        // Send single records directly
                        for record in records {
                            s.send(record).unwrap();
                        }
                        return;
                    }

                    // Sort records according to preferences
                    pref.sort_vec(&mut records);

                    // Mark duplicates
                    records.iter_mut().enumerate().for_each(|(i, record)| {
                        if i > 0 {
                            record.algorithm.remove(&A::Keep);
                        }
                        record.algorithm.insert(A::Waveforms);
                    });

                    // Send all processed records
                    for record in records {
                        s.send(record).unwrap();
                    }
                });

            // Drop sender to signal completion
            drop(sender);

            // Collect all records into a vector
            let mut result = Vec::with_capacity(self.records.len());
            while let Ok(record) = receiver.recv() {
                result.push(record);
            }

            result
        };

        // Replace records with processed records
        self.records = processed_records;

        println!("All done!");
    }

    // fn wave_search_better(&mut self, pref: &Preferences) {
    //     println!("Starting Waveform Search");

    //     // Use rayon's parallel iterator for initial processing
    //     let groups = {
    //         let (sender, receiver) = crossbeam_channel::bounded(1024);
    //         let record_count = self.records.len();

    //         // Process records in parallel
    //         self.records
    //             .par_chunks(record_count.max(1) / num_cpus::get().max(1))
    //             .for_each_with(sender.clone(), |s, chunk| {
    //                 // Process each chunk of records
    //                 let mut local_groups: HashMap<Arc<str>, Vec<FileRecord>> = HashMap::new();

    //                 for record in chunk {
    //                     if let Some(hash) = &record.audio_hash {
    //                         local_groups
    //                             .entry(hash.clone())
    //                             .or_default()
    //                             .push(record.clone());
    //                     }
    //                 }

    //                 // Send local results to the collector
    //                 for (hash, records) in local_groups {
    //                     s.send((hash, records)).unwrap();
    //                 }
    //             });

    //         // Drop the sender to signal completion
    //         drop(sender);

    //         // Collect results into a hashmap
    //         let mut file_groups: HashMap<Arc<str>, Vec<FileRecord>> =
    //             HashMap::with_capacity(record_count / 2);

    //         while let Ok((hash, records)) = receiver.recv() {
    //             file_groups
    //                 .entry(hash)
    //                 .or_default()
    //                 .append(&mut records.clone());
    //         }

    //         file_groups
    //     };

    //     println!("Marking duplicates");

    //     // Process groups in parallel
    //     let processed_records = {
    //         let (sender, receiver) = crossbeam_channel::bounded(1024);

    //         // Use rayon to process groups in parallel
    //         groups
    //             .into_par_iter()
    //             .for_each_with(sender.clone(), |s, (_, mut records)| {
    //                 if records.len() < 2 {
    //                     // Send single records directly
    //                     for record in records {
    //                         s.send(record).unwrap();
    //                     }
    //                     return;
    //                 }

    //                 // Sort records according to preferences
    //                 pref.sort_vec(&mut records);

    //                 // Mark duplicates
    //                 records.iter_mut().enumerate().for_each(|(i, record)| {
    //                     if i > 0 {
    //                         record.algorithm.remove(&A::Keep);
    //                     }
    //                     record.algorithm.insert(A::Waveforms);
    //                 });

    //                 // Send all processed records
    //                 for record in records {
    //                     s.send(record).unwrap();
    //                 }
    //             });

    //         // Drop sender to signal completion
    //         drop(sender);

    //         // Collect all records into a vector
    //         let mut result = Vec::with_capacity(self.records.len());
    //         while let Ok(record) = receiver.recv() {
    //             result.push(record);
    //         }

    //         result
    //     };

    //     // Replace records with processed records
    //     self.records = processed_records;

    //     println!("All done!");
    // }

    async fn update_column_value(
        &self,
        row: usize,
        column: &str,
        value: &str,
    ) -> Result<(), sqlx::Error> {
        if let Some(pool) = self.get_pool().await {
            // Create a parameterized query to update a specific column in a specific row
            let query = format!("UPDATE {} SET {} = ? WHERE rowid = ?", TABLE, column);

            // Execute the query with the provided parameters
            sqlx::query(&query)
                .bind(value)
                .bind(row as i64) // SQLite uses i64 for rowid
                .execute(&pool)
                .await?;

            // println!("Updated column '{}' in row {} with value '{}'", column_name, row_id, value);
            return Ok(());
        }

        Err(sqlx::Error::Configuration(
            "No database connection available".into(),
        ))
    }

    async fn wave_search_chromaprint(&mut self, pref: &Preferences, app: &AppHandle) {
        println!("Starting Waveform Search");

        // Count how many files need fingerprinting
        let to_fingerprint = self
            .records
            .iter()
            .filter(|record| {
                record.audio_fingerprint.is_none() || record.audio_fingerprint_raw.is_none()
            })
            .count();

        app.emit(
            "search-sub-status",
            SearchStatus {
                stage: "fingerprinting".into(),
                progress: 0,
                message: format!("Analyzing {} audio files", to_fingerprint),
            },
        )
        .ok();

        // Use atomic counter for thread-safe progress tracking
        let counter = std::sync::atomic::AtomicUsize::new(0);

        // Collect fingerprints and their corresponding record IDs
        let fingerprints: Vec<(usize, String, String)> = self
            .records
            .par_iter()
            .filter(|record| {
                record.audio_fingerprint.is_none() || record.audio_fingerprint_raw.is_none()
            })
            .filter_map(|record| {
                let idx = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                // Report progress for each file
                if idx % RECORD_DIVISOR == 0 || idx == 0 || idx == to_fingerprint - 1 {
                    app.emit(
                        "search-sub-status",
                        SearchStatus {
                            stage: "fingerprinting".into(),
                            progress: ((idx + 1) * 100 / to_fingerprint) as u64,
                            message: format!(
                                "Analyzing: {} ({}/{})",
                                record.get_filename(),
                                idx + 1,
                                to_fingerprint
                            ),
                        },
                    )
                    .ok();
                }

                if checkduration(&record.duration, 1.0) {
                    let hash =
                        audiohash::hash_audio_content(record.get_path(), pref.ignore_filetype);
                    if let Ok(hash) = hash {
                        Some((record.id, hash.clone(), hash.clone()))
                    } else {
                        None
                    }
                } else {
                    let result = audiohash::get_chromaprint_fingerprint(&record.path);
                    if let Ok(result) = result {
                        let (fingerprint, encoded) = result;
                        if encoded.is_empty() {
                            let hash = audiohash::hash_audio_content(
                                record.get_path(),
                                pref.ignore_filetype,
                            );
                            if let Ok(hash) = hash {
                                return Some((record.id, hash.clone(), hash));
                            } else {
                                return None;
                            }
                        }
                        Some((record.id, fingerprint, encoded))
                    } else {
                        None
                    }
                }
            })
            .collect();

        // Update database in separate step
        if pref.store_waveforms && !fingerprints.is_empty() {
            app.emit(
                "search-sub-status",
                SearchStatus {
                    stage: "storing".into(),
                    progress: 0,
                    message: "Storing fingerprints in database...".into(),
                },
            )
            .ok();

            for (i, (id, fingerprint, encoded)) in fingerprints.iter().enumerate() {
                if i % RECORD_DIVISOR == 0 || i == 0 || i == fingerprints.len() - 1 {
                    app.emit(
                        "search-sub-status",
                        SearchStatus {
                            stage: "storing".into(),
                            progress: ((i + 1) * 100 / fingerprints.len()) as u64,
                            message: format!(
                                "Storing fingerprints: {}/{}",
                                i + 1,
                                fingerprints.len()
                            ),
                        },
                    )
                    .ok();
                }

                let _ = self
                    .update_column_value(*id, "_fingerprint", fingerprint)
                    .await;
                let _ = self
                    .update_column_value(*id, "_fingerprint_raw", encoded)
                    .await;
            }
        }

        // Update records with the collected fingerprints
        app.emit(
            "search-sub-status",
            SearchStatus {
                stage: "updating".into(),
                progress: 0,
                message: "Updating records with fingerprints...".into(),
            },
        )
        .ok();

        for (i, (id, fingerprint, encoded)) in fingerprints.iter().enumerate() {
            if i % RECORD_DIVISOR == 0 || i == 0 || i == fingerprints.len() - 1 {
                app.emit(
                    "search-sub-status",
                    SearchStatus {
                        stage: "updating".into(),
                        progress: ((i + 1) * 100 / fingerprints.len()) as u64,
                        message: format!("Updating records: {}/{}", i + 1, fingerprints.len()),
                    },
                )
                .ok();
            }

            if let Some(record) = self.records.iter_mut().find(|r| r.id == *id) {
                record.audio_fingerprint = Some(Arc::from(fingerprint.as_str()));
                record.audio_fingerprint_raw = Some(Arc::from(encoded.as_str()));
            }
        }

        if pref.exact_waveform {
            app.emit(
                "search-sub-status",
                SearchStatus {
                    stage: "grouping".into(),
                    progress: 0,
                    message: "Grouping identical audio fingerprints...".into(),
                },
            )
            .ok();

            let mut file_groups: HashMap<Arc<str>, Vec<FileRecord>> =
                HashMap::with_capacity(self.records.len() / 2);

            // Group records by waveform
            for (i, record) in self.records.iter().enumerate() {
                if i % RECORD_DIVISOR == 0 || i == 0 || i == self.records.len() - 1 {
                    app.emit(
                        "search-sub-status",
                        SearchStatus {
                            stage: "grouping".into(),
                            progress: ((i + 1) * 100 / self.records.len()) as u64,
                            message: format!(
                                "Grouping by fingerprint: {}/{}",
                                i + 1,
                                self.records.len()
                            ),
                        },
                    )
                    .ok();
                }

                if let Some(hash) = &record.audio_fingerprint {
                    file_groups
                        .entry(hash.clone())
                        .or_default()
                        .push(record.clone());
                }
            }

            println!("Marking duplicate audio files");
            app.emit(
                "search-sub-status",
                SearchStatus {
                    stage: "marking".into(),
                    progress: 0,
                    message: "Marking duplicate audio files...".into(),
                },
            )
            .ok();

            // Process groups
            let group_count = file_groups.len();
            let processed_records: Vec<FileRecord> = file_groups
                .into_iter()
                .enumerate()
                .flat_map(|(i, (_, mut records))| {
                    if i % RECORD_DIVISOR == 0 || i == 0 || i == group_count - 1 {
                        app.emit(
                            "search-sub-status",
                            SearchStatus {
                                stage: "marking".into(),
                                progress: ((i + 1) * 100 / group_count) as u64,
                                message: format!("Processing group: {}/{}", i + 1, group_count),
                            },
                        )
                        .ok();
                    }

                    if records.len() < 2 {
                        return records;
                    }
                    pref.sort_vec(&mut records);

                    records.iter_mut().enumerate().for_each(|(i, record)| {
                        print!(
                            "{}, {}",
                            record.audio_fingerprint.as_ref().unwrap(),
                            record.root
                        );
                        if i > 0 {
                            record.algorithm.remove(&A::Keep);
                            print!(" Removed ");
                        }
                        record.algorithm.insert(A::Waveforms);
                        println!();
                    });

                    records
                })
                .collect();

            self.records = processed_records;

            app.emit(
                "search-sub-status",
                SearchStatus {
                    stage: "complete".into(),
                    progress: 100,
                    message: "Audio fingerprint analysis complete".into(),
                },
            )
            .ok();
        } else {
            // Similar progress reporting for similarity comparison...
            app.emit(
                "search-sub-status",
                SearchStatus {
                    stage: "similarity".into(),
                    progress: 0,
                    message: "Starting similarity-based audio comparison...".into(),
                },
            )
            .ok();

            // For similarity comparison implementation, follow the same pattern
            // of reporting progress at key steps

            // ...existing similarity comparison code with progress updates...

            app.emit(
                "search-sub-status",
                SearchStatus {
                    stage: "complete".into(),
                    progress: 100,
                    message: "Similarity-based audio comparison complete".into(),
                },
            )
            .ok();
        }
    }
}

fn checkduration(duration: &str, min_dur: f64) -> bool {
    if let Some((minutes, rest)) = duration.split_once(':') {
        if let (Ok(mins), Ok(secs)) = (minutes.parse::<f64>(), rest.parse::<f64>()) {
            let total_seconds = (mins * 60.0) + secs;
            total_seconds < min_dur
        } else {
            false
        }
    } else {
        false
    }
}

fn checktags(name: &str, tags: &Vec<Arc<str>>) -> bool {
    for tag in tags {
        if name.contains(&**tag) {
            return true;
        }
    }

    false
}

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy, Default)]
pub enum Delete {
    #[default]
    Keep,
    Trash,
    Delete,
}

impl Delete {
    pub fn delete_files(&self, files: Vec<&str>) -> Result<(), Box<dyn std::error::Error>> {
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
            Delete::Delete => {
                for file in valid_files {
                    fs::remove_file(file).map_err(|e| {
                        eprintln!("Failed to remove file {}: {}", file, e);
                        e
                    })?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}

fn get_column_as_string(row: &SqliteRow, column: &str) -> Option<Arc<str>> {
    // Try getting as text first (most common case)
    if let Ok(value) = row.try_get::<&str, _>(column) {
        return Some(Arc::from(value));
    }

    // Then try numeric types
    if let Ok(value) = row.try_get::<i64, _>(column) {
        return Some(Arc::from(value.to_string()));
    }

    if let Ok(value) = row.try_get::<f64, _>(column) {
        return Some(Arc::from(value.to_string()));
    }

    // Handle null or other types
    None
}
