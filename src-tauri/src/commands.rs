pub use crate::prelude::*;
pub use rfd::FileDialog;
use std::process::Command;

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
    path: String,
    is_compare: bool,
) -> Result<Arc<str>, String> {
    let mut state = state.lock().await;
    state.db = Database::new(&path, is_compare).await;
    if let Some(name) = state.db.get_name() {
        return Ok(name);
    }
    Ok(Arc::from("Select Database"))
}
#[tauri::command]
pub async fn close_db(state: State<'_, Mutex<AppState>>) -> Result<Arc<str>, String> {
    let mut state = state.lock().await;
    state.db = Database::default();
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
    let (tx, rx) = tokio::sync::oneshot::channel();
    let app = app.clone();
    let enabled = enabled.clone();
    let pref = pref.clone();

    let db = {
        let state = state.lock().await;
        state.db.abort.store(false, Ordering::SeqCst);
        state.db.clone()
    };

    let handle = tokio::spawn(async move {
        let result = run_search(app, db, enabled, pref).await;
        let _ = tx.send(result);
    });

    // Don't use a loop - just await both in parallel once
    tokio::select! {
        // Poll for abortion periodically
        _ = async {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
            loop {
                interval.tick().await;
                let state = state.lock().await;
                if state.db.abort.load(Ordering::SeqCst) {
                    return;
                }
            }
        } => {
            println!("Detected abort request, cancelling search task");
            handle.abort();
            Err("Aborted".to_string())
        }

        // Wait for search to complete
        result = rx => {
            match result {
                Ok(result) => {
                   result
                }
                Err(_) => {
                    Err("Fingerprinting task aborted or failed".to_string())
                }
            }
        }
    }
}

async fn run_search(
    app: AppHandle,
    mut db: Database,
    enabled: Enabled,
    pref: Preferences,
) -> Result<Vec<FileRecordFrontend>, String> {
    println!("Starting Search");

    let _ = db.add_column("_fingerprint").await;
    let _ = db.add_column("_DualMono").await;

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

    app.status("starting", counter * 100 / total, "Starting search...");
    app.substatus("starting", 0, "Gathering records from database...");

    counter += 1;
    let _ = db.fetch_all_filerecords(&enabled, &pref, &app).await;
    if db.abort.load(Ordering::SeqCst) {
        println!("Aborting fingerprint scan - early exit");
        return Err("Aborted".to_string());
    }

    if enabled.dbcompare {
        app.status(
            "compare",
            counter * 100 / total,
            &format!("Comparing records against {}", enabled.compare_db),
        );

        counter += 1;
        db.compare_search(&enabled, &pref, &app).await;
    }
    if db.abort.load(Ordering::SeqCst) {
        println!("Aborting fingerprint scan - early exit");
        return Err("Aborted".to_string());
    }

    if enabled.basic {
        app.status(
            "dupes",
            counter * 100 / total,
            "Performing Duplicate Search",
        );

        counter += 1;
        db.dupe_search(&pref, &enabled, &app);

        app.substatus("starting", 10, "Sorting Records");

        db.records.sort_by(|a, b| a.root.cmp(&b.root));
    }
    if enabled.dual_mono {
        app.status(
            "dualm",
            counter * 100 / total,
            "Performing Dual Mono Search",
        );

        counter += 1;
        db.dual_mono_search(&pref, &app).await;
    }
    if db.abort.load(Ordering::SeqCst) {
        println!("Aborting fingerprint scan - early exit");
        return Err("Aborted".to_string());
    }
    if enabled.waveform {
        app.status(
            "waveform",
            counter * 100 / total,
            "Analyzing audio content for waveform analysis",
        );

        counter += 1;

        let _ = db.wave_search_chromaprint(&pref, &app).await;
    }
    {}
    app.status("complete", 100, "Search completed! Gathering Results");

    println!("Search Ended");
    Ok(db.records_2_frontend().await)
}

#[tauri::command]
pub async fn clear_fingerprints(state: State<'_, Mutex<AppState>>) -> Result<Arc<str>, String> {
    println!("Clearing Fingerprints");
    let state = state.lock().await;
    let _ = state.db.remove_column("_fingerprint").await;
    println!("Fingerprints Cleared");
    Ok(state.db.get_name().unwrap_or(Arc::from("Select Database")))
}

#[tauri::command]
pub async fn remove_records(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    clone: bool,
    clone_tag: String,
    records: Vec<usize>,
    delete: Delete,
    files: Vec<&str>,
    dual_mono: Vec<DualMono>,
    strip_dual_mono: bool,
) -> Result<Arc<str>, String> {
    println!("Removing Records");
    println!("Dual Mono: {:?}", dual_mono);
    let mut state = state.lock().await;

    if strip_dual_mono {
        app.rstatus("starting", 0, "Stripping Dual Mono Records...");

        let _ = state.db.clean_multi_mono(&app, &dual_mono).await;
    }

    if clone {
        app.rstatus(
            "starting",
            20,
            "Creating Safety Copy of Current Database...",
        );

        state.db = state.db.create_clone(&clone_tag).await;
    }
    app.rstatus("starting", 30, "Removing Records from Database...");

    let _ = state.db.remove(&records, &app).await;
    app.rstatus(
        "starting",
        70,
        match delete {
            Delete::Trash => "Moving files to Trash",
            Delete::Delete => "Deleting Files",
            Delete::Keep => "Cleaning up....",
        },
    );

    let _ = delete.delete_files(files, &app);

    println!("Remove Ended");
    app.rstatus("complete", 100, "Success! Removal is complete");

    Ok(state.db.get_name().unwrap_or(Arc::from("Select Database")))
}

