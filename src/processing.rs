#![allow(non_snake_case)]
use sqlx::Sqlite;
use sqlx::{sqlite::SqlitePool, Row, Error};
use tokio::sync::mpsc;
use std::collections::HashSet;
use std::collections::HashMap;
// use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
// use std::path::Path;
// use std::error::Error;


use regex::Regex;
use crate::app::*;

const TABLE: &str = "justinmetadata";

pub async fn smreplace_get(pool: &SqlitePool, find: &mut String, column: &mut String ) -> Result<usize, sqlx::Error>  {
    
    let search_query = format!("SELECT COUNT(rowid) FROM {} WHERE {} LIKE ?", TABLE, column);
    let result: (i64,) = sqlx::query_as(&search_query)
        .bind(format!("%{}%", find))
        .fetch_one(pool)
        .await?;

    Ok(result.0 as usize)
}

pub async fn smreplace_process(pool: &SqlitePool, find: &mut String, replace: &mut String, column: &mut String, dirty: bool ) {
   
    let dirty_text = if dirty { ", _Dirty = 1" } else { "" };

    let replace_query = format!(
        "UPDATE {} SET {} = REPLACE({}, '{}', '{}'){} WHERE {} LIKE '%{}%'", 
        TABLE, column, column, find, replace, dirty_text, column, find
    );
    let _ = sqlx::query(&replace_query).execute(pool).await;

}

pub async fn gather_duplicate_filenames_in_database(
    pool: &SqlitePool, 
    order: Vec<String>, 
    group_sort: &Option<String>, 
    group_null: bool, 
    verbose: bool
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    
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
                if verbose {
                    println!("Records without a {} entry will be processed together.", group);
                }
                String::new()
            } else {
                if verbose {
                    println!("Records without a {} entry will be skipped.", group);
                }
                format!("WHERE {} IS NOT NULL AND {} != ''", group, group)
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
                ROW_NUMBER() OVER (
                    PARTITION BY {}
                    ORDER BY {}
                ) as rn
            FROM justinmetadata
            {}
        )
        SELECT id, filename, duration FROM ranked WHERE rn > 1
        ",
        partition_by, order_clause, where_clause
    );

    // Execute the query and fetch the results
    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await?;
    
    // Iterate through the rows and insert them into the hashset
    for row in rows {
        let id: u32 = row.get(0);
        let file_record = FileRecord {
            id: id as usize,
            filename: row.get(1),
            duration: row.try_get(2).unwrap_or("".to_string()),  // Handle possible NULL in duration
        };
        file_records.insert(file_record);
    }

    if verbose {
        println!("Marked {} duplicate records for deletion.", file_records.len());
    }

    Ok(file_records)
}


pub async fn gather_deep_dive_records(pool: &SqlitePool, progress_sender: mpsc::Sender<ProgressMessage>,) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let mut file_records = HashSet::new();
    let mut file_groups: HashMap<String, Vec<FileRecord>> = HashMap::new();

    let query = "SELECT rowid, filename, duration FROM justinmetadata";

    let rows = sqlx::query(query)
    .fetch_all(pool)
    .await?;

    let total = rows.len();
    let mut counter: usize = 1;
    for row in &rows {
        let id: u32 = row.get(0);
        let file_record = FileRecord {
            id: id as usize,
            filename: row.get(1),
            duration: row.try_get(2).unwrap_or("".to_string()),  // Handle possible NULL in duration
        };
        
        let base_filename = get_root_filename(&file_record.filename)
            .unwrap_or_else(|| file_record.filename.clone());


        file_groups
            .entry(base_filename)
            .or_insert_with(Vec::new)
            .push(file_record);

            // let _ = io::stdout().flush();
            // print!("\r{} / {}", counter, total);
            let _ = progress_sender.send(ProgressMessage::Update(counter, total)).await;
            counter += 1;   
    }

    for (root, records) in file_groups {
        if records.len() <= 1 {continue;}   
        let root_found = records.iter().any(|record| record.filename == root);
        if root_found {
            file_records.extend(
                records.into_iter().filter(|record| record.filename != root)
            );
        } else {
            file_records.extend(
                records.into_iter().skip(1)
            );
        }
    }

    Ok(file_records)
}




pub async fn gather_filenames_with_tags(pool: &SqlitePool, tags: &Vec<String>) -> Result<HashSet<FileRecord>, sqlx::Error>  {
        // tags.status = format!("Searching for filenames containing tags");
        println!("Tokio Start");
        let mut file_records = HashSet::new();

        for tag in tags {
            let query = "SELECT rowid, filename, duration FROM justinmetadata WHERE filename LIKE '%' || ? || '%'";
    
            // Execute the query and fetch rows
            let rows = sqlx::query(query)
                .bind(tag.clone())
                .fetch_all(pool)
                .await?;
    
            // Collect file records from the query result
            for row in rows {
                let id: u32 = row.get(0);
                let file_record = FileRecord {
                    id: id as usize,
                    filename: row.get(1),
                    duration: row.try_get(2).unwrap_or("".to_string()),  // Handle possible NULL in duration
                };
                file_records.insert(file_record);
            }
        }
        println!("Found Tags");
        Ok(file_records)
        // tags.status = format!("{} total records containing tags marked for deletion", tags.records.len());
}

