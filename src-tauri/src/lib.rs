pub mod commands;
pub mod preferences;
pub mod search;
pub mod windows;
pub use crate::audio::*;
pub mod audio;
pub use dirs::home_dir;
pub use preferences::*;
pub mod prelude;
// pub use FFcodex::*;
pub use commands::*;
pub use regex::Regex;
pub use sqlx::Row;
pub use sqlx::sqlite::{SqlitePool, SqliteRow};
use std::hash::Hash;
// use tauri::App;
// use tauri::menu::{Menu, MenuBuilder, MenuItem, Submenu};

pub const TABLE: &str = "justinmetadata";
pub const RECORD_DIVISOR: usize = 1231;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    set_library_path();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let version = app.package_info().version.to_string();
            let _ = app
                .webview_windows()
                .get("main")
                .unwrap()
                .set_title(&format!("SMDB Companion :: v{}", version));
            audio::playback::init_audio_system();
            app.manage(Mutex::new(AppState::default()));

            // menu(app)?;
            Ok(())
        })
        // .plugin(tauri_plugin_window::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_current_version,
            open_db,
            close_db,
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
            cancel_search,
            play_audio,
            stop_audio,
            pause_audio,
            resume_audio,
            clear_fingerprints,
            refresh_all_windows,
            open_database_folder,
            reveal_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// In your main.rs or lib.rs
fn set_library_path() {
    use std::env;
    use std::path::Path;

    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            #[cfg(debug_assertions)]
            let resources_path = Path::new(&exe_dir).join("../../../resources");

            #[cfg(not(debug_assertions))]
            let resources_path = if exe_dir.to_string_lossy().contains("MacOS") {
                // We're inside a macOS bundle
                Path::new(&exe_dir)
                    .join("../Resources/resources")
                    .to_path_buf()
            } else {
                Path::new(&exe_dir).join("resources").to_path_buf()
            };

            if resources_path.exists() {
                println!("Found resources at: {}", resources_path.display());
                let path_string = resources_path.to_string_lossy().to_string();
                // env::set_var is unsafe because it modifies process-wide state
                unsafe {
                    env::set_var("DYLD_LIBRARY_PATH", &path_string);
                }
                println!("Set DYLD_LIBRARY_PATH to {}", path_string);
            } else {
                println!("Resources path not found at: {}", resources_path.display());
            }
        }
    }
}

#[derive(Default)]
pub struct AppState {
    db: Database,
    // search_results: Vec<FileRecordFrontEnd>,
    // handle: JoinHandle<Result<Vec<FileRecordFrontend>>>,
    // abort: Arc<AtomicBool>,
    // enabled: Enabled,
    // pref: Preferences,
    // reg: Registration,
}

#[derive(Clone, serde::Serialize)]
pub struct StatusUpdate {
    stage: String,
    progress: usize,
    message: String,
}

pub trait StatusEmitter {
    fn status(&self, stage: &str, progress: usize, message: &str);
    fn substatus(&self, stage: &str, progress: usize, message: &str);
    fn rstatus(&self, stage: &str, progress: usize, message: &str);
    fn rsubstatus(&self, stage: &str, progress: usize, message: &str);
}

impl StatusEmitter for AppHandle {
    fn status(&self, stage: &str, progress: usize, message: &str) {
        self.emit(
            "search-status",
            StatusUpdate {
                stage: stage.into(),
                progress,
                message: message.into(),
            },
        )
        .ok();
    }
    fn substatus(&self, stage: &str, progress: usize, message: &str) {
        self.emit(
            "search-sub-status",
            StatusUpdate {
                stage: stage.into(),
                progress,
                message: message.into(),
            },
        )
        .ok();
    }
    fn rstatus(&self, stage: &str, progress: usize, message: &str) {
        self.emit(
            "remove-status",
            StatusUpdate {
                stage: stage.into(),
                progress,
                message: message.into(),
            },
        )
        .ok();
    }
    fn rsubstatus(&self, stage: &str, progress: usize, message: &str) {
        self.emit(
            "remove-sub-status",
            StatusUpdate {
                stage: stage.into(),
                progress,
                message: message.into(),
            },
        )
        .ok();
    }
}