#[tauri::command]
pub async fn find(
    state: State<'_, Mutex<AppState>>,
    find: String,
    column: String,
    case_sensitive: bool,
    pref: Preferences,
    app: AppHandle,
) -> Result<Vec<FileRecordFrontend>, String> {
    println!("Starting Search");

    // Use a scope to ensure the mutex is released promptly
    let _ = {
        let mut state = state.lock().await;
        let case = if case_sensitive { "GLOB" } else { "LIKE" };
        let query =
            // format!("SELECT rowid, filepath, duration FROM {TABLE} WHERE {column} {case} ?");
            format!("SELECT rowid, filepath, duration, _fingerprint, description, channels, bitdepth, samplerate, _DualMono  FROM {TABLE} WHERE {column} {case} ?");

        // Get pool with error handling
        let pool = state.db.get_pool().await.unwrap();

        // Execute query with error handling
        let rows = match sqlx::query(&query)
            .bind(if case_sensitive {
                format!("*{}*", find) // GLOB wildcard (*)
            } else {
                format!("%{}%", find) // LIKE wildcard (%)
            })
            .fetch_all(&pool)
            .await
        {
            Ok(rows) => rows,
            Err(e) => return Err(format!("Database query failed: {}", e)),
        };

        println!("{} Rows Found", rows.len());
        app.status("starting", 50, &format!("{} Records Found", rows.len()));

        // Add a timeout for processing to prevent hanging
        let processing_timeout = std::time::Duration::from_secs(60); // 60 second timeout

        // Process records with error handling and timeout
        let new_records: Vec<FileRecord> = match tokio::time::timeout(processing_timeout, async {
            rows.par_iter()
                .enumerate()
                .map(|(i, row)| {
                    app.substatus(
                        "processing",
                        i * 100 / rows.len(),
                        &format!("Processing: {}/{} Records", i, rows.len()),
                    );
                    let mut record = FileRecord::new(row, &Enabled::default(), &pref, true);
                    // Safely create record with error handling
                    record.algorithm.insert(A::Replace);
                    record.algorithm.remove(&A::Keep);
                    Some(record)
                })
                .filter_map(|record| record) // Remove None values
                .collect()
        })
        .await
        {
            Ok(records) => records,
            Err(_) => return Err("Processing timed out after 60 seconds".to_string()),
        };

        // Update the records in the database
        state.db.records = new_records;

        // Clone to avoid holding the lock longer than needed
        state.db.records.clone()
    };

    // Now state is unlocked, get results (which acquires its own lock)
    println!("Search Ended");
    get_results(state).await
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
            format!(
                "UPDATE {TABLE} SET FilePath = REPLACE(Filename, '{}', '{}'){} WHERE Filename {} ?",
                data.find, data.replace, dirty_text, case_text
            ),
            format!(
                "UPDATE {TABLE} SET Filename = REPLACE(Filename, '{}', '{}'){} WHERE Filename {} ?",
                data.find, data.replace, dirty_text, case_text
            ),
            format!(
                "UPDATE {TABLE} SET Pathname = REPLACE(Pathname, '{}', '{}'){} WHERE Pathname {} ?",
                data.find, data.replace, dirty_text, case_text
            ),
            format!(
                "UPDATE justinrdb_Pathname SET Pathname = REPLACE(Pathname, '{}', '{}'){} WHERE Pathname {} ?",
                data.find, data.replace, dirty_text, case_text
            ),
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

#[tauri::command]
pub async fn get_results(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<FileRecordFrontend>, String> {
    // Try to acquire lock without waiting
    let state = match state.try_lock() {
        Ok(guard) => guard,
        Err(_) => return Err("State is currently locked".into()),
    };

    // Use Rayon's parallel iterator to transform the records in parallel
    let results: Vec<FileRecordFrontend> = state
        .db
        .records
        .par_iter() // Parallel iterator from Rayon
        .map(|record| {
            let mut algorithm: Vec<_> = record.algorithm.iter().cloned().collect();
            algorithm.sort_by(|a, b| {
                if a == &A::Waveforms {
                    std::cmp::Ordering::Less
                } else if b == &A::Waveforms {
                    std::cmp::Ordering::Greater
                } else {
                    b.cmp(a)
                }
            });
            FileRecordFrontend {
                id: record.id,
                path: Arc::from(record.get_path()),
                filename: Arc::from(record.get_filename()),
                algorithm,
                duration: record.duration.clone(),
                description: record.description.clone(),
                bitdepth: record.bitdepth,
                samplerate: record.samplerate,
                channels: record.channels,
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
    let _ = Command::new("qlmanage")
        .args(["-p", &file_path])
        .spawn()
        .expect("Failed to open Quick Look")
        .wait();
}

#[tauri::command]
pub async fn cancel_search(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let state = state.lock().await;
    // *state.db.abort.write().await = true;
    state.db.abort.store(true, Ordering::SeqCst);
    println!("❌❌❌❌❌ ABORTING SEARCH!!!!!!!!!");

    Ok(String::from("Search Canceled"))
}

// use tauri::{AppHandle, Manager, Window};

#[tauri::command]
pub fn refresh_all_windows(app: AppHandle) {
    for (_label, window) in app.webview_windows() {
        let _ = window.eval("window.location.reload()");
    }
}
