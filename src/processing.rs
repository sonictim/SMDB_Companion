#![allow(non_snake_case)]
use rfd::FileDialog;
// use sqlx::Sqlite;
use sqlx::{sqlite::SqlitePool, Row};
use std::collections::{HashMap, HashSet};
use std::result::Result;

// use futures::future::join_all;
use futures::stream::{self, StreamExt};
use rayon::prelude::*;
use std::path::Path;
// use std::sync::atomic::{AtomicUsize, Ordering};
// use std::sync::Arc;
use tokio::sync::mpsc;

use crate::app::*;
// use hex;
use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Digest, Sha256};

pub fn generate_license_key(username: &str, email: &str) -> String {
    let salt = "Valhalla Delay";
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}{}", username, email, salt).as_bytes());
    let hash = hasher.finalize();
    hex::encode_upper(hash)
}

static FILENAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?P<base>.+?)(?:\.(?:\d+|M))*$").unwrap());

fn get_root_filename(filename: &str, ignore_extension: bool) -> Option<String> {
    let path = Path::new(filename);
    let mut name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    // Use the regex to capture the base name
    if let Some(caps) = FILENAME_REGEX.captures(&name) {
        name = caps["base"].to_string();
    } else {
        println!("{} Did not match Regex", filename);
    }

    if ignore_extension {
        return Some(name);
    }

    // Reattach the extension if it's not being ignored
    Some(format!("{name}.{extension}"))
}

const TABLE: &str = "justinmetadata";

// pub fn wrap_async<F, T>(pool: &SqlitePool, config: &mut Config, label: &str, action: F)
pub fn wrap_async<F, T>(config: &mut Config, label: &str, action: F)
where
    // F: FnOnce(&SqlitePool, mpsc::Sender<ProgressMessage>) -> T + Send + 'static,
    F: FnOnce() -> T + Send + 'static,
    T: std::future::Future<Output = Result<HashSet<FileRecord>, sqlx::Error>> + Send + 'static,
{
    config.working = true;
    config.status = label.to_string();
    // let records = config.records.clone();
    if let Some(tx) = config.tx.clone() {
        // if let Some(sender) = config.progress_sender.clone() {
        // let pool = pool.clone();

        let handle = tokio::spawn(async move {
            println!("Inside Async Task");

            // let _results  = action(&pool, sender).await;
            let results = action().await;

            if (tx.send(results.expect("Tokio Results Error HashSet")).await).is_err() {
                eprintln!("Failed to send db");
            }
        });
        config.handle = Some(handle);
        // }
    }
}

pub async fn smreplace_get(
    pool: &SqlitePool,
    find: &mut String,
    column: &mut String,
    case_sensitive: bool,
) -> Result<usize, sqlx::Error> {
    let case = if case_sensitive { "GLOB" } else { "LIKE" };
    let search_query = format!("SELECT COUNT(rowid) FROM {TABLE} WHERE {column} {case} ?");
    let result: (i64,) = sqlx::query_as(&search_query)
        .bind(format!("%{}%", find))
        .fetch_one(pool)
        .await?;

    Ok(result.0 as usize)
}

pub async fn smreplace_process(
    pool: &SqlitePool,
    find: &mut String,
    replace: &mut String,
    column: &mut String,
    dirty: bool,
    is_filepath: bool,
    case_sensitive: bool,
) {
    let dirty_text = if dirty && !is_filepath {
        ", _Dirty = 1"
    } else {
        ""
    };
    let case_text = if case_sensitive { "GLOB" } else { "LIKE" };

    let replace_query = format!(
        "UPDATE {} SET {} = REPLACE({}, '{}', '{}'){} WHERE {} {} '%{}%'",
        TABLE, column, column, find, replace, dirty_text, column, case_text, find
    );
    let _ = sqlx::query(&replace_query).execute(pool).await;

    if is_filepath {
        let mut column = "Filename";
        let replace_query = format!(
            "UPDATE {} SET {} = REPLACE({}, '{}', '{}'){} WHERE {} {} '%{}%'",
            TABLE, column, column, find, replace, dirty_text, column, case_text, find
        );
        let _ = sqlx::query(&replace_query).execute(pool).await;

        column = "Pathname";
        let replace_query = format!(
            "UPDATE {} SET {} = REPLACE({}, '{}', '{}'){} WHERE {} {} '%{}%'",
            TABLE, column, column, find, replace, dirty_text, column, case_text, find
        );
        let _ = sqlx::query(&replace_query).execute(pool).await;

        let table = "justinrdb_Pathname";
        let replace_query = format!(
            "UPDATE {} SET {} = REPLACE({}, '{}', '{}'){} WHERE {} {} '%{}%'",
            table, column, column, find, replace, dirty_text, column, case_text, find
        );
        let _ = sqlx::query(&replace_query).execute(pool).await;
    }
}

