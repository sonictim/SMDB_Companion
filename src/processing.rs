#![allow(non_snake_case)]
use crate::prelude::*;
use rfd::FileDialog;
use sqlx::sqlite::SqliteRow;
use dirs::home_dir;
use futures::stream::{self, StreamExt};
use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Digest, Sha256};
use clipboard::{ClipboardContext, ClipboardProvider};
use reqwest::Client;
use std::error::Error;

pub fn generate_license_key(username: &str, email: &str) -> String {
    let salt = "Valhalla Delay";
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}{}", username, email, salt).as_bytes());
    let hash = hasher.finalize();
    hex::encode_upper(hash)
}

pub fn copy_to_clipboard(text: String) {
    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();

    ctx.set_contents(text).unwrap();

}

// pub fn remove_all_spaces(input: &str) -> String {
//     input.chars().filter(|c| !c.is_whitespace()).collect()
// }


pub async fn fetch_latest_version() -> Result<String, Box<dyn Error>> {
    let file_id = "1C8jyVjkMgeglYK-FnmTuoRqwf5Nd6PGG";
    let download_url = format!("https://drive.google.com/uc?export=download&id={}", file_id);
    let client = Client::new();

    let response = client.get(&download_url).send().await?;

    if response.status().is_success() {
        let content = response.text().await?;
        Ok(content.trim().to_string())
    } else {
        Err(format!("Failed to retrieve the file: {}", response.status()).into())
    }
}

