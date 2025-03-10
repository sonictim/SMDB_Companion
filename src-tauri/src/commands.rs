use crate::*;

pub use rfd::FileDialog;
use std::process::Command;
pub use std::sync::Arc;
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Emitter};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command(rename_all = "snake_case")]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub fn get_current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn get_reg(data: Registration) -> Result<Arc<str>, String> {
    Ok(generate_license_key(&data.name, &data.email))
}

#[tauri::command]
pub async fn check_reg(data: Registration) -> Result<bool, String> {
    Ok(
        generate_license_key(&data.name, &data.email) == Arc::from(data.license.to_uppercase())
            || generate_license_key_old(&data.name, &data.email)
                == Arc::from(data.license.to_uppercase()),
    )
}

#[tauri::command]
pub async fn open_db(
    state: State<'_, Mutex<AppState>>,
    is_compare: bool,
) -> Result<Arc<str>, String> {
    let mut state = state.lock().await;
    let x = state.db.path.clone();
    state.db.open(is_compare).await;
    if x == state.db.path {
        return Err(String::from("no path change"));
    }
    if let Some(name) = state.db.get_name() {
        return Ok(name);
    }
    Ok(Arc::from("Select Database"))
}

#[tauri::command]
pub async fn get_db_name(state: State<'_, Mutex<AppState>>) -> Result<Arc<str>, String> {
    println!("Get DB Name");
    let state = state.lock().await;
    if let Some(path) = &state.db.path {
        return Ok(path.file_stem().unwrap().to_str().unwrap().into());
    }

    Ok(Arc::from("Select Database"))
}

#[tauri::command]
pub async fn get_db_size(state: State<'_, Mutex<AppState>>) -> Result<usize, String> {
    println!("Get DB Size");
    let state = state.lock().await;
    Ok(state.db.get_size())
}

#[tauri::command]
pub async fn get_records_size(state: State<'_, Mutex<AppState>>) -> Result<usize, String> {
    println!("Get records Size");
    let state = state.lock().await;
    Ok(state.db.get_records_size())
}

#[tauri::command]
pub async fn search(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    enabled: Enabled,
    pref: Preferences,
) -> Result<Vec<FileRecordFrontend>, String> {
    println!("Starting Search");

    // Count enabled algorithms
    let mut counter = 0;
    let mut total = 1;
    if enabled.basic {
        total += 1;
    }
    if enabled.waveform {
        total += 1;
    }
    if enabled.dbcompare {
        total += 1;
    }

    println!("Running {} algorithms", counter);

    // Emit initial status
    app.emit(
        "search-status",
        SearchStatus {
            stage: "starting".into(),
            progress: counter * 100 / total,
            message: "Starting search...".into(),
        },
    )
    .unwrap();

    // Run first phase
    {
        let mut state = state.lock().await;

        app.emit(
            "search-status",
            SearchStatus {
                stage: "starting".into(),
                progress: counter * 100 / total,
                message: "Gathering records from database...".into(),
            },
        )
        .unwrap();
        counter += 1;

        let _ = state.db.fetch_all_filerecords(&enabled, &pref, &app).await;
    }

    if enabled.dbcompare {
        let mut state = state.lock().await;
        app.emit(
            "search-status",
            SearchStatus {
                stage: "compare".into(),
                progress: counter * 100 / total,
                message: format!("Comparing records against {}", enabled.compare_db),
            },
        )
        .unwrap();
        counter += 1;
        state.db.compare_search(&enabled, &pref, &app).await;

        // Emit progress update
    }

    if enabled.waveform {
        let mut state = state.lock().await;

        app.emit(
            "search-status",
            SearchStatus {
                stage: "basic".into(),
                progress: counter * 100 / total,
                message: "Analyzing audio content for waveform analysis".into(),
            },
        )
        .unwrap();
        counter += 1;

        state.db.wave_search(&pref);

        // Emit progress update
    }

    if enabled.basic {
        let mut state = state.lock().await;
        app.emit(
            "search-status",
            SearchStatus {
                stage: "dupes".into(),
                progress: counter * 100 / total,
                message: "Performing Duplicate Search".into(),
            },
        )
        .unwrap();
        counter += 1;
        state.db.dupe_search(&pref, &app);

        // Emit progress update
    }

    // Final update
    app.emit(
        "search-status",
        SearchStatus {
            stage: "complete".into(),
            progress: counter * 100 / total,
            message: "Search completed!  Gathering Results".into(),
        },
    )
    .unwrap();

    println!("Search Ended");
    get_results(state).await
}

// Define a struct for search status updates

#[tauri::command]
pub async fn remove_records(
    state: State<'_, Mutex<AppState>>,
    clone: bool,
    clone_tag: String,
    records: Vec<usize>,
    delete: Delete,
    files: Vec<&str>,
) -> Result<String, String> {
    println!("Removing Records");
    let mut state = state.lock().await;
    if clone {
        state.db = state.db.create_clone(&clone_tag).await;
    }
    let _ = state.db.remove(&records).await;
    let _ = delete.delete_files(files);
    println!("Remove Ended");
    Ok(String::from("Remove Success"))
}