#[derive(Serialize, Deserialize)] // Need Deserialize to receive it back
pub struct FileRecordFrontend {
    id: usize,
    path: Arc<str>,
    filename: Arc<str>,
    algorithm: Vec<Algorithm>,
    channels: u32,
    bitdepth: u32,
    samplerate: u32,
    duration: Arc<str>,
    description: Arc<str>,
    // data: HashMap<Arc<str>, Arc<str>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DualMono {
    pub id: usize,
    pub path: String,
}

// Change visibility of `FileRecord` fields to private where possible
#[derive(Default, Debug, Serialize, Clone)]
pub struct FileRecord {
    pub id: usize,
    path: std::path::PathBuf,          // Made private
    root: Arc<str>,                    // Made private
    duration: Arc<str>,                // Made private
    samplerate: u32,                   // Made private
    bitdepth: u32,                     // Made private
    channels: u32,                     // Made private
    description: Arc<str>,             // Made private
    data: HashMap<Arc<str>, Arc<str>>, // Made private
    fingerprint: Option<Arc<str>>,     // Made private
    dual_mono: Option<bool>,           // Made private
    algorithm: HashSet<Algorithm>,     // Made private
}
impl Hash for FileRecord {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Only hash the fields you want to consider for identity
        self.id.hash(state);
        // Add any other fields you want to include in the hash calculation
        // For example: self.path.hash(state);
    }
}

impl PartialEq for FileRecord {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for FileRecord {}

impl FileRecord {
    pub fn new(row: &SqliteRow, enabled: &Enabled, pref: &Preferences, is_compare: bool) -> Self {
        let _ = is_compare;
        let id = row.get::<u32, _>(0) as usize;
        let path_str: &str = row.get(1);

        #[cfg(not(target_os = "windows"))]
        let path = PathBuf::from(path_str);

        #[cfg(target_os = "windows")]
        let mut path = PathBuf::from(path_str);
        #[cfg(target_os = "windows")]
        if let Some(p) = windows::auto_convert_macos_path_to_windows(path_str) {
            path = p;
        }

        let duration_str: &str = row.get(2);
        let description: &str = row.get(4);
        let channels = row.get(5);
        let bitdepth = row.get(6);
        let samplerate = row.get(7);

        let mut algorithm = HashSet::new();
        let mut keep = true;
        if enabled.invalidpath || enabled.dual_mono {
            // Use the path existence check from the PathBuf directly
            // This handles cross-platform path separators correctly
            let path_exists = path.exists();

            if !path_exists {
                algorithm.insert(Algorithm::InvalidPath);
                if enabled.invalidpath {
                    keep = false;
                }
            }
        }
        if enabled.duration && checkduration(duration_str, enabled.min_dur) {
            algorithm.insert(Algorithm::Duration);
            keep = false;
        }
        if enabled.filetags && checktags(path_str, &pref.autoselects) {
            algorithm.insert(Algorithm::FileTags);
            keep = false;
        }
        if keep {
            algorithm.insert(Algorithm::Keep);
        }

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

        let f = row.try_get::<&str, _>("_fingerprint").unwrap_or_default();

        let fingerprint = if f.is_empty() || !pref.fetch_waveforms {
            None
        } else {
            Some(Arc::from(f))
        };

        let mut dual_mono = None;

        if pref.fetch_waveforms {
            let dm = row.try_get::<&str, _>("_DualMono").ok();
            dual_mono = match dm {
                Some("1") => Some(true),
                Some("0") => Some(false),
                _ => None,
            };
        }

        let mut record = Self {
            id,
            path,
            root: Arc::default(),
            duration: Arc::from(duration_str),
            data,
            fingerprint,
            algorithm,
            channels,
            bitdepth,
            samplerate,
            description: Arc::from(description),
            dual_mono,
        };

        record.set_root(enabled, pref);
        record
    }

