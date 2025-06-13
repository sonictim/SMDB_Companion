pub mod filerecord;
pub use filerecord::*;

#[derive(Default, Clone)]
pub struct Database {
    pub path: Option<PathBuf>,
    pub size: usize,
    pub records: Vec<FileRecord>, // Changed from Arc<[FileRecord]> to Vec<FileRecord>
    pub is_compare: bool,
    pub abort: Arc<AtomicBool>,
}

// Change visibility of `Database` methods to private where possible
impl Database {
    pub async fn new(path: &str, is_compare: bool) -> Self {
        println!("🆕 Creating new Database instance");
        println!("📁 Path: {}", path);
        println!("🔄 Is compare: {}", is_compare);

        let mut d = Database {
            path: Some(PathBuf::from(path)),
            size: 0,
            records: Vec::new(),
            is_compare,
            abort: Arc::new(AtomicBool::new(false)),
        };

        println!("📏 Fetching initial database size...");
        d.size = d.fetch_size().await.unwrap_or_else(|e| {
            println!("⚠️  Failed to fetch size, using 0: {}", e);
            0
        });

        println!("💾 Initializing records vector with capacity: {}", d.size);
        d.records = Vec::with_capacity(d.size);

        println!("✅ Database instance created successfully");
        d
    }

    pub async fn open(&mut self, is_compare: bool) -> Option<Self> {
        let home_dir = home_dir();
        match home_dir {
            Some(home_dir) => {
                println!("Found SMDB dir");
                let db_dir = home_dir.join("Library/Application Support/SoundminerV6/Databases");
                let path = FileDialog::new()
                    .add_filter("SQLite Database", &["sqlite"])
                    .set_directory(db_dir)
                    .pick_file();
                self.init(path, is_compare).await;
            }
            None => {
                let path = FileDialog::new()
                    .add_filter("SQLite Database", &["sqlite"])
                    .pick_file();
                self.init(path, is_compare).await;
            }
        }
        None
    }

    pub async fn init(&mut self, path: Option<PathBuf>, is_compare: bool) {
        if let Some(path) = path {
            self.path = Some(path);
            self.size = self.fetch_size().await.unwrap_or(0);
            self.records = Vec::with_capacity(self.size); // No need for .into()
            self.is_compare = is_compare;
        }
    }

