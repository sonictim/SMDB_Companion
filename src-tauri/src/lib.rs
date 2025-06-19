pub mod commands;
pub mod preferences;
pub mod search;
pub mod windows;
pub use crate::audio::*;
pub mod audio;
pub use dirs::home_dir;
pub use preferences::*;
pub mod database;
pub mod prelude;
// pub use FFcodex::*;
pub use commands::*;
pub use regex::Regex;
pub use sqlx::Row;
pub use sqlx::mysql::{MySqlPool, MySqlRow};
pub use sqlx::sqlite::{SqlitePool, SqliteRow};
use std::hash::Hash;
// use tauri::App;
// use tauri::menu::{Menu, MenuBuilder, MenuItem, Submenu};

pub const SQLITE_TABLE: &str = "justinmetadata";
pub const MYSQL_TABLE: &str = "metadata";
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
            clear_selected_fingerprints,
            refresh_all_windows,
            open_database_folder,
            reveal_files,
            check_folder_exists,
            test_server_database,
            search_file_system,
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

static FILENAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?P<base>.+?)(?:\.(?:\d+|M))*$").unwrap());

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

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Default)]
pub enum Delete {
    #[default]
    Keep,
    Trash,
    Delete,
    Archive(String),
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
            app.substatus("Removing Files", 100, "No files to process");
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
            Delete::Archive(archive_folder) => {
                println!("Archiving files to: {}", archive_folder);
                app.substatus(
                    "Removing Files",
                    10,
                    &format!(
                        "Archiving {} files to: {}",
                        valid_files.len(),
                        archive_folder
                    ),
                );

                for (i, file) in valid_files.iter().enumerate() {
                    app.substatus(
                        "Removing Files",
                        10 + (i * 90 / valid_files.len()),
                        &format!("Archiving file {}/{}", i + 1, valid_files.len()),
                    );

                    let source_path = std::path::Path::new(file);

                    // Process the path based on OS
                    let processed_path = if cfg!(target_os = "windows") {
                        // Remove drive letter on Windows (e.g., "C:\folder\file.wav" -> "\folder\file.wav")
                        if let Some(path_str) = source_path.to_str() {
                            if path_str.len() >= 3 && path_str.chars().nth(1) == Some(':') {
                                // Remove drive letter and colon (e.g., "C:" -> "")
                                &path_str[2..]
                            } else {
                                path_str
                            }
                        } else {
                            file
                        }
                    } else if cfg!(target_os = "macos") {
                        // Remove /Volumes prefix on macOS for removable drives
                        if let Some(path_str) = source_path.to_str() {
                            if path_str.starts_with("/Volumes/") {
                                // Remove "/Volumes" prefix (e.g., "/Volumes/Drive/folder/file.wav" -> "/Drive/folder/file.wav")
                                &path_str[8..] // Remove "/Volumes" (8 characters)
                            } else {
                                path_str
                            }
                        } else {
                            file
                        }
                    } else {
                        // For other platforms, use the original path
                        file
                    };

                    // Create the destination path
                    let dest_path = std::path::Path::new(archive_folder)
                        .join(processed_path.trim_start_matches(['/', '\\']));

                    // Ensure the destination directory exists
                    if let Some(dest_dir) = dest_path.parent() {
                        if let Err(e) = std::fs::create_dir_all(dest_dir) {
                            eprintln!(
                                "Failed to create archive directory {}: {}",
                                dest_dir.display(),
                                e
                            );
                            app.substatus(
                                "Removing Files",
                                10 + (i * 90 / valid_files.len()),
                                &format!(
                                    "Error: Failed to create directory: {}",
                                    dest_dir.display()
                                ),
                            );
                            continue;
                        }
                    }

                    // Move the file
                    match std::fs::rename(file, &dest_path) {
                        Ok(_) => {
                            println!("Successfully archived: {} -> {}", file, dest_path.display());
                            app.substatus(
                                "Removing Files",
                                10 + (i * 90 / valid_files.len()),
                                &format!(
                                    "Archived: {}",
                                    source_path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                ),
                            );
                        }
                        Err(e) => {
                            eprintln!("Failed to archive file {}: {}", file, e);
                            app.substatus(
                                "Removing Files",
                                10 + (i * 90 / valid_files.len()),
                                &format!(
                                    "Error archiving: {}",
                                    source_path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                ),
                            );
                        }
                    }
                }
            }
            _ => {}
        }

        app.substatus("Removing Files", 100, "File removal complete");
        Ok(())
    }
}

fn get_column_as_string_sqlite(row: &SqliteRow, column: &str) -> Option<Arc<str>> {
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
fn get_column_as_string_mysql(row: &MySqlRow, column: &str) -> Option<Arc<str>> {
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
                    SQLITE_TABLE, column
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
