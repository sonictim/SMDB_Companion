pub use crate::prelude::*;

#[derive(Debug, Clone)]
pub enum Pool {
    Sqlite(SqlitePool),
    MySql(MySqlPool),
}

impl Default for Pool {
    fn default() -> Self {
        // Create a dummy SQLite pool - this is just a placeholder
        // In practice, you should handle this case appropriately
        panic!("Pool::default() should not be called - use Pool::connect() instead")
    }
}

impl Pool {
    pub async fn add_column(&self, add: &str) -> Result<(), sqlx::Error> {
        match self {
            Pool::Sqlite(pool) => {
                let query = format!(
                    "ALTER TABLE {} ADD COLUMN {} TEXT;",
                    self.get_table_name(),
                    add
                );
                sqlx::query(&query).execute(pool).await?;
                Ok(())
            }
            Pool::MySql(pool) => {
                let query = format!(
                    "ALTER TABLE {} ADD COLUMN {} TEXT;",
                    self.get_table_name(),
                    add
                );
                sqlx::query(&query).execute(pool).await?;
                Ok(())
            }
        }
    }

    pub async fn fetch_columns(&self) -> Result<Vec<Arc<str>>, sqlx::Error> {
        let query = format!("PRAGMA table_info({});", &self.get_table_name());
        let mut columns = match self {
            Pool::Sqlite(pool) => sqlx::query(query.as_str())
                .fetch_all(pool)
                .await?
                .into_iter()
                .filter_map(|row| {
                    let column_name: &str = row.try_get("name").ok()?; // Extract "name" column
                    if !column_name.starts_with('_') {
                        Some(column_name.into())
                    } else {
                        None
                    }
                })
                .collect::<Vec<Arc<str>>>(),
            Pool::MySql(pool) => sqlx::query(query.as_str())
                .fetch_all(pool)
                .await?
                .into_iter()
                .filter_map(|row| {
                    let column_name: &str = row.try_get("name").ok()?; // Extract "name" column
                    if !column_name.starts_with('_') {
                        Some(column_name.into())
                    } else {
                        None
                    }
                })
                .collect::<Vec<Arc<str>>>(),
        };

        columns.sort();

        Ok(columns)
    }

