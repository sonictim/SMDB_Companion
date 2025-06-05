pub mod commands;
pub mod database;
pub mod filerecord;
pub mod preferences;
pub mod search;
pub mod windows;
pub use crate::audio::*;
pub use crate::database::*;
pub use crate::filerecord::*;
pub mod audio;
pub use dirs::home_dir;
pub use preferences::*;
pub mod prelude;
pub use commands::*;
pub use regex::Regex;
use sqlx::{AnyPool, Row, any::AnyRow};
use std::hash::Hash;
// use tauri::App;
// use tauri::menu::{Menu, MenuBuilder, MenuItem, Submenu};

// pub const TABLE: &str = "justinmetadata";
pub const LOCAL_TABLE: &str = "justinmetadata";
pub const SERVER_TABLE: &str = "metadata";
pub const RECORD_DIVISOR: usize = 1231;

static FILENAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?P<base>.+?)(?:\.(?:\d+|M))*$").unwrap());

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    set_library_path();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Install SQLx Any drivers
            sqlx::any::install_default_drivers();
            println!("✅ SQLx Any drivers installed successfully");

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
            open_server_db,
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

fn get_column_as_string(row: &AnyRow, column: &str) -> Option<Arc<str>> {
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