    pub async fn create_clone(&self, tag: &str) -> Result<Database, std::io::Error> {
        let source_path = self.path.as_ref().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "No source database path")
        })?;

        // Create the new path with proper cross-platform handling
        let mut path_string = source_path.display().to_string();
        path_string = path_string.replace(".sqlite", &format!("_{}.sqlite", tag));
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
        fs::copy(source_path, &target_path).map_err(|e| {
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
        db.init(Some(target_path), false).await;
        Ok(db)
    }

    pub fn get_path(&self) -> Option<Arc<str>> {
        if let Some(path) = &self.path {
            if let Some(path_str) = path.to_str() {
                println!("🛤️  Database path: {}", path_str);
                return Some(Arc::from(path_str));
            } else {
                println!("❌ Failed to convert path to string");
            }
        } else {
            println!("❌ No database path set");
        }
        None
    }

    pub fn get_name(&self) -> Option<Arc<str>> {
        if let Some(path) = &self.path {
            if let Some(name) = path.file_stem() {
                if let Some(name_str) = name.to_str() {
                    return Some(Arc::from(name_str));
                }
            }
        }
        None
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

    pub async fn get_pool(&self) -> Option<SqlitePool> {
        if let Some(path) = self.get_path() {
            println!("🔌 Attempting to connect to database: {}", path);

            // Check if file exists first
            let path_buf = std::path::PathBuf::from(path.as_ref());
            if !path_buf.exists() {
                println!("❌ Database file does not exist: {}", path);
                return None;
            }

            // Check file size
            match std::fs::metadata(&path_buf) {
                Ok(metadata) => {
                    let file_size = metadata.len();
                    println!("📊 Database file size: {} bytes", file_size);
                    if file_size == 0 {
                        println!("⚠️  Database file is empty (0 bytes)");
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get database metadata: {}", e);
                    return None;
                }
            }

            // Attempt connection
            match SqlitePool::connect(&path).await {
                Ok(pool) => {
                    println!("✅ Successfully connected to database");
                    Some(pool)
                }
                Err(e) => {
                    println!("❌ Failed to connect to database: {}", e);
                    None
                }
            }
        } else {
            println!("❌ No database path available for connection");
            None
        }
    }

    pub async fn remove_column(&self, remove: &str) -> Result<(), sqlx::Error> {
        if let Some(pool) = self.get_pool().await {
            // First check if the column already exists
            let columns = sqlx::query(&format!("PRAGMA table_info({});", SQLITE_TABLE))
                .fetch_all(&pool)
                .await?;

            // Check if our column exists
            let column_exists = columns.iter().any(|row| {
                let column_name: &str = row.try_get("name").unwrap_or_default();
                column_name == remove
            });

            // Only remove the column if it exists
            if column_exists {
                // Remove the column
                let query = format!("ALTER TABLE {} DROP COLUMN {};", SQLITE_TABLE, remove);
                sqlx::query(&query).execute(&pool).await?;
                println!("Removed column: {}", remove);
            } else {
                println!("Column '{}' does not exist", remove);
            }

            return Ok(());
        }

        Err(sqlx::Error::Configuration(
            "No database connection available".into(),
        ))
    }

    pub async fn add_column(&self, add: &str) -> Result<(), sqlx::Error> {
        if let Some(pool) = self.get_pool().await {
            // First check if the column already exists
            let columns = sqlx::query(&format!("PRAGMA table_info({});", SQLITE_TABLE))
                .fetch_all(&pool)
                .await?;

            // Check if our column exists
            let column_exists = columns.iter().any(|row| {
                let column_name: &str = row.try_get("name").unwrap_or_default();
                column_name == add
            });

            // Only add the column if it doesn't exist
            if !column_exists {
                // Add the column with TEXT type (you can change this if needed)
                let query = format!("ALTER TABLE {} ADD COLUMN {} TEXT;", SQLITE_TABLE, add);
                sqlx::query(&query).execute(&pool).await?;
                println!("Added new column: {}", add);
            } else {
                println!("Column '{}' already exists", add);
            }

            return Ok(());
        }

        Err(sqlx::Error::Configuration(
            "No database connection available".into(),
        ))
    }

    pub async fn fetch_size(&self) -> Result<usize, sqlx::Error> {
        println!("📏 Attempting to fetch database size...");

        if let Some(pool) = self.get_pool().await {
            println!("🔍 Executing count query on table: {}", SQLITE_TABLE);

            match sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {}", SQLITE_TABLE))
                .fetch_one(&pool)
                .await
            {
                Ok(count) => {
                    let size = count.0 as usize;
                    println!("✅ Database size fetched successfully: {} records", size);
                    Ok(size)
                }
                Err(e) => {
                    println!("❌ Failed to fetch database size: {}", e);
                    Err(e)
                }
            }
        } else {
            println!("❌ No database pool available for size fetch");
            Ok(0)
        }
    }

    pub async fn remove(&self, ids: &[usize], app: &AppHandle) -> Result<(), sqlx::Error> {
        const BATCH_SIZE: usize = 12321; // Define the batch size
        let _ = app;
        let mut counter = 0;
        if let Some(pool) = self.get_pool().await {
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
                    "DELETE FROM {} WHERE rowid IN ({})",
                    SQLITE_TABLE, placeholders
                );

                // Create a query builder
                let mut query_builder = sqlx::query(&query);

                // Bind each ID individually
                for &id in chunk {
                    query_builder = query_builder.bind(id as i64);
                }

                // Execute the query
                query_builder.execute(&pool).await?;
            }
            app.status("Final Checks", 100, "Records successfully removed");
        }
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

    pub async fn fetch(&self, query: &str) -> Vec<SqliteRow> {
        if let Some(pool) = self.get_pool().await {
            sqlx::query(query)
                .fetch_all(&pool)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    pub async fn fetch_filerecords(
        &mut self,
        query: &str,
        enabled: &Enabled,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), sqlx::Error> {
        // self.records.clear();
        let completed = AtomicUsize::new(0);
        let rows = self.fetch(query).await;
        let mut records = Vec::with_capacity(rows.len());
        println!("{} Rows Found", rows.len());
        let new_records: Vec<FileRecord> = rows
            .par_iter()
            .enumerate()
            .filter_map(|(count, row)| {
                let new_completed = completed.fetch_add(1, Ordering::SeqCst) + 1;
                if new_completed % RECORD_DIVISOR == 0 {
                    app.substatus(
                        "Gathering File Records",
                        new_completed * 100 / rows.len(),
                        format!("Processing Records into Memory: {}/{}", count, rows.len())
                            .as_str(),
                    );
                }
                FileRecord::new_sqlite(row, enabled, pref, self.is_compare)
            })
            .collect();
        app.substatus("Gathering File Records", 100, "Complete");

        records.extend(new_records);
        self.records = records;
        Ok(())
    }

    pub async fn fetch_all_filerecords(
        &mut self,
        enabled: &Enabled,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), sqlx::Error> {
        println!("Gathering all records from database");
        self.fetch_filerecords(
            &format!(
                "SELECT rowid, filepath, duration, _fingerprint, description, channels, bitdepth, samplerate, _DualMono, {} FROM {}",
                pref.get_data_requirements(),
                SQLITE_TABLE
            ),
            enabled,
            pref,
            app,
        )
        .await
    }

    pub async fn fetch_columns(&self) -> Result<Vec<Arc<str>>, sqlx::Error> {
        // Query for table info using PRAGMA
        let mut columns = self
            .fetch(&format!("PRAGMA table_info({});", SQLITE_TABLE))
            .await
            .into_iter()
            .filter_map(|row| {
                let column_name: &str = row.try_get("name").ok()?; // Extract "name" column
                if !column_name.starts_with('_') {
                    Some(column_name.into())
                } else {
                    None
                }
            })
            .collect::<Vec<Arc<str>>>();
        columns.sort();
        // if let Some(index) = columns.iter().position(|x| x.as_ref() == "FilePath") {
        //     let filepath = columns.remove(index); // Remove the item
        //     columns.insert(0, filepath); // Insert it at the beginning
        // }

        Ok(columns)
    }
    pub async fn update_channel_count_to_mono(
        &self,
        app: &AppHandle,
        record_ids: &[usize],
    ) -> Result<(), sqlx::Error> {
        const BATCH_SIZE: usize = 1000; // Smaller batch size for updates

        if let Some(pool) = self.get_pool().await {
            let mut counter = 0;

            // Begin a transaction for better performance
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
                    "UPDATE {} SET Channels = 1, _Dirty = 1 WHERE rowid IN ({})",
                    SQLITE_TABLE, placeholders
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

            // Final status update
            app.status(
                "Stripping Multi-Mono",
                100,
                format!("Updated {} records to mono", record_ids.len()).as_str(),
            );
        }

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
        if let Some(pool) = self.get_pool().await {
            let mut counter = 0;

            // Begin a transaction for better performance
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
                    "UPDATE {} SET {} = {} WHERE rowid IN ({})",
                    SQLITE_TABLE, column, value, placeholders
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

            // Final status update
            app.status(
                "Stripping Multi-Mono",
                100,
                format!("Updated {} records to mono", record_ids.len()).as_str(),
            );
        }

        Ok(())
    }
}