    fn get_type(&self) -> &'static str {
        match self {
            Pool::MySql(_) => "MySQL",
            Pool::Sqlite(_) => "SQLite",
        }
    }

    fn get_table_name(&self) -> &'static str {
        match self {
            Pool::MySql(_) => "metadata",
            Pool::Sqlite(_) => "justinmetadata",
        }
    }
    pub async fn connect(url: &str) -> Result<Self, sqlx::Error> {
        if url.starts_with("sqlite://") {
            println!("🔌 Attempting to connect to SQLite database: {}", url);
            let pool = SqlitePool::connect(url).await?;
            println!("✅ Success");
            Ok(Pool::Sqlite(pool))
        } else if url.starts_with("mysql://") {
            println!("🔌 Attempting to connect to SQLite database: {}", url);
            let pool = MySqlPool::connect(url).await?;
            println!("✅ Success");
            Ok(Pool::MySql(pool))
        } else {
            Err(sqlx::Error::Configuration(
                "Unsupported database URL".into(),
            ))
        }
    }

    pub async fn fetch_size(&self) -> Result<usize, sqlx::Error> {
        match self {
            Pool::Sqlite(pool) => {
                let count: (i64,) =
                    sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", self.get_table_name()))
                        .fetch_one(pool)
                        .await?;
                Ok(count.0 as usize)
            }
            Pool::MySql(pool) => {
                let count: (i64,) =
                    sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", self.get_table_name()))
                        .fetch_one(pool)
                        .await?;
                Ok(count.0 as usize)
            }
        }
    }

    pub fn is_sqlite(&self) -> bool {
        matches!(self, Pool::Sqlite(_))
    }

    pub fn is_mysql(&self) -> bool {
        matches!(self, Pool::MySql(_))
    }

    pub async fn execute(&self, query: &str) -> Result<(), sqlx::Error> {
        match self {
            Pool::Sqlite(pool) => {
                sqlx::query(query).execute(pool).await?;
                Ok(())
            }
            Pool::MySql(pool) => {
                sqlx::query(query).execute(pool).await?;
                Ok(())
            }
        }
    }
    pub async fn fetch_filerecords(
        &self,
        query: &str,
        enabled: &Enabled,
        pref: &Preferences,
        is_compare: bool,
        app: &AppHandle,
    ) -> Result<Vec<FileRecord>, sqlx::Error> {
        match self {
            Pool::Sqlite(pool) => {
                let completed = AtomicUsize::new(0);

                let rows = sqlx::query(query).fetch_all(pool).await?;
                let results = rows
                    .par_iter()
                    .filter_map(|row| {
                        let new_completed = completed.fetch_add(1, Ordering::SeqCst) + 1;
                        if new_completed % RECORD_DIVISOR == 0 {
                            app.substatus(
                                "Gathering File Records",
                                new_completed * 100 / rows.len(),
                                format!(
                                    "Processing Records into Memory: {}/{}",
                                    new_completed,
                                    rows.len()
                                )
                                .as_str(),
                            );
                        }
                        FileRecord::new_sqlite(row, enabled, pref, is_compare)
                    })
                    .collect::<Vec<_>>();
                app.substatus("Gathering File Records", 100, "Complete");
                Ok(results)
            }

            Pool::MySql(pool) => {
                let completed = AtomicUsize::new(0);

                let rows = sqlx::query(query).fetch_all(pool).await?;
                let results = rows
                    .par_iter()
                    .filter_map(|row| {
                        let new_completed = completed.fetch_add(1, Ordering::SeqCst) + 1;
                        if new_completed % RECORD_DIVISOR == 0 {
                            app.substatus(
                                "Gathering File Records",
                                new_completed * 100 / rows.len(),
                                format!(
                                    "Processing Records into Memory: {}/{}",
                                    new_completed,
                                    rows.len()
                                )
                                .as_str(),
                            );
                        }
                        FileRecord::new_mysql(row, enabled, pref, is_compare)
                    })
                    .collect::<Vec<_>>();
                app.substatus("Gathering File Records", 100, "Complete");
                Ok(results)
            }
        }
    }
    pub async fn update_channel_count_to_mono(
        &self,
        app: &AppHandle,
        record_ids: &[usize],
    ) -> Result<(), sqlx::Error> {
        const BATCH_SIZE: usize = 1000; // Smaller batch size for updates

        let mut counter = 0;

        // Begin a transaction and process based on pool type
        match self {
            Pool::Sqlite(pool) => {
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
                        "UPDATE {} SET Channels = 1, _Dirty = 1 WHERE recid IN ({})",
                        &self.get_table_name(),
                        placeholders
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
            }
            Pool::MySql(pool) => {
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
                        "UPDATE {} SET Channels = 1, _Dirty = 1 WHERE recid IN ({})",
                        &self.get_table_name(),
                        placeholders
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
            }
        }

        // Final status update
        app.status(
            "Stripping Multi-Mono",
            100,
            format!("Updated {} records to mono", record_ids.len()).as_str(),
        );

        Ok(())
    }
    pub async fn batch_update_column(
        &self,
        app: &AppHandle,
        pref: &Preferences,
        record_ids: &[usize],
        column: &str,
        value: &str,
    ) -> Result<(), sqlx::Error> {
        let mut counter = 0;

        // Begin a transaction and process based on pool type
        match self {
            Pool::Sqlite(pool) => {
                let mut tx = pool.begin().await?;

                // Process in batches
                for chunk in record_ids.chunks(pref.batch_size) {
                    // Create placeholders for SQL IN clause
                    let placeholders = std::iter::repeat("?")
                        .take(chunk.len())
                        .collect::<Vec<_>>()
                        .join(",");

                    // Build update query
                    let query = format!(
                        "UPDATE {} SET {} = {} WHERE recid IN ({})",
                        &self.get_table_name(),
                        column,
                        value,
                        placeholders
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
            }
            Pool::MySql(pool) => {
                let mut tx = pool.begin().await?;

                // Process in batches
                for chunk in record_ids.chunks(pref.batch_size) {
                    // Create placeholders for SQL IN clause
                    let placeholders = std::iter::repeat("?")
                        .take(chunk.len())
                        .collect::<Vec<_>>()
                        .join(",");

                    // Build update query
                    let query = format!(
                        "UPDATE {} SET {} = {} WHERE recid IN ({})",
                        &self.get_table_name(),
                        column,
                        value,
                        placeholders
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
            }
        }

        // Final status update
        app.status(
            "Stripping Multi-Mono",
            100,
            format!("Updated {} records to mono", record_ids.len()).as_str(),
        );

        Ok(())
    }
}

// #[derive(Default, Clone, Serialize, Deserialize, Clone)]

// pub enum DbPath {
//     Local(PathBuf),
// }

// impl DbPath {
//     pub fn get_path(&self) -> Option<Arc<str>> {
//         match self {
//             DbPath::Local(path) => {
//                 if let Some(path_str) = path.to_str() {
//                     println!("🛤️  Database path: {}", path_str);
//                     Some(Arc::from(path_str))
//                 } else {
//                     println!("❌ Failed to convert path to string");
//                     None
//                 }
//             }
//             DbPath::Server(server_db) => Some(server_db.get_connection_string()),
//         }
//     }

//     pub fn get_name(&self) -> Option<Arc<str>> {
//         match self {
//             DbPath::Local(path) => {
//                 if let Some(name) = path.file_stem() {
//                     if let Some(name_str) = name.to_str() {
//                         return Some(Arc::from(name_str));
//                     }
//                 }
//                 None
//             }
//             DbPath::Server(server_db) => Some(Arc::from(server_db.database.as_str())),
//         }
//     }
// }

#[derive(Default, Clone)]
pub struct Database {
    pub url: String,
    pub size: usize,
    pub records: Vec<FileRecord>,
    pub is_compare: bool,
    pub abort: Arc<AtomicBool>,
    pub pool: Pool,
}

// Change visibility of `Database` methods to private where possible
impl Database {
    pub async fn new(path: String, is_compare: bool) -> Result<Self, sqlx::Error> {
        println!("🆕 Creating new Database instance");
        println!("📁 Path: {}", path);
        println!("🔄 Is compare: {}", is_compare);

        let mut d = Database {
            pool: Pool::connect(&path).await?,
            url: path,
            size: 0,
            records: Vec::new(),
            is_compare,
            abort: Arc::new(AtomicBool::new(false)),
        };

        d.size = d.pool.fetch_size().await?;

        println!("💾 Initializing records vector with capacity: {}", d.size);
        d.records = Vec::with_capacity(d.size);

        println!("✅ Database instance created successfully");
        Ok(d)
    }

    // pub async fn open(&mut self, is_compare: bool) -> Option<Self> {
    //     let home_dir = home_dir();
    //     match home_dir {
    //         Some(home_dir) => {
    //             println!("Found SMDB dir");
    //             let db_dir = home_dir.join("Library/Application Support/SoundminerV6/Databases");
    //             let path = FileDialog::new()
    //                 .add_filter("SQLite Database", &["sqlite"])
    //                 .set_directory(db_dir)
    //                 .pick_file();
    //             self.init(path, is_compare).await;
    //         }
    //         None => {
    //             let path = FileDialog::new()
    //                 .add_filter("SQLite Database", &["sqlite"])
    //                 .pick_file();
    //             self.init(path, is_compare).await;
    //         }
    //     }
    //     None
    // }

    pub async fn init(&mut self, path: String, is_compare: bool) {
        self.pool = Pool::connect(&path).await.unwrap_or_else(|_| {
            println!("❌ Failed to connect to database at {}", path);
            Pool::default() // Return a default pool if connection fails
        });
        self.url = path;
        self.size = self.pool.fetch_size().await.unwrap_or(0);
        self.records = Vec::with_capacity(self.size); // No need for .into()
        self.is_compare = is_compare;
    }

    pub async fn create_clone(&self, tag: &str) -> Result<Database, std::io::Error> {
        let source_path = PathBuf::from(self.get_path());
        // Create the new path with proper cross-platform handling
        let mut path_string = self.get_path().to_string();
        path_string = path_string.replace(".sqlite", &format!("_{}.sqlite", tag));
        let target_string = format!("sqlite://{}", path_string);
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
        fs::copy(&source_path, &target_path).map_err(|e| {
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
        db.init(target_string, false).await;
        Ok(db)
    }

    pub fn get_path(&self) -> &str {
        let (_, path) = self.url.split_once("://").unwrap_or(("", &self.url));
        path
    }

    pub fn get_name(&self) -> String {
        let binding = self.get_path();
        let path = Path::new(&binding);
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn get_records_size(&self) -> usize {
        self.records
            .iter()
            .filter(|record| !record.algorithm.contains(&A::Keep))
            .count()
    }

    // pub async fn get_pool(&self) -> Option<SqlitePool> {
    //     if let Some(pool) = &self.pool {
    //         match pool {
    //             Pool::Sqlite(pool) => Some(pool.clone()),
    //             _ => None,
    //         }
    //     } else {
    //         println!("❌ No database pool available for connection");
    //         None
    //     }
    // }

    pub async fn add_column(&self, add: &str) -> Result<(), sqlx::Error> {
        self.pool.add_column(add).await?;
        println!("Added column: {}", add);
        Ok(())
    }

    pub async fn remove_column(&self, remove: &str) -> Result<(), sqlx::Error> {
        let query = format!(
            "ALTER&self.get_table() {} DROP COLUMN {};",
            &self.pool.get_table_name(),
            remove
        );
        self.pool.execute(&query).await?;
        println!("Removed column: {}", remove);

        Ok(())
    }

    pub async fn remove(&self, ids: &[usize], app: &AppHandle) -> Result<(), sqlx::Error> {
        const BATCH_SIZE: usize = 12321; // Define the batch size
        let _ = app;
        let mut counter = 0;

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
            let query = format!(
                "DELETE FROM {} WHERE recid IN ({})",
                &self.pool.get_table_name(),
                placeholders
            );

            match self.pool {
                Pool::Sqlite(ref pool) => {
                    println!("Executing query on SQLite pool");
                    // Create a query builder for SQLite
                    let mut query_builder = sqlx::query(&query);
                    // Bind each ID individually
                    for &id in chunk {
                        query_builder = query_builder.bind(id as i64);
                    }
                    query_builder.execute(pool).await?;
                }
                Pool::MySql(ref pool) => {
                    println!("Executing query on MySQL pool");
                    // Create a query builder for MySQL
                    let mut query_builder = sqlx::query(&query);
                    // Bind each ID individually
                    for &id in chunk {
                        query_builder = query_builder.bind(id as i64);
                    }
                    query_builder.execute(pool).await?;
                }
            }
        }
        app.status("Final Checks", 100, "Records successfully removed");
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
                .pool
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

    pub async fn fetch_all_filerecords(
        &self,
        enabled: &Enabled,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<Vec<FileRecord>, sqlx::Error> {
        let query = format!("SELECT * FROM {}", self.pool.get_table_name());
        self.pool
            .fetch_filerecords(&query, enabled, pref, self.is_compare, app)
            .await
    }

    pub async fn batch_update_column(
        &self,
        app: &AppHandle,
        pref: &Preferences,
        record_ids: &[usize],
        column: &str,
        value: &str,
    ) -> Result<(), sqlx::Error> {
        self.pool
            .batch_update_column(app, pref, record_ids, column, value)
            .await
    }
}

pub async fn batch_store_data_optimized(
    pool: &SqlitePool,
    data: &[(usize, &str)],
    column: &str,
    app: &AppHandle,
    table: &str,
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
                    "UPDATE {} SET {} = ? WHERE recid = ?",
                    table, column,
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