#[tauri::command]
pub async fn find(
    state: State<'_, Mutex<AppState>>,
    find: String,
    column: String,
    case_sensitive: bool,
    pref: Preferences,
) -> Result<String, String> {
    println!("Starting Search");
    let mut state = state.lock().await;
    let case = if case_sensitive { "GLOB" } else { "LIKE" };
    let query = format!("SELECT rowid, filepath, duration FROM {TABLE} WHERE {column} {case} ?");
    let pool = state.db.get_pool().await.unwrap();
    let rows = sqlx::query(&query)
        .bind(if case_sensitive {
            format!("*{}*", find) // GLOB wildcard (*)
        } else {
            format!("%{}%", find) // LIKE wildcard (%)
        })
        .fetch_all(&pool)
        .await
        .unwrap();
    println!("{} Rows Found", rows.len());
    let new_records: Vec<FileRecord> = rows
        .par_iter()
        .map(|row| FileRecord::new(row, &Enabled::default(), &pref, true))
        .map(|mut record| {
            record.algorithm.insert(A::Replace);
            record.algorithm.remove(&A::Keep);
            record
        })
        .collect();
    state.db.records = new_records.into();
    println!("Search Ended");
    Ok(String::from("Find Success"))
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Metadata {
    find: String,
    replace: String,
    column: String,
    case_sensitive: bool,
    mark_dirty: bool,
}

#[tauri::command]
pub async fn replace_metadata(
    state: State<'_, Mutex<AppState>>,
    data: Metadata,
) -> Result<String, String> {
    println!("Starting Replace");
    let state = state.lock().await;
    let dirty_text = if data.mark_dirty
        && (data.column == "Filename" || data.column == "FilePath" || data.column == "Pathname")
    {
        ", _Dirty = 1"
    } else {
        ""
    };
    let case_text = if data.case_sensitive { "GLOB" } else { "LIKE" };

    let queries = if data.column == "Filename"
        || data.column == "FilePath"
        || data.column == "Pathname"
    {
        vec![
                    format!("UPDATE {TABLE} SET FilePath = REPLACE(Filename, '{}', '{}'){} WHERE Filename {} ?", data.find, data.replace, dirty_text, case_text),
                    format!("UPDATE {TABLE} SET Filename = REPLACE(Filename, '{}', '{}'){} WHERE Filename {} ?", data.find, data.replace, dirty_text, case_text ),
                    format!("UPDATE {TABLE} SET Pathname = REPLACE(Pathname, '{}', '{}'){} WHERE Pathname {} ?", data.find, data.replace, dirty_text, case_text),
                    format!("UPDATE justinrdb_Pathname SET Pathname = REPLACE(Pathname, '{}', '{}'){} WHERE Pathname {} ?", data.find, data.replace, dirty_text, case_text),
                ]
    } else {
        vec![format!(
            "UPDATE {TABLE} SET {} = REPLACE({}, '{}', '{}'){} WHERE {} {} ?",
            data.column, data.column, data.find, data.replace, dirty_text, data.column, case_text,
        )]
    };
    let pool = state.db.get_pool().await.unwrap();
    for query in queries {
        println!("{}", query);
        let result = sqlx::query(&query)
            .bind(if data.case_sensitive {
                format!("*{}*", data.find) // GLOB wildcard (*)
            } else {
                format!("%{}%", data.find) // LIKE wildcard (%)
            })
            .execute(&pool)
            .await;
        println!("{:?}", result);
    }

    println!("Replace Ended");
    Ok(String::from("Replace Success"))
}

use rayon::prelude::*;

#[tauri::command]
pub async fn get_results(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<FileRecordFrontend>, String> {
    // Try to acquire lock without waiting
    let state = match state.try_lock() {
        Ok(guard) => guard,
        Err(_) => return Err("State is currently locked".into()),
    };

    // Create a clone of the records to release the lock sooner
    // (This step might be optional depending on your architecture)
    // let records_snapshot = state.db.records.clone(); // Assuming records can be cloned efficiently

    // Use Rayon's parallel iterator to transform the records in parallel
    let results: Vec<FileRecordFrontend> = state
        .db
        .records
        .par_iter() // Parallel iterator from Rayon
        .map(|record| {
            let algorithm = record.algorithm.iter().cloned().collect();
            FileRecordFrontend {
                id: record.id,
                path: Arc::from(record.get_path()),
                root: Arc::from(record.get_filename()),
                algorithm,
            }
        })
        .collect(); // Parallel collect

    Ok(results)
}

#[tauri::command]
pub async fn get_columns(state: State<'_, Mutex<AppState>>) -> Result<Vec<Arc<str>>, String> {
    let state = state.lock().await;
    let columns = state.db.fetch_columns().await.unwrap_or(Vec::new());
    Ok(columns)
}

#[tauri::command]
pub fn open_quicklook(file_path: String) {
    Command::new("qlmanage")
        .args(&["-p", &file_path])
        .spawn()
        .expect("Failed to open Quick Look");
}