pub fn open_download_url() {
    let url = r#"https://drive.google.com/open?id=1qdGqoUMqq_xCrbA6IxUTYliZUmd3Tn3i&usp=drive_fs"#;
    let _ = webbrowser::open(url).is_ok();
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

// const TABLE: &str = "justinmetadata";

// pub fn wrap_async<F, T>(config: &mut NodeConfig, action: F)
// where
//     F: FnOnce() -> T + Send + 'static,
//     T: std::future::Future<Output = Result<HashSet<FileRecord>, sqlx::Error>> + Send + 'static,
// {
//     config.working = true;
//     let tx = config.records_io.tx.clone();

//         let handle = tokio::spawn(async move {
//             let results = action().await;
//             if (tx.send(results.expect("Tokio Results Error HashSet")).await).is_err() {
//                 eprintln!("Failed to send db");
//             }
//         });
//         config.handle = Some(handle);
        
    
// }

// pub async fn smreplace_get(
//     pool: &SqlitePool,
//     find: &mut String,
//     column: &mut String,
//     case_sensitive: bool,
// ) -> Result<usize, sqlx::Error> {
//     let case = if case_sensitive { "GLOB" } else { "LIKE" };
//     let search_query = format!("SELECT COUNT(rowid) FROM {TABLE} WHERE {column} {case} ?");
//     let result: (i64,) = sqlx::query_as(&search_query)
//         .bind(format!("%{}%", find))
//         .fetch_one(pool)
//         .await?;

//     Ok(result.0 as usize)
// }

// pub async fn smreplace_process(
//     pool: &SqlitePool,
//     find: &mut String,
//     replace: &mut String,
//     column: &mut String,
//     dirty: bool,
//     is_filepath: bool,
//     case_sensitive: bool,
// ) {
//     let dirty_text = if dirty && !is_filepath {
//         ", _Dirty = 1"
//     } else {
//         ""
//     };
//     let case_text = if case_sensitive { "GLOB" } else { "LIKE" };

//     let replace_query = format!(
//         "UPDATE {} SET {} = REPLACE({}, '{}', '{}'){} WHERE {} {} '%{}%'",
//         TABLE, column, column, find, replace, dirty_text, column, case_text, find
//     );
//     let _ = sqlx::query(&replace_query).execute(pool).await;

//     if is_filepath {
//         let mut column = "Filename";
//         let replace_query = format!(
//             "UPDATE {} SET {} = REPLACE({}, '{}', '{}'){} WHERE {} {} '%{}%'",
//             TABLE, column, column, find, replace, dirty_text, column, case_text, find
//         );
//         let _ = sqlx::query(&replace_query).execute(pool).await;

//         column = "Pathname";
//         let replace_query = format!(
//             "UPDATE {} SET {} = REPLACE({}, '{}', '{}'){} WHERE {} {} '%{}%'",
//             TABLE, column, column, find, replace, dirty_text, column, case_text, find
//         );
//         let _ = sqlx::query(&replace_query).execute(pool).await;

//         let table = "justinrdb_Pathname";
//         let replace_query = format!(
//             "UPDATE {} SET {} = REPLACE({}, '{}', '{}'){} WHERE {} {} '%{}%'",
//             table, column, column, find, replace, dirty_text, column, case_text, find
//         );
//         let _ = sqlx::query(&replace_query).execute(pool).await;
//     }
// }



pub async fn gather_duplicate_filenames_in_database(
    pool: SqlitePool,
    progress_sender: mpsc::Sender<ProgressMessage>,
    status_sender: mpsc::Sender<Arc<str>>,
    order: Vec<String>,
    match_groups: Vec<String>,
    match_null: bool,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let mut file_records = HashSet::new();
    let _ = status_sender.send("Gathering Duplicate Records".into()).await;
    println!("basic search begin");
    // Construct the ORDER BY clause dynamically
    let order_clause = order.join(", ");
    let partition_by_clause = match_groups.join(", ");
    let where_clause = if match_null || match_groups.is_empty() {
        String::new()
    } else {
        let non_null_conditions: Vec<String> = match_groups
            .iter()
            .map(|group| format!("{group} IS NOT NULL AND {group} !=''"))
            .collect();
        format!("WHERE {}", non_null_conditions.join(" AND "))
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
        partition_by_clause, order_clause, TABLE, where_clause
    );
    println!("fetching rows: {}", &sql);
    let rows = sqlx::query(&sql).fetch_all(&pool).await?;
    println!("received rows");
    let _ = status_sender.send("Organizing Records".into()).await;

    let total = rows.len();
    let mut counter = 0;

    for row in rows {
        
        file_records.insert(row_to_file_record(&row));
        counter += 1;

        if counter % 100 == 0 {
            let _ = progress_sender
                .send(ProgressMessage::Update(counter, total))
                .await;
        }
    }

    Ok(file_records)
}




pub async fn gather_deep_dive_records(
    pool: SqlitePool,
    progress_sender: mpsc::Sender<ProgressMessage>,
    status_sender: mpsc::Sender<Arc<str>>,
    ignore_extension: bool,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let mut file_groups: HashMap<String, Vec<FileRecord>> = HashMap::new();
    let _ = status_sender.send("Gathering Duplicates with Similar Filenames".into()).await;

    let query = &format!("SELECT rowid, filename, duration, filepath FROM {}", TABLE);

    let rows = sqlx::query(query).fetch_all(&pool).await?;
    let _ = status_sender.send("Organizing Results".into()).await;

    let total = rows.len();
    let mut counter: usize = 0;

    // Use a parallel iterator to process the rows
    let processed_records: Vec<(String, FileRecord)> = rows
        .par_iter() // Use a parallel iterator
        .map(|row| {
            let file_record = row_to_file_record(row);
            let base_filename = get_root_filename(&file_record.filename, ignore_extension)
                .unwrap_or_else(|| file_record.filename.to_string());
            (base_filename, file_record)
        })
        .collect(); // Collect results into a Vec<(String, FileRecord)>

    let _ = status_sender.send("Processing Records".into()).await;
    for (base_filename, file_record) in processed_records {
        file_groups
            .entry(base_filename)
            .or_default()
            .push(file_record);

        counter += 1;

        if counter % 100 == 0 {
            let _ = progress_sender
                .send(ProgressMessage::Update(counter, total))
                .await;
        }
    }

    let _ = status_sender.send("Finishing up".into()).await;

    let mut file_records = HashSet::new();
    for (root, records) in file_groups {
        if records.len() <= 1 {
            continue;
        }

        let root_found = records.iter().any(|record| {
            if ignore_extension {
                let name = Path::new(&*record.filename)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap();
                return name == root;
            }

            *record.filename == root
        });
        if root_found {
            file_records.extend(records.into_iter().filter(|record| {
                if ignore_extension {
                    let name = Path::new(&*record.filename)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap();
                    return name != root;
                }
                *record.filename != root
            }));
        } else {
            file_records.extend(records.into_iter().skip(1));
        }
    }
    let _ = status_sender
        .send(format!("Found {} duplicate records", file_records.len()).into())
        .await;

    Ok(file_records)
}

pub async fn gather_filenames_with_tags(
    pool: SqlitePool,
    progress_sender: mpsc::Sender<ProgressMessage>,
    status_sender: mpsc::Sender<Arc<str>>,
    tags: Vec<String>,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let _ = status_sender.send("Searching for Filenames with Specified Tags".into()).await;

    let total = tags.len();
    // let mut counter = 1;
    let mut file_records = HashSet::new();
    let max_concurrency = 10; // Adjust based on your system's capacity and connection pool size

    // Process each tag concurrently with a controlled level of concurrency
    let results = stream::iter(tags.into_iter())
        .enumerate()
        .map(|(counter, tag)| {
            
            let pool = pool.clone();
            let progress_sender = progress_sender.clone();
            let status_sender = status_sender.clone();
            async move {
                let query = format!(
                    "SELECT rowid, filename, duration, filepath FROM {} WHERE filename LIKE '%' || ? || '%'",
                    TABLE
                );

                
                let result = sqlx::query(&query).bind(&tag).fetch_all(&pool).await; // Return the result (Result<Vec<sqlx::sqlite::SqliteRow>, sqlx::Error>)
                let _ = status_sender.send((format!["Searching for tag: {}", &tag]).into()).await;
                let _ = progress_sender
                    .send(ProgressMessage::Update(counter, total))
                    .await;
                
                result
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
                    file_records.insert(row_to_file_record(&row));
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

    matching_records.retain(|record| filenames_to_check.contains(&*record.filename));

    if matching_records.is_empty() {
        println!("NO OVERLAPPING FILE RECORDS FOUND!");
    } else {
        println!("Found {} overlapping file records.", matching_records.len());
    }

    Ok(matching_records.into_iter().collect())
}

pub fn row_to_file_record(row: &SqliteRow) -> FileRecord {
    let id: u32 = row.get(0);
        let filename: &str = row.get(1);
        let duration = row.try_get(2).unwrap_or("");
        let path: &str = row.get(3);
        FileRecord {
            id: id as usize,
            filename: filename.into(),
            duration: duration.into(),
            path: path.into(),
        }
}


pub async fn fetch_filerecords_from_database(
    pool: &SqlitePool,
    query: &str,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let mut file_records = HashSet::new();

    let rows = sqlx::query(query).fetch_all(pool).await?;

    for row in rows {
        file_records.insert(row_to_file_record(&row));
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

fn extract_filenames_set_from_records(file_records: &HashSet<FileRecord>) -> HashSet<Arc<str>> {
    file_records
        .iter()
        .map(|record| record.filename.clone())
        .collect()
}



pub async fn delete_file_records(
    pool: &SqlitePool,
    records: &HashSet<FileRecord>,
    progress_sender: mpsc::Sender<ProgressMessage>,
    status_sender: mpsc::Sender<Arc<str>>,
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
            .send(format!("Processed {} / {}", progress, total).into())
            .await;
    }

    // After all deletions, perform the cleanup
    let _ = status_sender.send("Cleaning Up Database".into()).await;
    let _result = sqlx::query("VACUUM").execute(pool).await;

    println!("VACUUM done inside delete function");

    Ok(())
}

pub async fn create_duplicates_db(
    pool: &SqlitePool,
    dupe_records_to_keep: &HashSet<FileRecord>,
    progress_sender: mpsc::Sender<ProgressMessage>,
    status_sender: mpsc::Sender<Arc<str>>,
) -> Result<(), sqlx::Error> {
    println!("Generating Duplicates Only Database. This can take a while.");
    let _ = status_sender
        .send("Creating Duplicates Only Database. This can be slow.".into())
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
    let home_dir = home_dir();
    match home_dir {
        Some(home_dir) => {
            println!("Found SMDB dir");
            let db_dir = home_dir.join("Library/Application Support/SoundminerV6/Databases");
            if let Some(path) = FileDialog::new()
                .add_filter("SQLite Database", &["sqlite"])
                .set_directory(db_dir)
                .pick_file()
            {
                let db_path = path.display().to_string();
                if db_path.ends_with(".sqlite") {
                    println!("Opening Database {}", db_path);
                    let db = Database::open(&db_path).await;
                    return Some(db);
                }
            }
        }
        None => {
            println!("did not find SMDB dir");
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

// pub fn parse_to_sql(column: &str, operator: &OrderOperator, input: &str) -> String {
//     match operator {
//         OrderOperator::Largest => format! {"{} DESC", column.to_lowercase()},
//         OrderOperator::Smallest => format!("{} ASC", column.to_lowercase()),
//         OrderOperator::Is => format!(
//             "CASE WHEN {} IS '%{}%' THEN 0 ELSE 1 END ASC",
//             column.to_lowercase(),
//             input
//         ),
//         OrderOperator::IsNot => format!(
//             "CASE WHEN {} IS '%{}%' THEN 1 ELSE 0 END ASC",
//             column.to_lowercase(),
//             input
//         ),
//         OrderOperator::Contains => format!(
//             "CASE WHEN {} LIKE '%{}%' THEN 0 ELSE 1 END ASC",
//             column.to_lowercase(),
//             input
//         ),
//         OrderOperator::DoesNotContain => format!(
//             "CASE WHEN {} LIKE '%{}%' THEN 1 ELSE 0 END ASC",
//             column.to_lowercase(),
//             input
//         ),
//         OrderOperator::IsEmpty => format!(
//             "CASE WHEN {} IS NOT NULL AND {} != '' THEN 1 ELSE 0 END ASC",
//             column.to_lowercase(),
//             column.to_lowercase()
//         ),
//         OrderOperator::IsNotEmpty => format!(
//             "CASE WHEN {} IS NOT NULL AND {} != '' THEN 0 ELSE 1 END ASC",
//             column.to_lowercase(),
//             column.to_lowercase()
//         ),
//     }
// }

// pub fn parse_to_user_friendly(column: &str, operator: &OrderOperator, input: &str) -> String {
//     match operator {
//         OrderOperator::Largest => format! {"Largest {}", column},
//         OrderOperator::Smallest => format!("Smallest {} ", column),
//         OrderOperator::Is => format!("{} is '{}'", column, input),
//         OrderOperator::IsNot => format!("{} is NOT '{}'", column, input),
//         OrderOperator::Contains => format!("{} contains '{}'", column, input),
//         OrderOperator::DoesNotContain => format!("{} does NOT contain '{}'", column, input),
//         OrderOperator::IsEmpty => format!("{} is empty", column,),
//         OrderOperator::IsNotEmpty => format!("{} is NOT empty", column,),
//     }
// }

// pub fn parse_to_struct(column: String, operator: OrderOperator, input: String) -> PreservationLogic {
//     PreservationLogic{
//         // column,
//         // operator,
//         // variable: input,    
//         friendly: parse_to_user_friendly(&column, &operator, &input),
//         sql: parse_to_sql(&column, &operator, &input),
//     }
// }