    pub fn set_root(&mut self, _enabled: &Enabled, pref: &Preferences) {
        // Use Cow to avoid unnecessary string allocations
        let mut name = Cow::Borrowed(self.get_filestem().trim());

        // Optimize tag processing with minimal allocations
        if let Some((base, _)) = pref.tags.iter().find_map(|tag| name.split_once(&**tag)) {
            name = Cow::Owned(base.to_string());
        }

        // Use const array to avoid repeated allocations
        const COPY_TAGS: [&str; 6] = [" copy.", " Copy.", " COPY.", ".copy.", ".Copy.", ".COPY."];

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

    pub fn check_path(&self) -> bool {
        self.path.exists()
    }

    pub fn get_filepath(&self) -> String {
        // Use display() which handles both Windows and macOS paths correctly
        // and converts to lossy UTF-8 representation when needed
        self.path.display().to_string()
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

    pub fn get_duration(&self) -> Result<f64, String> {
        if let Some((minutes, rest)) = self.duration.split_once(':') {
            if let (Ok(mins), Ok(secs)) = (minutes.parse::<f64>(), rest.parse::<f64>()) {
                return Ok((mins * 60.0) + secs);
            }
        }
        Err("Unable to parse duration".to_string())
    }
}

static FILENAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?P<base>.+?)(?:\.(?:\d+|M))*$").unwrap());

#[derive(Default, Clone)]
pub struct Database {
    path: Option<PathBuf>,
    size: usize,
    records: Vec<FileRecord>, // Changed from Arc<[FileRecord]> to Vec<FileRecord>
    is_compare: bool,
    // abort: Arc<RwLock<bool>>,
    abort: Arc<AtomicBool>,
}

// Change visibility of `Database` methods to private where possible
impl Database {
    pub async fn new(path: &str, is_compare: bool) -> Self {
        println!("🆕 Creating new Database instance");
        println!("📁 Path: {}", path);
        println!("🔄 Is compare: {}", is_compare);

        let mut d = Database {
            path: Some(PathBuf::from(path)),
            size: 0,
            records: Vec::new(),
            is_compare,
            abort: Arc::new(AtomicBool::new(false)),
        };

        println!("📏 Fetching initial database size...");
        d.size = d.fetch_size().await.unwrap_or_else(|e| {
            println!("⚠️  Failed to fetch size, using 0: {}", e);
            0
        });

        println!("💾 Initializing records vector with capacity: {}", d.size);
        d.records = Vec::with_capacity(d.size);

        println!("✅ Database instance created successfully");
        d
    }

    pub async fn open(&mut self, is_compare: bool) -> Option<Self> {
        let home_dir = home_dir();
        match home_dir {
            Some(home_dir) => {
                println!("Found SMDB dir");
                let db_dir = home_dir.join("Library/Application Support/SoundminerV6/Databases");
                let path = FileDialog::new()
                    .add_filter("SQLite Database", &["sqlite"])
                    .set_directory(db_dir)
                    .pick_file();
                self.init(path, is_compare).await;
            }
            None => {
                let path = FileDialog::new()
                    .add_filter("SQLite Database", &["sqlite"])
                    .pick_file();
                self.init(path, is_compare).await;
            }
        }
        None
    }

    async fn init(&mut self, path: Option<PathBuf>, is_compare: bool) {
        if let Some(path) = path {
            self.path = Some(path);
            self.size = self.fetch_size().await.unwrap_or(0);
            self.records = Vec::with_capacity(self.size); // No need for .into()
            self.is_compare = is_compare;
        }
    }

    async fn create_clone(&self, tag: &str) -> Result<Database, std::io::Error> {
        let source_path = self.path.as_ref().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "No source database path")
        })?;

        // Create the new path with proper cross-platform handling
        let mut path_string = source_path.display().to_string();
        path_string = path_string.replace(".sqlite", &format!("_{}.sqlite", tag));
        let target_path = PathBuf::from(path_string);

