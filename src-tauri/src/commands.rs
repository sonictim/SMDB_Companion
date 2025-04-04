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

    app.emit(
        "search-status",
        StatusUpdate {
            stage: "starting".into(),
            progress: counter * 100 / total,
            message: "Starting search...".into(),
        },
    )
    .ok();
    app.emit(
        "search-sub-status",
        StatusUpdate {
            stage: "starting".into(),
            progress: 0,
            message: "Gathering records from database...".into(),
        },
    )
    .ok();

    counter += 1;
    let _ = db.fetch_all_filerecords(&enabled, &pref, &app).await;
    if db.abort.load(Ordering::SeqCst) {
        println!("Aborting fingerprint scan - early exit");
        return Err("Aborted".to_string());
    }

    if enabled.dbcompare {
        app.emit(
            "search-status",
            StatusUpdate {
                stage: "compare".into(),
                progress: counter * 100 / total,
                message: format!("Comparing records against {}", enabled.compare_db),
            },
        )
        .unwrap();
        counter += 1;
        db.compare_search(&enabled, &pref, &app).await;
    }
    if db.abort.load(Ordering::SeqCst) {
        println!("Aborting fingerprint scan - early exit");
        return Err("Aborted".to_string());
    }

    if enabled.basic {
        app.emit(
            "search-status",
            StatusUpdate {
                stage: "dupes".into(),
                progress: counter * 100 / total,
                message: "Performing Duplicate Search".into(),
            },
        )
        .unwrap();
        counter += 1;
        db.dupe_search(&pref, &enabled, &app);

        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "starting".into(),
                progress: 10,
                message: "Sorting Records".into(),
            },
        )
        .unwrap();

        db.records.sort_by(|a, b| a.root.cmp(&b.root));
    }
    if enabled.dual_mono {
        app.emit(
            "search-status",
            StatusUpdate {
                stage: "dualm".into(),
                progress: counter * 100 / total,
                message: "Performing Dual Mono Search".into(),
            },
        )
        .unwrap();
        counter += 1;
        db.dual_mono_search(&app).await;
    }
    if db.abort.load(Ordering::SeqCst) {
        println!("Aborting fingerprint scan - early exit");
        return Err("Aborted".to_string());
    }
    if enabled.waveform {
        app.emit(
            "search-status",
            StatusUpdate {
                stage: "basic".into(),
                progress: counter * 100 / total,
                message: "Analyzing audio content for waveform analysis".into(),
            },
        )
        .unwrap();
        counter += 1;

        let _ = db.wave_search_chromaprint(&pref, &app).await;
    }
    {}

    app.emit(
        "search-status",
        StatusUpdate {
            stage: "complete".into(),
            progress: counter * 100 / total,
            message: "Search completed!  Gathering Results".into(),
        },
    )
    .unwrap();
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

    if clone {
        app.emit(
            "remove-status",
            StatusUpdate {
                stage: "starting".into(),
                progress: 20,
                message: "Creating Safety Copy of Current Database...".into(),
            },
        )
        .ok();
        state.db = state.db.create_clone(&clone_tag).await;
    }
    app.emit(
        "remove-status",
        StatusUpdate {
            stage: "starting".into(),
            progress: 40,
            message: "Removing Records from {}...".into(),
        },
    )
    .ok();
    let _ = state.db.remove(&records, &app).await;
    app.emit(
        "remove-status",
        StatusUpdate {
            stage: "starting".into(),
            progress: 70,
            message: match delete {
                Delete::Trash => "Moving files to Trash",
                Delete::Delete => "Deleting Files",
                Delete::Keep => "Cleaning up....",
            }
            .into(),
        },
    )
    .ok();
    let _ = delete.delete_files(files, &app);

    if strip_dual_mono {
        app.emit(
            "remove-status",
            StatusUpdate {
                stage: "starting".into(),
                progress: 85,
                message: "Stripping Dual Mono Records...".into(),
            },
        )
        .ok();
        let _ = state.db.clean_multi_mono(&app, &dual_mono).await;
    }
    println!("Remove Ended");
    app.emit(
        "remove-status",
        StatusUpdate {
            stage: "complete".into(),
            progress: 100,
            message: "Success! Removal is complete".into(),
        },
    )
    .ok();

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
    {
        let mut state = state.lock().await;
        let case = if case_sensitive { "GLOB" } else { "LIKE" };
        let query =
            format!("SELECT rowid, filepath, duration FROM {TABLE} WHERE {column} {case} ?");
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
        app.emit(
            "search-status",
            StatusUpdate {
                stage: "starting".into(),
                progress: 50,
                message: format!("{} Records Found", rows.len()),
            },
        )
        .ok();
        let new_records: Vec<FileRecord> = rows
            .par_iter()
            .enumerate()
            .map(|(i, row)| {
                app.emit(
                    "search-sub-status",
                    StatusUpdate {
                        stage: "processing".into(),
                        progress: (i * 100 / rows.len()) as u64,
                        message: format!("Processing: {}/{} Records", i, rows.len()),
                    },
                )
                .ok();

                FileRecord::new(row, &Enabled::default(), &pref, true)
            })
            .map(|mut record| {
                record.algorithm.insert(A::Replace);
                record.algorithm.remove(&A::Keep);
                record
            })
            .collect();
        state.db.records = new_records;
    }
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