pub async fn gather_duplicate_filenames_in_database(
    pool: SqlitePool,
    // config: Config,
    order: Vec<String>,
    group_sort: Option<String>,
    group_null: bool,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let verbose = true;
    // let order = config.list;
    // let mut group_sort = None;
    // if config.search {group_sort = Some(config.selected)}

    let mut file_records = HashSet::new();

    // Construct the ORDER BY clause dynamically
    let order_clause = order.join(", ");

    // Build the SQL query based on whether a group_sort is provided
    let (partition_by, where_clause) = match group_sort {
        Some(group) => {
            if verbose {
                println!("Grouping duplicate record search by {}", group);
            }
            let where_clause = if group_null {
                String::new()
            } else {
                format!("WHERE {group} IS NOT NULL AND {group} != ''")
            };
            (format!("{}, filename", group), where_clause)
        }
        None => ("filename".to_string(), String::new()),
    };

    let sql = format!(
        "
        WITH ranked AS (
            SELECT
                rowid AS id,
                filename,
                duration,
                filepath,
                ROW_NUMBER() OVER (
                    PARTITION BY {}
                    ORDER BY {}
                ) as rn
            FROM {}
            {}
        )
        SELECT id, filename, duration, filepath FROM ranked WHERE rn > 1
        ",
        partition_by, order_clause, TABLE, where_clause
    );

    // Execute the query and fetch the results
    let rows = sqlx::query(&sql).fetch_all(&pool).await?;

    // Iterate through the rows and insert them into the hashset
    for row in rows {
        let id: u32 = row.get(0);
        let file_record = FileRecord {
            id: id as usize,
            filename: row.get(1),
            duration: row.try_get(2).unwrap_or("".to_string()), // Handle possible NULL in duration
            path: row.get(3),
        };
        file_records.insert(file_record);
    }

    if verbose {
        println!(
            "Marked {} duplicate records for deletion.",
            file_records.len()
        );
    }

    Ok(file_records)
}

pub async fn gather_deep_dive_records(
    pool: SqlitePool,
    progress_sender: mpsc::Sender<ProgressMessage>,
    status_sender: mpsc::Sender<String>,
    ignore_extension: bool,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let mut file_groups: HashMap<String, Vec<FileRecord>> = HashMap::new();

    let query = &format!("SELECT rowid, filename, duration, filepath FROM {}", TABLE);

    let rows = sqlx::query(query).fetch_all(&pool).await?;

    let total = rows.len();
    let mut counter: usize = 0;

    // let _ = status_sender.send("Starting Parallel iterations".to_string()).await;

    // Use a parallel iterator to process the rows
    let processed_records: Vec<(String, FileRecord)> = rows
        .par_iter() // Use a parallel iterator
        .map(|row| {
            let id: u32 = row.get(0);
            let file_record = FileRecord {
                id: id as usize,
                filename: row.get(1),
                duration: row.try_get(2).unwrap_or("".to_string()), // Handle possible NULL in duration
                path: row.get(3),
            };

            let base_filename = get_root_filename(&file_record.filename, ignore_extension)
                .unwrap_or_else(|| file_record.filename.clone());

            (base_filename, file_record)
        })
        .collect(); // Collect results into a Vec<(String, FileRecord)>

    let _ = status_sender.send("Processing Records".to_string()).await;
    // Now merge the results into the file_groups (sequentially)
    for (base_filename, file_record) in processed_records {
        file_groups
            .entry(base_filename)
            .or_default()
            .push(file_record);

        counter += 1;

        // Send progress updates every 100 rows
        if counter % 100 == 0 {
            let _ = progress_sender
                .send(ProgressMessage::Update(counter, total))
                .await;
        }
    }

    let _ = status_sender.send("Finishing up".to_string()).await;
    // Now handle merging the file groups into a HashSet of file_records

    let mut file_records = HashSet::new();
    for (root, records) in file_groups {
        if records.len() <= 1 {
            continue;
        }

        let root_found = records.iter().any(|record| {
            if ignore_extension {
                let name = Path::new(&record.filename)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap();
                return name == root;
            }

            record.filename == root
        });
        if root_found {
            file_records.extend(records.into_iter().filter(|record| {
                if ignore_extension {
                    let name = Path::new(&record.filename)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap();
                    return name != root;
                }
                record.filename != root
            }));
        } else {
            file_records.extend(records.into_iter().skip(1));
        }
    }
    let _ = status_sender
        .send(format!("Found {} duplicate records", file_records.len()))
        .await;

    Ok(file_records)
}