        // Check if source file exists and is readable
        if !source_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Source database not found: {}", source_path.display()),
            ));
        }

        // Check if target directory exists and is writable
        if let Some(parent) = target_path.parent() {
            if !parent.exists() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Target directory not found: {}", parent.display()),
                ));
            }

            // Test write permissions by attempting to create a temporary file
            let test_file = parent.join(".write_test_tmp");
            match std::fs::File::create(&test_file) {
                Ok(_) => {
                    let _ = std::fs::remove_file(&test_file); // Clean up
                }
                Err(e) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!(
                            "Cannot write to target directory {}: {}",
                            parent.display(),
                            e
                        ),
                    ));
                }
            }
        }

        // Perform the copy operation with proper error handling
        fs::copy(source_path, &target_path).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to copy database from {} to {}: {}",
                    source_path.display(),
                    target_path.display(),
                    e
                ),
            )
        })?;

        // Initialize the new database
        let mut db = Database::default();
        db.init(Some(target_path), false).await;
        Ok(db)
    }

    fn get_path(&self) -> Option<Arc<str>> {
        if let Some(path) = &self.path {
            if let Some(path_str) = path.to_str() {
                println!("🛤️  Database path: {}", path_str);
                return Some(Arc::from(path_str));
            } else {
                println!("❌ Failed to convert path to string");
            }
        } else {
            println!("❌ No database path set");
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
            println!("🔌 Attempting to connect to database: {}", path);

            // Check if file exists first
            let path_buf = std::path::PathBuf::from(path.as_ref());
            if !path_buf.exists() {
                println!("❌ Database file does not exist: {}", path);
                return None;
            }

            // Check file size
            match std::fs::metadata(&path_buf) {
                Ok(metadata) => {
                    let file_size = metadata.len();
                    println!("📊 Database file size: {} bytes", file_size);
                    if file_size == 0 {
                        println!("⚠️  Database file is empty (0 bytes)");
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get database metadata: {}", e);
                    return None;
                }
            }

            // Attempt connection
            match SqlitePool::connect(&path).await {
                Ok(pool) => {
                    println!("✅ Successfully connected to database");
                    Some(pool)
                }
                Err(e) => {
                    println!("❌ Failed to connect to database: {}", e);
                    None
                }
            }
        } else {
            println!("❌ No database path available for connection");
            None
        }
    }

    async fn remove_column(&self, remove: &str) -> Result<(), sqlx::Error> {
        if let Some(pool) = self.get_pool().await {
            // First check if the column already exists
            let columns = sqlx::query(&format!("PRAGMA table_info({});", TABLE))
                .fetch_all(&pool)
                .await?;

            // Check if our column exists
            let column_exists = columns.iter().any(|row| {
                let column_name: &str = row.try_get("name").unwrap_or_default();
                column_name == remove
            });

            // Only remove the column if it exists
            if column_exists {
                // Remove the column
                let query = format!("ALTER TABLE {} DROP COLUMN {};", TABLE, remove);
                sqlx::query(&query).execute(&pool).await?;
                println!("Removed column: {}", remove);
            } else {
                println!("Column '{}' does not exist", remove);
            }

            return Ok(());
        }

        Err(sqlx::Error::Configuration(
            "No database connection available".into(),
        ))
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
        println!("📏 Attempting to fetch database size...");

        if let Some(pool) = self.get_pool().await {
            println!("🔍 Executing count query on table: {}", TABLE);

            match sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {}", TABLE))
                .fetch_one(&pool)
                .await
            {
                Ok(count) => {
                    let size = count.0 as usize;
                    println!("✅ Database size fetched successfully: {} records", size);
                    Ok(size)
                }
                Err(e) => {
                    println!("❌ Failed to fetch database size: {}", e);
                    Err(e)
                }
            }
        } else {
            println!("❌ No database pool available for size fetch");
            Ok(0)
        }
    }

    pub async fn remove(&self, ids: &[usize], app: &AppHandle) -> Result<(), sqlx::Error> {
        const BATCH_SIZE: usize = 12321; // Define the batch size
        let _ = app;
        let mut counter = 0;
        if let Some(pool) = self.get_pool().await {
            // Iterate over chunks of IDs
            for chunk in ids.chunks(BATCH_SIZE) {
                app.status(
                    "Record Removal",
                    counter * 100 / ids.len(),
                    &format!("Removing Records... {}/{}", counter, ids.len()),
                );

                counter += BATCH_SIZE;
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
            app.status("Final Checks", 100, "Records successfully removed");
        }
        Ok(())
    }

    pub async fn clean_multi_mono(
        &self,
        app: &AppHandle,
        records: &Vec<DualMono>,
    ) -> Result<(), sqlx::Error> {
        use std::sync::Mutex;

        println!("Cleaning up multi-mono files");
        println!("{} Records Found", records.len());
        let completed = AtomicUsize::new(0);
        let failures = AtomicUsize::new(0);

        // Create a synchronized collection for successful record IDs only
        let successful_ids = Arc::new(Mutex::new(Vec::with_capacity(records.len())));

        records.par_iter().for_each(|record| {
            let new_completed = completed.fetch_add(1, Ordering::SeqCst) + 1;
            let path = Path::new(&record.path);
            let filename = path.file_name().unwrap_or_default().to_str().unwrap_or("");

            app.substatus(
                "Stripping Multi-Mono",
                new_completed * 100 / records.len(),
                filename,
            );

            // Debug log initial state
            println!("Processing file: {}", record.path);
            println!("  ID: {}", record.id);

            // First check if file exists
            if !path.exists() {
                failures.fetch_add(1, Ordering::SeqCst);
                println!("ERROR: File not found: {}", record.path);
                return;
            }

            // Check file extension
            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown");
            println!("  Format: {}", extension);

            // Process the file
            match ffcodex_lib::clean_multi_mono(path.to_str().unwrap_or_default()) {
                Ok(_) => {
                    println!("  Strip multi-mono successful");
                    match successful_ids.lock() {
                        Ok(mut ids) => {
                            ids.push(record.id);
                        }
                        Err(_) => {
                            println!("ERROR: Failed to acquire lock on successful_ids");
                        }
                    }
                }
                Err(strip_err) => {
                    failures.fetch_add(1, Ordering::SeqCst);
                    println!(
                        "ERROR: Strip multi-mono failed for {}: {}",
                        record.path, strip_err
                    );
                }
            }
        });

        // Safely get the successful IDs
        let successful_ids = match Arc::try_unwrap(successful_ids) {
            Ok(mutex) => match mutex.into_inner() {
                Ok(ids) => ids,
                Err(_) => {
                    println!("ERROR: Failed to unlock successful_ids mutex");
                    Vec::new()
                }
            },
            Err(_) => {
                println!("ERROR: Failed to unwrap Arc for successful_ids");
                Vec::new()
            }
        };

        // Add summary logging
        let total = records.len();
        let failed = failures.load(Ordering::SeqCst);
        let success = successful_ids.len();
        println!(
            "SUMMARY: Total: {}, Successful: {}, Failed: {}",
            total, success, failed
        );

        // Only update database if we have SUCCESSFUL records to update
        if !successful_ids.is_empty() {
            match self
                .update_channel_count_to_mono(app, &successful_ids)
                .await
            {
                Ok(_) => {
                    app.substatus(
                        "Stripping Multi-Mono",
                        100,
                        &format!("Updated {} files to mono, {} failures", success, failed),
                    );
                    Ok(())
                }
                Err(e) => {
                    app.substatus(
                        "Stripping Multi-Mono",
                        100,
                        &format!("Error updating database: {}", e),
                    );
                    Err(e)
                }
            }
        } else {
            app.substatus(
                "Stripping Multi-Mono",
                100,
                "No files were successfully processed",
            );
            Ok(())
        }
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
        let completed = AtomicUsize::new(0);
        let rows = self.fetch(query).await;
        let mut records = Vec::with_capacity(rows.len());
        println!("{} Rows Found", rows.len());
        let new_records: Vec<FileRecord> = rows
            .par_iter()
            .enumerate()
            .map(|(count, row)| {
                let new_completed = completed.fetch_add(1, Ordering::SeqCst) + 1;
                if new_completed % RECORD_DIVISOR == 0 {
                    app.substatus(
                        "Gathering File Records",
                        new_completed * 100 / rows.len(),
                        format!("Processing Records into Memory: {}/{}", count, rows.len())
                            .as_str(),
                    );
                }
                FileRecord::new(row, enabled, pref, self.is_compare)
            })
            .collect();
        app.substatus("Gathering File Records", 100, "Complete");

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
                "SELECT rowid, filepath, duration, _fingerprint, description, channels, bitdepth, samplerate, _DualMono, {} FROM {}",
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
    pub async fn update_channel_count_to_mono(
        &self,
        app: &AppHandle,
        record_ids: &[usize],
    ) -> Result<(), sqlx::Error> {
        const BATCH_SIZE: usize = 1000; // Smaller batch size for updates

        if let Some(pool) = self.get_pool().await {
            let mut counter = 0;

            // Begin a transaction for better performance
            let mut tx = pool.begin().await?;

            // Process in batches
            for chunk in record_ids.chunks(BATCH_SIZE) {
                // Create placeholders for SQL IN clause
                let placeholders = std::iter::repeat("?")
                    .take(chunk.len())
                    .collect::<Vec<_>>()
                    .join(",");

                // Build update query
                let query = format!(
                    "UPDATE {} SET Channels = 1, _Dirty = 1 WHERE rowid IN ({})",
                    TABLE, placeholders
                );

                // Create query builder
                let mut query_builder = sqlx::query(&query);

                // Bind all IDs
                for &id in chunk {
                    query_builder = query_builder.bind(id as i64);
                }

                // Execute the query within transaction
                query_builder.execute(&mut *tx).await?;

                // Update progress
                counter += chunk.len();
                app.status(
                    "Stripping Multi-Mono",
                    counter * 100 / record_ids.len(),
                    format!(
                        "Updating channel metadata: {}/{}",
                        counter,
                        record_ids.len()
                    )
                    .as_str(),
                );
            }

            // Commit the transaction
            tx.commit().await?;

            // Final status update
            app.status(
                "Stripping Multi-Mono",
                100,
                format!("Updated {} records to mono", record_ids.len()).as_str(),
            );
        }

        Ok(())
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
    pub fn delete_files(
        &self,
        files: Vec<&str>,
        app: &AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("Removing Files");
        app.substatus(
            "Removing Files",
            0,
            "Preparing to remove files... Checking Validity",
        );

        // Filter valid files using PathBuf for cross-platform compatibility
        let valid_files: Vec<&str> = files
            .par_iter()
            .filter(|&&file| {
                let path = std::path::Path::new(file);
                let exists = path.exists();
                if !exists {
                    println!("File does not exist: {}", file);
                }
                exists
            })
            .cloned()
            .collect();

        println!(
            "Valid files to process: {}/{}",
            valid_files.len(),
            files.len()
        );
        app.substatus(
            "Removing Files",
            10,
            &format!("Processing {} valid files", valid_files.len()),
        );

        if valid_files.is_empty() {
            app.substatus("Removing Files", 100, "No valid files to process");
            return Ok(());
        }

        match self {
            Delete::Trash => {
                match trash::delete_all(&valid_files) {
                    Ok(_) => {
                        println!("Successfully moved all files to trash");
                    }
                    Err(e) => {
                        eprintln!("Failed to move files to trash: {}", e);
                        app.substatus(
                            "Removing Files",
                            100,
                            &format!("Error moving files to trash: {}", e),
                        );
                        return Err(Box::new(e));
                    }
                }
                app.substatus("Removing Files", 100, "All files moved to trash");
                return Ok(());
            }
            Delete::Delete => {
                let total = valid_files.len();
                for (i, file) in valid_files.iter().enumerate() {
                    app.substatus(
                        "Removing Files",
                        10 + (i * 90 / total),
                        &format!("Permanently deleting: {}/{}", i + 1, total),
                    );

                    // Use PathBuf::display() for consistent cross-platform path handling
                    let path = std::path::PathBuf::from(file);
                    let normalized_path = path.display().to_string();

                    if let Err(e) = fs::remove_file(&normalized_path) {
                        eprintln!("Failed to remove file {}: {}", normalized_path, e);
                        app.substatus(
                            "Removeing Files",
                            10 + (i * 90 / total),
                            &format!("Warning: Failed to delete: {}", normalized_path),
                        );
                    } else {
                        println!("Successfully deleted file: {}", normalized_path);
                    }
                }
            }
            _ => {}
        }

        app.substatus("Removing Files", 100, "File removal complete");
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

async fn batch_store_data_optimized(
    pool: &SqlitePool,
    data: &[(usize, &str)],
    column: &str,
    app: &AppHandle,
) {
    let name: &str = column.strip_prefix('_').unwrap_or(column);

    if data.is_empty() {
        println!("No {} to store", name);
        return;
    }

    println!("Storing {} {} in database", data.len(), name);

    app.substatus(
        "Storing Batch to Database",
        0,
        format!("Storing {} {} in database...", name, data.len()).as_str(),
    );

    match pool.begin().await {
        Ok(mut tx) => {
            let _ = sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&mut *tx)
                .await;
            let _ = sqlx::query("PRAGMA synchronous = NORMAL")
                .execute(&mut *tx)
                .await;

            let total = data.len();
            let mut success_count = 0;
            let mut error_count = 0;

            for (i, (id, d)) in data.iter().enumerate() {
                if i % 25 == 0 || i == total - 1 {
                    app.substatus(
                        "Storing Batch to Database",
                        (i + 1) * 100 / total,
                        format!("Storing {}: {}/{}", name, i + 1, total).as_str(),
                    );
                }

                let result = sqlx::query(&format!(
                    "UPDATE {} SET {} = ? WHERE rowid = ?",
                    TABLE, column
                ))
                .bind(d)
                .bind(*id as i64)
                .execute(&mut *tx)
                .await;

                match result {
                    Ok(result) => {
                        if result.rows_affected() == 0 {
                            println!(
                                "WARNING: No rows affected when updating {} for ID {}",
                                name, id
                            );
                        } else {
                            success_count += 1;
                        }
                    }
                    Err(e) => {
                        println!("ERROR updating {} for ID {}: {}", name, id, e);
                        error_count += 1;
                    }
                }
            }
            app.substatus(
                "Storing Batch to Database",
                99,
                &format!(
                    "Committing all changes to database: {} {}s updated, {} errors",
                    success_count, name, error_count
                ),
            );

            match tx.commit().await {
                Ok(_) => {
                    println!(
                        "Transaction committed successfully: {} {}s updated, {} errors",
                        success_count, name, error_count
                    );

                    let checkpoint_result = sqlx::query("PRAGMA wal_checkpoint(FULL)")
                        .execute(pool)
                        .await;

                    if let Err(e) = checkpoint_result {
                        println!("WARNING: Checkpoint failed: {}", e);
                    } else {
                        println!("Database checkpoint successful");
                    }
                }
                Err(e) => println!("ERROR: Transaction failed to commit: {}", e),
            }
            app.substatus(
                "Storing Batch to Database",
                100,
                format!(
                    "Database update complete: {} {} stored",
                    success_count, name
                )
                .as_str(),
            );
        }
        Err(e) => {
            println!("ERROR: Failed to start transaction: {}", e);
            app.substatus(
                "Storing Batch to Database",
                100,
                &format!("ERROR: Databaes update failed: {}", e),
            );
        }
    }
}
