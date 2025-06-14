pub mod filerecord;
pub use filerecord::*;
pub mod mysql;
pub mod sqlite;

#[derive(Clone)]

pub enum Pool {
    Sqlite(SqlitePool),
    Mysql(MySqlPool),
}

impl Pool {
    pub async fn connect(url: &str) -> R<Self, sqlx::Error> {
        if url.starts_with("sqlite://") {
            let pool = SqlitePool::connect(url).await?;
            Ok(Pool::Sqlite(pool))
        } else if url.starts_with("mysql://") {
            let pool = MySqlPool::connect(url).await?;
            Ok(Pool::Mysql(pool))
        } else {
            Err(sqlx::Error::Configuration(
                "Unsupported database URL".into(),
            ))
        }
    }

    pub async fn fetch_filerecords(
        &mut self,
        is_compare: bool,
        query: &str,
        enabled: &Enabled,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<Vec<FileRecord>, sqlx::Error> {
        match self {
            Pool::Mysql(pool) => {
                mysql::fetch_filerecords_mysql(pool, is_compare, query, enabled, pref, app).await
            }
            Pool::Sqlite(pool) => {
                sqlite::fetch_filerecords_sqlite(pool, is_compare, query, enabled, pref, app).await
            }
        }
    }

    pub async fn fetch_columns(&self) -> Result<Vec<Arc<str>>, sqlx::Error> {
        match self {
            Pool::Mysql(pool) => mysql::fetch_columns_mysql(pool).await,
            Pool::Sqlite(pool) => sqlite::fetch_columns_sqlite(pool).await,
        }
    }

    pub async fn fetch_size(&self) -> Result<usize, sqlx::Error> {
        match self {
            Pool::Mysql(pool) => mysql::fetch_size_mysql(pool).await,
            Pool::Sqlite(pool) => sqlite::fetch_size_sqlite(pool).await,
        }
    }

    pub async fn remove(&self, ids: &[usize], app: &AppHandle) -> Result<(), sqlx::Error> {
        match self {
            Pool::Mysql(pool) => mysql::remove_mysql(pool, ids, app).await,
            Pool::Sqlite(pool) => sqlite::remove_sqlite(pool, ids, app).await,
        }
    }

    pub async fn add_column(&self, add: &str) -> Result<(), sqlx::Error> {
        match self {
            Pool::Mysql(pool) => mysql::add_column_mysql(pool, add).await,
            Pool::Sqlite(pool) => sqlite::add_column_sqlite(pool, add).await,
        }
    }

    pub async fn remove_column(&self, remove: &str) -> Result<(), sqlx::Error> {
        match self {
            Pool::Mysql(pool) => mysql::remove_column_mysql(pool, remove).await,
            Pool::Sqlite(pool) => sqlite::remove_column_sqlite(pool, remove).await,
        }
    }

    pub async fn batch_update_column(
        &self,
        app: &AppHandle,
        pref: &Preferences,
        record_ids: &[usize],
        column: &str,
        value: &str,
    ) -> Result<(), sqlx::Error> {
        match self {
            Pool::Mysql(pool) => {
                mysql::batch_update_column_mysql(pool, app, pref, record_ids, column, value).await
            }
            Pool::Sqlite(pool) => {
                sqlite::batch_update_column_sqlite(pool, app, pref, record_ids, column, value).await
            }
        }
    }
    pub async fn update_channel_count_to_mono(
        &self,
        app: &AppHandle,
        record_ids: &[usize],
    ) -> Result<(), sqlx::Error> {
        match self {
            Pool::Mysql(pool) => {
                mysql::update_channel_count_to_mono_mysql(pool, app, record_ids).await
            }
            Pool::Sqlite(pool) => {
                sqlite::update_channel_count_to_mono_sqlite(pool, app, record_ids).await
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct Database {
    pub url: String,
    pub pool: Option<Pool>,
    pub size: usize,
    pub records: Vec<FileRecord>, // Changed from Arc<[FileRecord]> to Vec<FileRecord>
    pub is_compare: bool,
    pub abort: Arc<AtomicBool>,
}

// Change visibility of `Database` methods to private where possible
impl Database {
    pub async fn get_pool_sqlite(&self) -> R<SqlitePool, sqlx::Error> {
        if let Some(Pool::Sqlite(pool)) = &self.pool {
            Ok(pool.clone())
        } else {
            Err(sqlx::Error::Configuration(
                "Database is not a SQLite pool".into(),
            ))
        }
    }

    pub async fn new(url: String, is_compare: bool) -> R<Self, sqlx::Error> {
        println!("🆕 Creating new Database instance");
        println!("📁 Url: {}", url);
        println!("🔄 Is compare: {}", is_compare);

        let pool = Pool::connect(&url).await?;
        println!("🔌 Database connection established");

        let mut d = Database {
            url,
            pool: Some(pool),
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

    pub async fn init(&mut self, url: String, is_compare: bool) -> R<(), sqlx::Error> {
        let pool = Pool::connect(&url).await?;
        println!("🔌 Database connection established");
        self.pool = Some(pool);
        self.url = url;
        self.size = self.fetch_size().await.unwrap_or(0);
        self.records = Vec::with_capacity(self.size); // No need for .into()
        self.is_compare = is_compare;
        Ok(())
    }

    pub async fn create_clone(&self, tag: &str) -> Result<Database, std::io::Error> {
        let source_path = PathBuf::from(self.get_address().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid database URL: cannot extract file path",
            )
        })?);

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
        let target_path = format!("sqlite://{}", target_path.display());
        db.init(target_path, false).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to initialize database: {}", e),
            )
        })?;
        Ok(db)
    }

    // pub fn get_path(&self) -> Option<Arc<str>> {
    //     if let Some(path) = &self.path {
    //         if let Some(path_str) = path.to_str() {
    //             println!("🛤️  Database path: {}", path_str);
    //             return Some(Arc::from(path_str));
    //         } else {
    //             println!("❌ Failed to convert path to string");
    //         }
    //     } else {
    //         println!("❌ No database path set");
    //     }
    //     None
    // }

    pub fn get_address(&self) -> Option<&str> {
        self.url.split("://").nth(1)
    }

    pub fn get_name(&self) -> Option<&str> {
        let a = self.get_address()?;
        let a = a.split('/').last()?;
        if let Some(name) = a.strip_suffix(".sqlite") {
            Some(name)
        } else {
            Some(a)
        }
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
    //     println!("🔌 Attempting to connect to database: {}", self.url);

    //     // Check if file exists first
    //     let path_buf = std::path::PathBuf::from(self.get_address()?);
    //     if !path_buf.exists() {
    //         println!("❌ Database file does not exist: {}", self.url);
    //         return None;
    //     }

    //     // Check file size
    //     match std::fs::metadata(&path_buf) {
    //         Ok(metadata) => {
    //             let file_size = metadata.len();
    //             println!("📊 Database file size: {} bytes", file_size);
    //             if file_size == 0 {
    //                 println!("⚠️  Database file is empty (0 bytes)");
    //             }
    //         }
    //         Err(e) => {
    //             println!("❌ Failed to get database metadata: {}", e);
    //             return None;
    //         }
    //     }

    //     // Attempt connection
    //     match SqlitePool::connect(&self.url).await {
    //         Ok(pool) => {
    //             println!("✅ Successfully connected to database");
    //             Some(pool)
    //         }
    //         Err(e) => {
    //             println!("❌ Failed to connect to database: {}", e);
    //             None
    //         }
    //     }
    // }

    pub async fn add_column(&self, add: &str) -> Result<(), sqlx::Error> {
        if let Some(pool) = self.pool.as_ref() {
            println!("➕ Adding column to database: {}", add);
            pool.add_column(add).await
        } else {
            Err(sqlx::Error::Configuration(
                "No database connection available".into(),
            ))
        }
    }

    pub async fn fetch_size(&self) -> Result<usize, sqlx::Error> {
        if let Some(pool) = self.pool.as_ref() {
            println!("📏 Fetching database size");
            pool.fetch_size().await
        } else {
            Err(sqlx::Error::Configuration(
                "No database connection available".into(),
            ))
        }
    }

    pub async fn remove(&self, ids: &[usize], app: &AppHandle) -> Result<(), sqlx::Error> {
        if let Some(pool) = self.pool.as_ref() {
            println!("🗑️  Removing records from database");
            pool.remove(ids, app).await
        } else {
            Err(sqlx::Error::Configuration(
                "No database connection available".into(),
            ))
        }
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

    // pub async fn fetch(&self, query: &str) -> Vec<SqliteRow> {
    //     if let Some(pool) = self.get_pool().await {
    //         sqlx::query(query)
    //             .fetch_all(&pool)
    //             .await
    //             .unwrap_or_default()
    //     } else {
    //         Vec::new()
    //     }
    // }

    pub async fn fetch_filerecords(
        &mut self,
        query: &str,
        enabled: &Enabled,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), sqlx::Error> {
        if let Some(pool) = self.pool.as_mut() {
            println!("🔍 Fetching file records with query: {}", query);
            self.records = pool
                .fetch_filerecords(self.is_compare, query, enabled, pref, app)
                .await?;
            println!("✅ Fetched {} records from database", self.records.len());
        } else {
            return Err(sqlx::Error::Configuration(
                "No database connection available".into(),
            ));
        }
        Ok(())
    }

    pub async fn fetch_all_filerecords(
        &mut self,
        enabled: &Enabled,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), sqlx::Error> {
        println!("Gathering all records from database");

        let mut table = SQLITE_TABLE;
        if let Some(Pool::Mysql(_)) = self.pool.as_ref() {
            table = MYSQL_TABLE;
        }

        self.fetch_filerecords(
            &format!(
                "SELECT recid, filepath, duration, _fingerprint, description, channels, bitdepth, samplerate, _DualMono, {} FROM {}",
                pref.get_data_requirements(),
                table
            ),
            enabled,
            pref,
            app,
        )
        .await
    }

    pub async fn fetch_columns(&self) -> Result<Vec<Arc<str>>, sqlx::Error> {
        if let Some(pool) = self.pool.as_ref() {
            println!("🔍 Fetching columns from database");
            pool.fetch_columns().await
        } else {
            Err(sqlx::Error::Configuration(
                "No database connection available".into(),
            ))
        }
    }
    pub async fn update_channel_count_to_mono(
        &self,
        app: &AppHandle,
        record_ids: &[usize],
    ) -> Result<(), sqlx::Error> {
        if let Some(pool) = self.pool.as_ref() {
            pool.update_channel_count_to_mono(app, record_ids).await
        } else {
            Err(sqlx::Error::Configuration(
                "No database connection available".into(),
            ))
        }
    }
    pub async fn batch_update_column(
        &self,
        app: &AppHandle,
        pref: &Preferences,
        record_ids: &[usize],
        column: &str,
        value: &str,
    ) -> Result<(), sqlx::Error> {
        if let Some(pool) = self.pool.as_ref() {
            pool.batch_update_column(app, pref, record_ids, column, value)
                .await
        } else {
            Err(sqlx::Error::Configuration(
                "No database connection available".into(),
            ))
        }
    }

    pub async fn remove_column(&self, app: &AppHandle, column: &str) -> Result<(), sqlx::Error> {
        if let Some(pool) = self.pool.as_ref() {
            app.status(
                "Removing Column",
                0,
                &format!("Removing column: {}", column),
            );
            pool.remove_column(column).await?;
            app.status("Removing Column", 100, "Column removed successfully");
            Ok(())
        } else {
            Err(sqlx::Error::Configuration(
                "No database connection available".into(),
            ))
        }
    }
}