pub async fn gather_filenames_with_tags(
    pool: SqlitePool,
    tags: Vec<String>,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let mut file_records = HashSet::new();
    let max_concurrency = 10; // Adjust based on your system's capacity and connection pool size

    // Process each tag concurrently with a controlled level of concurrency
    let results = stream::iter(tags.into_iter())
        .map(|tag| {
            let pool = pool.clone();
            async move {
                let query = format!(
                    "SELECT rowid, filename, duration, filepath FROM {} WHERE filename LIKE '%' || ? || '%'",
                    TABLE
                );
                sqlx::query(&query).bind(tag).fetch_all(&pool).await // Return the result (Result<Vec<sqlx::sqlite::SqliteRow>, sqlx::Error>)
            }
        })
        .buffer_unordered(max_concurrency) // Control the level of concurrency
        .collect::<Vec<Result<Vec<sqlx::sqlite::SqliteRow>, sqlx::Error>>>()
        .await;

    // Iterate over the results and insert the file records
    for result in results {
        match result {
            Ok(rows) => {
                for row in rows {
                    let id: u32 = row.get(0);
                    let file_record = FileRecord {
                        id: id as usize,
                        filename: row.get(1),
                        duration: row.try_get(2).unwrap_or("".to_string()), // Handle possible NULL in duration
                        path: row.get(3),
                    };
                    file_records.insert(file_record);
                }
            }
            Err(err) => {
                return Err(err); // Return early if an error occurs
            }
        }
    }

    println!("Found Tags");
    Ok(file_records)
}

pub async fn gather_compare_database_overlaps(
    target_pool: &SqlitePool,
    compare_pool: &SqlitePool,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let compare_records = fetch_all_filerecords_from_database(compare_pool).await?;
    let filenames_to_check = extract_filenames_set_from_records(&compare_records);

    let mut matching_records = fetch_all_filerecords_from_database(target_pool).await?;
    println!("Comparing filenames between databases");

    matching_records.retain(|record| filenames_to_check.contains(&record.filename));

    if matching_records.is_empty() {
        println!("NO OVERLAPPING FILE RECORDS FOUND!");
    } else {
        println!("Found {} overlapping file records.", matching_records.len());
    }

    Ok(matching_records.into_iter().collect())
}

pub async fn fetch_filerecords_from_database(
    pool: &SqlitePool,
    query: &str,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let mut file_records = HashSet::new();

    let rows = sqlx::query(query).fetch_all(pool).await?;

    for row in rows {
        let id: u32 = row.get(0);
        let file_record = FileRecord {
            id: id as usize,
            filename: row.get(1),
            duration: row.try_get(2).unwrap_or("".to_string()), // Handle possible NULL in duration
            path: row.get(3),
        };
        file_records.insert(file_record);
    }
    Ok(file_records)
}
pub async fn fetch_all_filerecords_from_database(
    pool: &SqlitePool,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    println!("Gathering all records from database");
    fetch_filerecords_from_database(
        pool,
        &format!("SELECT rowid, filename, duration, filepath FROM {}", TABLE),
    )
    .await
}

fn extract_filenames_set_from_records(file_records: &HashSet<FileRecord>) -> HashSet<String> {
    file_records
        .iter()
        .map(|record| record.filename.clone())
        .collect()
}

// use futures::future::join_all;
// use sqlx::Error;
// use std::sync::{
//     atomic::{AtomicUsize, Ordering},
//     Arc,
// };
// use tokio::task::JoinHandle;

