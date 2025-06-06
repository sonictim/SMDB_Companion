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
// #[tauri::command]
// pub async fn open_server_db(
//     state: State<'_, Mutex<AppState>>,
//     server: ServerDb,
//     is_compare: bool,
// ) -> Result<Arc<str>, String> {
//     let mut state = state.lock().await;
//     state.db = Database::new_server(&server, is_compare).await;
//     if let Some(name) = state.db.get_name() {
//         return Ok(name);
//     }
//     Ok(Arc::from("Select Database"))
// }

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

    Ok(state
        .db
        .get_name()
        .unwrap_or_else(|| Arc::from("Select Database")))
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
) -> Result<Vec<Vec<FileRecordFrontend>>, String> {
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
                Err(e) => {
                    Err(format!("Fingerprinting task aborted or failed: {}", e))
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
) -> Result<Vec<Vec<FileRecordFrontend>>, String> {
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

    app.status("Starting", counter * 100 / total, "Starting search...");
    app.substatus("Starting", 0, "Gathering records from database...");
    counter += 1;

    let _ = db.fetch_all_filerecords(&enabled, &pref, &app).await;
    if db.abort.load(Ordering::SeqCst) {
        println!("Aborting fingerprint scan - early exit");
        return Err("Aborted".to_string());
    }

    if enabled.dbcompare {
        counter += 1;
        app.status(
            "Compare Database",
            counter * 100 / total,
            &format!("Comparing records against {}", enabled.compare_db),
        );

        db.compare_search(&enabled, &pref, &app).await;
    }
    if db.abort.load(Ordering::SeqCst) {
        println!("Aborting fingerprint scan - early exit");
        return Err("Aborted".to_string());
    }

    if enabled.basic {
        counter += 1;
        app.status(
            "Duplicate Search",
            counter * 100 / total,
            "Performing Duplicate Search",
        );

        db.dupe_search(&pref, &enabled, &app);

        app.substatus("Duplicate Search", 10, "Sorting Records");

        db.records.sort_by(|a, b| a.root.cmp(&b.root));
    }
    if enabled.dual_mono {
        counter += 1;
        app.status(
            "Dual Mono Search",
            counter * 100 / total,
            "Performing Dual Mono Search",
        );

        db.dual_mono_search(&pref, &app).await;
    }
    if db.abort.load(Ordering::SeqCst) {
        println!("Aborting fingerprint scan - early exit");
        return Err("Aborted".to_string());
    }
    if enabled.waveform {
        counter += 1;
        app.status(
            "Audio Content Search",
            counter * 100 / total,
            "Analyzing audio content for waveform analysis",
        );

        let _ = db.wave_search_chromaprint(&pref, &app).await;
    }
    {}
    app.status("Final Checks", 100, "Search completed! Gathering Results");

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
pub async fn clear_selected_fingerprints(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    pref: Preferences,
    rows: Vec<usize>,
) -> Result<(), String> {
    println!("Clearing Fingerprints for selected records");
    let state = state.lock().await;
    let _ = state
        .db
        .batch_update_column(&app, &pref, &rows, "_fingerprint", "NULL")
        .await;
    println!("Fingerprints Cleared");
    Ok(())
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
    app.substatus("Starting", 0, "Nothing to report here...");

    if strip_dual_mono {
        app.status("Dual Mono Processing", 0, "Stripping Dual Mono Records...");
        match state.db.clean_multi_mono(&app, &dual_mono).await {
            Ok(_) => {
                app.status("Dual Mono Processing", 100, "Dual Mono Records Stripped");
            }
            Err(e) => {
                let error_msg = format!("Failed to strip dual mono records: {}", e);
                // Check if it's a permission error
                if is_permission_error_sqlx(&e) {
                    return Err(format!("PERMISSION_ERROR: {}", error_msg));
                }
                app.status("Dual Mono Processing", 100, &error_msg);
                return Err(error_msg);
            }
        }
    }

    if clone {
        app.status(
            "Database Cloning",
            20,
            "Creating Safety Copy of Current Database...",
        );

        match state.db.create_clone(&clone_tag).await {
            Ok(cloned_db) => {
                state.db = cloned_db;
            }
            Err(e) => {
                let error_msg = format!("Failed to create database clone: {}", e);
                // Check if it's a permission error
                if is_permission_error(&e) {
                    return Err(format!("PERMISSION_ERROR: {}", error_msg));
                }
                app.status("Database Cloning", 100, &error_msg);
                return Err(error_msg);
            }
        }
    }

    app.status("Record Removal", 30, "Removing Records from Database...");
    match state.db.remove(&records, &app).await {
        Ok(_) => {
            app.status("Record Removal", 100, "Records Removed Successfully");
        }
        Err(e) => {
            let error_msg = format!("Failed to remove records: {}", e);
            // Check if it's a permission error
            if is_permission_error_sqlx(&e) {
                return Err(format!("PERMISSION_ERROR: {}", error_msg));
            }
            app.status("Record Removal", 100, &error_msg);
            return Err(error_msg);
        }
    }

    app.status(
        "Audio File Management",
        70,
        match delete {
            Delete::Trash => "Moving files to Trash",
            Delete::Delete => "Deleting Files",
            Delete::Keep => "Cleaning up....",
        },
    );

    // Handle file deletion with permission checking
    match delete.delete_files(files, &app) {
        Ok(_) => {}
        Err(e) => {
            if is_permission_error_str(&e.to_string()) {
                return Err(format!("PERMISSION_ERROR: Failed to delete files: {}", e));
            } else {
                return Err(format!("File operation failed: {}", e));
            }
        }
    }

    println!("Remove Ended");
    app.status("Final Checks", 100, "Success! Removal is complete");

    Ok(state.db.get_path().unwrap_or(Arc::from("Select Database")))
}

// Helper functions to detect permission errors
fn is_permission_error(error: &std::io::Error) -> bool {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => true,
        _ => {
            let error_str = error.to_string().to_lowercase();
            error_str.contains("permission")
                || error_str.contains("access denied")
                || error_str.contains("unauthorized")
        }
    }
}

fn is_permission_error_sqlx(error: &sqlx::Error) -> bool {
    let error_str = error.to_string().to_lowercase();
    error_str.contains("permission")
        || error_str.contains("access denied")
        || error_str.contains("unauthorized")
        || error_str.contains("readonly")
        || error_str.contains("database is locked")
}

fn is_permission_error_str(error_str: &str) -> bool {
    let error_lower = error_str.to_lowercase();
    error_lower.contains("permission")
        || error_lower.contains("access denied")
        || error_lower.contains("unauthorized")
        || error_lower.contains("operation not permitted")
}

#[tauri::command]
pub async fn find(
    state: State<'_, Mutex<AppState>>,
    app: AppHandle,
    find: String,
    column: String,
    case_sensitive: bool,
    pref: Preferences,
) -> Result<Vec<FileRecordFrontend>, String> {
    println!("Starting Search");
    app.status("Metadata Search", 0, "Searching for records...");
    app.substatus("Metadata Search", 0, "Preparing SQL query...");

    // Use a scope to ensure the mutex is released promptly
    let _ = {
        let mut state = state.lock().await;
        let table = state.db.get_table();
        let case = if case_sensitive { "GLOB" } else { "LIKE" };
        let query =
            // format!("SELECT recid, filepath, duration FROM {TABLE} WHERE {column} {case} ?");
            format!("SELECT recid, filepath, duration, _fingerprint, description, channels, bitdepth, samplerate, _DualMono  FROM {table} WHERE {column} {case} ?");

        // Get pool with error handling
        let pool = state.db.get_pool().await.unwrap();

        app.substatus("Metadata Search", 10, "Querying Database...");

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
        app.substatus(
            "Metadata Search",
            20,
            &format!("{} Records Found", rows.len()),
        );
        println!("{} Rows Found", rows.len());
        app.status(
            "Metadata Record Retrieval",
            50,
            &format!("{} Records Found", rows.len()),
        );

        // Add a timeout for processing to prevent hanging
        let processing_timeout = std::time::Duration::from_secs(60); // 60 second timeout

        // Process records with error handling and timeout
        let new_records: Vec<FileRecord> = match tokio::time::timeout(processing_timeout, async {
            rows.par_iter()
                .enumerate()
                .map(|(i, row)| {
                    app.substatus(
                        "Processing Records",
                        i * 100 / rows.len(),
                        &format!("Processing: {}/{} Records", i, rows.len()),
                    );
                    let mut record = FileRecord::new_sqlite(row, &Enabled::default(), &pref, true)?;
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
        app.status(
            "Metadata Record Retrieval",
            90,
            &format!("{} Records Processed", new_records.len()),
        );
        app.substatus(
            "Metadata Record Retrieval",
            100,
            &format!("{} Records Processed", new_records.len()),
        );
        // Update the records in the database
        state.db.records = new_records;
        app.status(
            "Metadata Record Retrieval",
            100,
            &format!("{} Records Processed", state.db.records.len()),
        );
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
    app: AppHandle,
    data: Metadata,
) -> Result<String, String> {
    println!("Starting Replace");
    let state = state.lock().await;
    let table = state.db.get_table();
    app.status(
        "Metadata Replacement",
        0,
        &format!(
            "Replacing '{}' with '{}' in column '{}'",
            data.find, data.replace, data.column
        ),
    );
    app.substatus("Metadata Replacement", 0, "Preparing SQL query...");

    let dirty_text = if data.mark_dirty
        && (data.column == "Filename" || data.column == "FilePath" || data.column == "Pathname")
    {
        ", _Dirty = 1"
    } else {
        ""
    };
    let case_text = if data.case_sensitive { "GLOB" } else { "LIKE" };

    let bind_value = if data.case_sensitive {
        format!("*{}*", data.find) // GLOB wildcard (*)
    } else {
        format!("%{}%", data.find) // LIKE wildcard (%)
    };

    let pool = state.db.get_pool().await.unwrap();

    if data.column == "Filename" || data.column == "FilePath" || data.column == "Pathname" {
        // First update the file paths
        let query = format!(
            "UPDATE {table} SET 
                FilePath = REPLACE(FilePath, '{}', '{}'),
                Filename = REPLACE(Filename, '{}', '{}'),
                Pathname = REPLACE(Pathname, '{}', '{}'){} 
            WHERE Filename {} ? OR FilePath {} ? OR Pathname {} ?",
            data.find,
            data.replace,
            data.find,
            data.replace,
            data.find,
            data.replace,
            dirty_text,
            case_text,
            case_text,
            case_text
        );

        println!("{}", query);
        app.substatus("Metadata Replacement", 10, "Updating file paths...");

        let result = sqlx::query(&query)
            .bind(&bind_value)
            .bind(&bind_value)
            .bind(&bind_value)
            .execute(&pool)
            .await;

        println!("Main table result: {:?}", result);

        // Then update the pathname table
        app.substatus("Metadata Replacement", 30, "Updating pathname table...");
        let pathname_query = format!(
            "UPDATE justinrdb_Pathname SET Pathname = REPLACE(Pathname, '{}', '{}') WHERE Pathname {} ?",
            data.find, data.replace, case_text
        );

        let pathname_result = sqlx::query(&pathname_query)
            .bind(&bind_value)
            .execute(&pool)
            .await;

        println!("Pathname table result: {:?}", pathname_result);

        // Finally, update the file path hashes
        app.substatus("Metadata Replacement", 50, "Updating file path hashes...");
        update_filepath_hash(&app, &data.find, &data.replace, case_text, &pool, table).await?;
    } else {
        // For non-file path columns, use the original simple update
        let query = format!(
            "UPDATE {table} SET {} = REPLACE({}, '{}', '{}'){} WHERE {} {} ?",
            data.column, data.column, data.find, data.replace, dirty_text, data.column, case_text,
        );

        app.substatus("Metadata Replacement", 10, "Querying Database...");

        let result = sqlx::query(&query).bind(&bind_value).execute(&pool).await;

        println!("{:?}", result);
    }

    app.substatus(
        "Metadata Replacement",
        100,
        "Replacement completed successfully",
    );
    app.status(
        "Final Checks",
        100,
        &format!(
            "Replaced '{}' with '{}' in column '{}'",
            data.find, data.replace, data.column
        ),
    );
    println!("Replace Ended");
    Ok(String::from("Replace Success"))
}

async fn update_filepath_hash(
    app: &AppHandle,
    find: &str,
    replace: &str,
    case_text: &str,
    pool: &sqlx::sqlite::SqlitePool,
    table: &str,
) -> Result<(), String> {
    use sha1::{Digest, Sha1};

    println!(
        "Hash update function called with find='{}', replace='{}'",
        find, replace
    );

    // Search for records that NOW contain the replacement string (after the update)
    let bind_value = if case_text == "GLOB" {
        format!("*{}*", replace) // GLOB wildcard (*)
    } else {
        format!("%{}%", replace) // LIKE wildcard (%)
    };

    let select_query = format!(
        "SELECT recid, FilePath FROM {table} WHERE Filename {} ? OR FilePath {} ? OR Pathname {} ?",
        case_text, case_text, case_text
    );

    app.substatus("Metadata Replacement", 50, "Fetching updated records...");

    let affected_rows = sqlx::query(&select_query)
        .bind(&bind_value) // Use replacement string, not original find string
        .bind(&bind_value)
        .bind(&bind_value)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to fetch affected records: {}", e))?;

    println!(
        "Found {} records containing replacement string '{}'",
        affected_rows.len(),
        replace
    );

    if affected_rows.is_empty() {
        app.substatus(
            "Metadata Replacement",
            90,
            "No records to update hashes for",
        );
        return Ok(());
    }

    // Rest of the function remains the same...
    app.substatus(
        "Metadata Replacement",
        60,
        &format!(
            "Calculating new hashes for {} records...",
            affected_rows.len()
        ),
    );

    let hashes: Vec<(i64, String)> = affected_rows
        .par_iter()
        .map(|row| {
            let recid: i64 = row.get(0);
            let file_path: String = row.get(1);

            // Generate SHA-1 hash of the NEW file path
            let mut hasher = Sha1::new();
            hasher.update(file_path.as_bytes());
            let hash = format!("{:x}", hasher.finalize());

            (recid, hash)
        })
        .collect();

    // Update hashes in batches for better performance
    app.substatus("Metadata Replacement", 70, "Updating file path hashes...");

    let batch_size = 10_000;
    let total_batches = hashes.len().div_ceil(batch_size);

    for (batch_num, chunk) in hashes.chunks(batch_size).enumerate() {
        let progress = 70 + (batch_num * 20 / total_batches);
        app.substatus(
            "Metadata Replacement",
            progress,
            &format!(
                "Updating batch {}/{} ({} hashes)",
                batch_num + 1,
                total_batches,
                chunk.len()
            ),
        );

        let mut update_query = format!("UPDATE {table} SET _FilePathHash = CASE recid ");

        for (recid, hash) in chunk {
            update_query.push_str(&format!("WHEN {} THEN '{}' ", recid, hash));
        }

        update_query.push_str("END WHERE recid IN (");
        update_query.push_str(
            &chunk
                .iter()
                .map(|(recid, _)| recid.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        );
        update_query.push(')');

        sqlx::query(&update_query)
            .execute(pool)
            .await
            .map_err(|e| {
                format!(
                    "Failed to update _FilePathHash batch {}: {}",
                    batch_num + 1,
                    e
                )
            })?;
    }

    app.substatus(
        "Metadata Replacement",
        90,
        &format!("Successfully updated {} file path hashes", hashes.len()),
    );

    Ok(())
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

#[tauri::command]
pub fn open_database_folder() -> Result<String, String> {
    // Get the user's home directory
    let home_dir = match home_dir() {
        Some(dir) => dir,
        None => return Err("Could not determine home directory".to_string()),
    };

    // Create platform-specific paths
    #[cfg(target_os = "windows")]
    let db_dir = home_dir
        .join("AppData")
        .join("Roaming")
        .join("SoundminerV6")
        .join("Databases");

    #[cfg(target_os = "macos")]
    let db_dir = home_dir
        .join("Library")
        .join("Application Support")
        .join("SoundminerV6")
        .join("Databases");

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let db_dir = home_dir.join(".soundminer").join("databases"); // Fallback for other platforms

    // Create the directory if it doesn't exist
    if !db_dir.exists() {
        match std::fs::create_dir_all(&db_dir) {
            Ok(_) => println!("Created directory: {}", db_dir.display()),
            Err(e) => return Err(format!("Failed to create directory: {}", e)),
        }
    }

    // Open the directory with the default file explorer
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        match Command::new("explorer").arg(&db_dir).spawn() {
            Ok(_) => return Ok(format!("Opened folder: {}", db_dir.display())),
            Err(e) => return Err(format!("Failed to open folder: {}", e)),
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        match Command::new("open").arg(&db_dir).spawn() {
            Ok(_) => Ok(format!("Opened folder: {}", db_dir.display())),
            Err(e) => Err(format!("Failed to open folder: {}", e)),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        return Err("Opening folders is not implemented for this platform".to_string());
    }
}

#[tauri::command]
pub fn reveal_files(paths: Vec<&str>) -> Result<String, String> {
    let mut success_count = 0;
    let mut errors = Vec::new();

    for path_str in paths {
        let path = PathBuf::from(path_str);
        println!("Revealing file: {}", path.display());

        if !path.exists() {
            errors.push(format!("File does not exist: {}", path.display()));
            continue;
        }

        let result = reveal_single_file(&path);
        match result {
            Ok(_) => success_count += 1,
            Err(e) => errors.push(e),
        }
    }

    if !errors.is_empty() {
        return Err(format!("Some files failed to open: {}", errors.join("; ")));
    }

    Ok(format!("Successfully revealed {} file(s)", success_count))
}

fn reveal_single_file(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        // Use /select to highlight the file in Explorer
        match Command::new("explorer")
            .args(["/select,", &path.display().to_string()])
            .spawn()
        {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to reveal file {}: {}", path.display(), e)),
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        // Use -R flag to reveal the file in Finder
        match Command::new("open")
            .args(["-R", &path.display().to_string()])
            .spawn()
        {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to reveal file {}: {}", path.display(), e)),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        // For Linux, try to use the file manager to show the parent directory
        use std::process::Command;

        let parent_dir = path.parent().unwrap_or(path);

        // Try different file managers in order of preference
        let file_managers = ["nautilus", "dolphin", "thunar", "pcmanfm", "xdg-open"];

        for fm in &file_managers {
            if let Ok(_) = Command::new("which").arg(fm).output() {
                match Command::new(fm).arg(parent_dir).spawn() {
                    Ok(_) => return Ok(()),
                    Err(_) => continue,
                }
            }
        }

        Err("No supported file manager found for this platform".to_string())
    }
}