pub async fn gather_compare_database_overlaps(
        target_pool: &SqlitePool,
        compare_pool: &SqlitePool
    ) -> Result<HashSet<FileRecord>, sqlx::Error> {
        let compare_records = fetch_all_filerecords_from_database(compare_pool).await?;
        let filenames_to_check = extract_filenames_set_from_records(&compare_records);
        
        let mut matching_records = fetch_all_filerecords_from_database(target_pool).await?;
        println!(
            "Comparing filenames between databases"
        );
        
        matching_records.retain(|record| filenames_to_check.contains(&record.filename));
    
        if matching_records.is_empty() {
            println!("NO OVERLAPPING FILE RECORDS FOUND!");
        } else {
            println!(
                "Found {} overlapping file records.",
                matching_records.len()
            );
        }
    
        Ok(matching_records.into_iter().collect())
}

pub async fn fetch_filerecords_from_database(pool: &SqlitePool, query: &str) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let mut file_records = HashSet::new();

    let rows = sqlx::query(query)
    .fetch_all(pool)
    .await?;

    for row in rows {
        let id: u32 = row.get(0);
        let file_record = FileRecord {
            id: id as usize,
            filename: row.get(1),
            duration: row.try_get(2).unwrap_or("".to_string()),  // Handle possible NULL in duration
        };
        file_records.insert(file_record);
    }
    Ok(file_records)
}
pub async fn fetch_all_filerecords_from_database(pool: &SqlitePool) -> Result<HashSet<FileRecord>, sqlx::Error> {
    println!("Gathering all records from database");
    fetch_filerecords_from_database(pool, "SELECT rowid, filename, duration FROM justinmetadata").await
}

fn extract_filenames_set_from_records(file_records: &HashSet<FileRecord>) -> HashSet<String> {
    file_records.iter().map(|record| record.filename.clone()).collect()
}

pub async fn delete_file_records(pool: &SqlitePool, records: &HashSet<FileRecord>, progress_sender: mpsc::Sender<ProgressMessage>) -> Result<(), sqlx::Error> {
    const CHUNK_SIZE: usize = 12321;
    // Extract IDs from records
    let ids: Vec<i64> = records.into_iter().map(|record| record.id as i64).collect();

    // Check if we have any IDs to process
    if ids.is_empty() {
        return Ok(());
    }

    let mut counter = 0;
    let total = records.len();

    // Process IDs in chunks
    for chunk in ids.chunks(CHUNK_SIZE) {
        // Construct the SQL query with placeholders for the chunk
        let query = format!(
            "DELETE FROM justinmetadata WHERE rowid IN ({})",
            chunk.iter()
                .map(|_| "?")
                .collect::<Vec<&str>>()
                .join(", ")
        );

        // Prepare the query with bound parameters
        let mut query = sqlx::query(&query);
        for &id in chunk {
            query = query.bind(id);
        }

        // Execute the deletion
        match query.execute(pool).await {
            Ok(_) => {
                counter += CHUNK_SIZE;
                if counter >= total { counter = total; }
                let _ = progress_sender.send(ProgressMessage::Update(counter, total)).await;
                println!("Processed {} / {}", counter, total);
            },
            Err(e) => {
                eprintln!("Failed to execute query: {}", e);
                return Err(e);
            }
        }
    }
    println!("done inside delete function");
    sqlx::query("VACUUM").execute(pool).await; // Execute VACUUM on the database
    println!("VACUUM done inside delete function");


    Ok(())
}

// pub async fn vacuum_db(pool: &SqlitePool) -> Result<(), sqlx::Error> { 
//     // println!("Cleaning up Database {}", get_connection_source_filepath(&conn));
//     Ok(())
// }


pub async fn create_duplicates_db(source_db_path: &str, dupe_records_to_keep: &HashSet<FileRecord>, progress_sender: mpsc::Sender<ProgressMessage>) -> Result<(), sqlx::Error> {
    println!("Generating Duplicates Only Database.  This can take awhile.");
    let duplicate_db_path = format!("{}_dupes.sqlite", &source_db_path.trim_end_matches(".sqlite"));
    fs::copy(&source_db_path, &duplicate_db_path).unwrap();
    let mut dupe_conn = SqlitePool::connect(&duplicate_db_path).await.expect("Pool did not open");
    
    if let Ok(mut dupe_records_to_delete) = fetch_all_filerecords_from_database(&dupe_conn).await {

        dupe_records_to_delete.retain(|record| !dupe_records_to_keep.contains(record));
        
        delete_file_records(&mut dupe_conn, &dupe_records_to_delete, progress_sender).await;
        // vacuum_db(&dupe_conn).await;
    }

    // println!("{} records moved to {}", get_db_size(&dupe_conn), duplicate_db_path);

    Ok(())
}


fn get_root_filename(filename: &str) -> Option<String> {
    // Use regex to strip off trailing pattern like .1, .M, but preserve file extension
    let re = Regex::new(r"^(?P<base>.+?)(\.\d+|\.\w+)+(?P<ext>\.\w+)$").unwrap();
    if let Some(caps) = re.captures(filename) {
        Some(format!("{}{}", &caps["base"], &caps["ext"]))
    } else {
        // If no match, return the original filename
        Some(filename.to_string())
    }
}

pub async fn open_db() -> Option<Database> {
    if let Some(path) = rfd::FileDialog::new().pick_file() {
        let db_path = path.display().to_string();
        if db_path.ends_with(".sqlite") {
            println!("Opening Database {}", db_path);
            let db = Database::open(db_path).await;
            return Some(db);
        }
    }    
    None
}

pub async fn get_db_size(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM justinmetadata")
        .fetch_one(pool)
        .await?;

    Ok(count.0 as usize)
}

pub async fn get_columns(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    // Query for table info using PRAGMA
    let columns = sqlx::query("PRAGMA table_info(justinmetadata);")
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

pub fn default_tags() -> Vec<String> {
const DEFAULT_TAGS_VEC: [&str; 44] = [
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
const TJF_TAGS_VEC: [&str; 48] = [
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