pub async fn delete_file_records(
    pool: &SqlitePool,
    records: &HashSet<FileRecord>,
    progress_sender: mpsc::Sender<ProgressMessage>,
    status_sender: mpsc::Sender<String>,
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
        match query.execute(pool).await {
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
        let progress = std::cmp::min(current_count, total);

        let _ = progress_sender
            .send(ProgressMessage::Update(progress, total))
            .await;
        let _ = status_sender
            .send(format!("Processed {} / {}", progress, total))
            .await;
    }

    // After all deletions, perform the cleanup
    let _ = status_sender.send("Cleaning Up Database".to_string()).await;
    let _result = sqlx::query("VACUUM").execute(pool).await;

    println!("VACUUM done inside delete function");

    Ok(())
}

pub async fn create_duplicates_db(
    pool: &SqlitePool,
    dupe_records_to_keep: &HashSet<FileRecord>,
    progress_sender: mpsc::Sender<ProgressMessage>,
    status_sender: mpsc::Sender<String>,
) -> Result<(), sqlx::Error> {
    println!("Generating Duplicates Only Database. This can take a while.");
    let _ = status_sender
        .send("Creating Duplicates Only Database. This can be slow.".to_string())
        .await;

    if let Ok(all_records) = fetch_all_filerecords_from_database(pool).await {
        // Use a parallel iterator to process records
        let dupe_records_to_delete: HashSet<FileRecord> = all_records
            .par_iter() // Parallel iterator
            .filter(|record| !dupe_records_to_keep.contains(record)) // Filter out records to keep
            .cloned() // Clone the records to create a new HashSet
            .collect(); // Collect into a HashSet

        let _result = delete_file_records(
            pool,
            &dupe_records_to_delete,
            progress_sender,
            status_sender,
        )
        .await;
    }

    Ok(())
}

pub async fn open_db() -> Option<Database> {
    if let Some(path) = FileDialog::new()
        .add_filter("SQLite Database", &["sqlite"])
        .pick_file()
    {
        let db_path = path.display().to_string();
        if db_path.ends_with(".sqlite") {
            println!("Opening Database {}", db_path);
            let db = Database::open(&db_path).await;
            return Some(db);
        }
    }
    None
}

pub async fn get_db_size(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", TABLE))
        .fetch_one(pool)
        .await?;

    Ok(count.0 as usize)
}

pub async fn get_columns(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
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

pub fn default_tags() -> Vec<String> {
    const DEFAULT_TAGS_VEC: [&str; 45] = [
        "-1eqa_",
        "-6030_",
        "-7eqa_",
        "-A2sA_",
        "-A44m_",
        "-A44s_",
        "-Alt7S_",
        "-ASMA_",
        "-AVrP_",
        "-AVrT_",
        "-AVSt_",
        "-DEC4_",
        "-Delays_",
        "-Dn_",
        "-DUPL_",
        "-DVerb_",
        "-GAIN_",
        "-M2DN_",
        "-NORM_",
        "-NYCT_",
        "-PiSh_",
        "-PnT2_",
        "-PnTPro_",
        "-ProQ2_",
        "-PSh_",
        "-Reverse_",
        "-RVRS_",
        "-RING_",
        "-RX7Cnct_",
        "-spce_",
        "-TCEX_",
        "-TiSh_",
        "-TmShft_",
        "-VariFi_",
        "-VlhllVV_",
        "-VSPD_",
        "-VitmnMn_",
        "-VtmnStr_",
        "-X2mA_",
        "-X2sA_",
        "-XForm_",
        "-Z2N5_",
        "-Z2S5_",
        "-Z4n2_",
        "-ZXN5_",
    ];
    DEFAULT_TAGS_VEC.map(|s| s.to_string()).to_vec()
}

pub fn tjf_tags() -> Vec<String> {
    const TJF_TAGS_VEC: [&str; 49] = [
        "-1eqa_",
        "-6030_",
        "-7eqa_",
        "-A2sA_",
        "-A44m_",
        "-A44s_",
        "-Alt7S_",
        "-ASMA_",
        "-AVrP_",
        "-AVrT_",
        "-AVSt_",
        "-DEC4_",
        "-Delays_",
        "-Dn_",
        "-DUPL_",
        "-DVerb_",
        "-GAIN_",
        "-M2DN_",
        "-NORM_",
        "-NYCT_",
        "-PiSh_",
        "-PnT2_",
        "-PnTPro_",
        "-ProQ2_",
        "-PSh_",
        "-Reverse_",
        "-RVRS_",
        "-RING_",
        "-RX7Cnct_",
        "-spce_",
        "-TCEX_",
        "-TiSh_",
        "-TmShft_",
        "-VariFi_",
        "-VlhllVV_",
        "-VSPD_",
        "-VitmnMn_",
        "-VtmnStr_",
        "-X2mA_",
        "-X2sA_",
        "-XForm_",
        "-Z2N5_",
        "-Z2S5_",
        "-Z4n2_",
        "-ZXN5_",
        ".new.",
        ".aif.",
        ".mp3.",
        ".wav.",
    ];
    TJF_TAGS_VEC.map(|s| s.to_string()).to_vec()
}

pub fn default_order() -> Vec<String> {
    const DEFAULT_ORDER_VEC: [&str; 12] = [
        "CASE WHEN Description IS NOT NULL AND Description != '' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%Audio Files%' THEN 1 ELSE 0 END ASC",
        "CASE WHEN pathname LIKE '%LIBRARIES%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%LIBRARY%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%/LIBRARY%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%LIBRARY/%' THEN 0 ELSE 1 END ASC",
        "duration DESC",
        "channels DESC",
        "sampleRate DESC",
        "bitDepth DESC",
        "BWDate ASC",
        "scannedDate ASC",
    ];
    DEFAULT_ORDER_VEC.map(|s| s.to_string()).to_vec()
}
pub fn default_order_friendly() -> Vec<String> {
    const DEFAULT_ORDER_FRIENDLY: [&str; 12] = [
        "Description is NOT Empty",
        "Pathname does NOT contain 'Audio Files'",
        "Pathname contains 'LIBRARIES'",
        "Pathname contains 'LIBRARY'",
        "Pathname contains '/LIBRARY'",
        "Pathname contains 'LIBRARY/'",
        "Largest Duration",
        "Largest Channel Count",
        "Largest Sample Rate",
        "Largest Bit Depth",
        "Smallest BWDate",
        "Smallest Scanned Date",
    ];
    DEFAULT_ORDER_FRIENDLY.map(|s| s.to_string()).to_vec()
}

pub fn tjf_order() -> Vec<String> {
    const TJF_ORDER_VEC: [&str; 22] = [
        "CASE WHEN pathname LIKE '%TJF RECORDINGS%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%LIBRARIES%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%SHOWS/Tim Farrell%' THEN 1 ELSE 0 END ASC",
        "CASE WHEN Description IS NOT NULL AND Description != '' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%Audio Files%' THEN 1 ELSE 0 END ASC",
        "CASE WHEN pathname LIKE '%RECORD%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%CREATED SFX%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%CREATED FX%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%LIBRARY%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%/LIBRARY%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%LIBRARY/%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%SIGNATURE%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%PULLS%' THEN 0 ELSE 1 END ASC",
        "CASE WHEN pathname LIKE '%EDIT%' THEN 1 ELSE 0 END ASC",
        "CASE WHEN pathname LIKE '%MIX%' THEN 1 ELSE 0 END ASC",
        "CASE WHEN pathname LIKE '%SESSION%' THEN 1 ELSE 0 END ASC",
        "duration DESC",
        "channels DESC",
        "sampleRate DESC",
        "bitDepth DESC",
        "BWDate ASC",
        "scannedDate ASC",
    ];
    TJF_ORDER_VEC.map(|s| s.to_string()).to_vec()
}

pub fn tjf_order_friendly() -> Vec<String> {
    const TJF_ORDER_FRIENDLY: [&str; 22] = [
        "Pathname contains 'TJF RECORDINGS'",
        "Pathname contains 'LIBRARIES'",
        "Pathname does NOT contain 'SHOWS/Tim Farrell'",
        "Description is NOT Empty",
        "Pathname does NOT contain 'Audio Files'",
        "Pathname contains 'RECORD'",
        "Pathname contains 'CREATED SFX'",
        "Pathname contains 'CREATED FX'",
        "Pathname contains 'LIBRARY'",
        "Pathname contains '/LIBRARY'",
        "Pathname contains 'LIBRARY/'",
        "Pathname contains 'SIGNATURE'",
        "Pathname contains 'PULLS'",
        "Pathname does NOT contain 'EDIT'",
        "Pathname does NOT contain 'MIX'",
        "Pathname does NOT contain 'SESSION'",
        "Largest Duration",
        "Largest Channel Count",
        "Largest Sample Rate",
        "Largest Bit Depth",
        "Smallest BWDate",
        "Smallest Scanned Date",
    ];
    TJF_ORDER_FRIENDLY.map(|s| s.to_string()).to_vec()
}
