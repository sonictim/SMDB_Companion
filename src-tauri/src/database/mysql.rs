use crate::prelude::*;

pub async fn fetch_filerecords_mysql(
    pool: &MySqlPool,
    is_compare: bool,
    query: &str,
    enabled: &Enabled,
    pref: &Preferences,
    app: &AppHandle,
) -> Result<Vec<FileRecord>, sqlx::Error> {
    let completed = AtomicUsize::new(0);
    let rows = match sqlx::query(query).fetch_all(pool).await {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error fetching records: {}", e);
            return Err(e);
        }
    };

    println!("{} Rows Found", rows.len());
    let records: Vec<FileRecord> = rows
        .par_iter()
        .enumerate()
        .filter_map(|(count, row)| {
            let new_completed = completed.fetch_add(1, Ordering::SeqCst) + 1;
            if new_completed % RECORD_DIVISOR == 0 {
                app.substatus(
                    "Gathering File Records",
                    new_completed * 100 / rows.len(),
                    format!("Processing Records into Memory: {}/{}", count, rows.len()).as_str(),
                );
            }
            FileRecord::new_mysql(row, enabled, pref, is_compare)
        })
        .collect();
    app.substatus("Gathering File Records", 100, "Complete");

    Ok(records)
}

pub async fn fetch_size_mysql(pool: &MySqlPool) -> Result<usize, sqlx::Error> {
    println!("📏 Attempting to fetch database size...");

    println!("🔍 Executing count query on table: {}", MYSQL_TABLE);

    match sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {}", MYSQL_TABLE))
        .fetch_one(pool)
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
}

pub async fn remove_mysql(
    pool: &MySqlPool,
    ids: &[usize],
    app: &AppHandle,
) -> Result<(), sqlx::Error> {
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
            MYSQL_TABLE, placeholders
        );

        // Create a query builder
        let mut query_builder = sqlx::query(&query);

        // Bind each ID individually
        for &id in chunk {
            query_builder = query_builder.bind(id as i64);
        }

        // Execute the query
        query_builder.execute(pool).await?;
    }
    app.status("Final Checks", 100, "Records successfully removed");
    Ok(())
}

pub async fn batch_update_column_mysql(
    pool: &MySqlPool,
    app: &AppHandle,
    pref: &Preferences,
    record_ids: &[usize],
    column: &str,
    value: &str,
) -> Result<(), sqlx::Error> {
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
            "UPDATE {} SET {} = {} WHERE recid IN ({})",
            MYSQL_TABLE, column, value, placeholders
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

    Ok(())
}

pub async fn update_channel_count_to_mono_mysql(
    pool: &MySqlPool,
    app: &AppHandle,
    record_ids: &[usize],
) -> Result<(), sqlx::Error> {
    const BATCH_SIZE: usize = 1000; // Smaller batch size for updates

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
            "UPDATE {} SET Channels = 1, _Dirty = 1 WHERE recid IN ({})",
            MYSQL_TABLE, placeholders
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

    Ok(())
}

pub async fn add_column_mysql(pool: &MySqlPool, add: &str) -> Result<(), sqlx::Error> {
    println!(
        "🔍 Checking if column '{}' exists in table '{}'",
        add, MYSQL_TABLE
    );

    let columns = match sqlx::query(&format!("SHOW COLUMNS FROM {}", MYSQL_TABLE))
        .fetch_all(pool)
        .await
    {
        Ok(cols) => {
            println!("✅ Successfully fetched {} columns", cols.len());
            cols
        }
        Err(e) => {
            println!("❌ Failed to fetch columns: {}", e);
            return Err(e);
        }
    };

    // Debug: Print all available columns
    println!("📋 Available columns:");
    for (i, row) in columns.iter().enumerate() {
        match row.try_get::<String, _>("Field") {
            Ok(field_name) => println!("  {}: {}", i, field_name),
            Err(e) => println!("  {}: Error getting field name: {}", i, e),
        }
    }

    // Check if our column exists (MySQL uses "Field" not "name")
    let column_exists = columns
        .iter()
        .any(|row| match row.try_get::<String, _>("Field") {
            Ok(column_name) => {
                println!("🔍 Comparing '{}' with '{}'", column_name, add);
                column_name == add
            }
            Err(e) => {
                println!("⚠️ Error getting column name: {}", e);
                false
            }
        });

    println!("🎯 Column '{}' exists: {}", add, column_exists);

    // Only add the column if it doesn't exist
    if !column_exists {
        println!("➕ Adding new column: {}", add);
        let query = format!("ALTER TABLE {} ADD COLUMN {} TEXT", MYSQL_TABLE, add);
        println!("🔧 Executing query: {}", query);

        match sqlx::query(&query).execute(pool).await {
            Ok(_) => println!("✅ Successfully added column: {}", add),
            Err(e) => {
                println!("❌ Failed to add column: {}", e);
                return Err(e);
            }
        }
    } else {
        println!("ℹ️ Column '{}' already exists", add);
    }

    Ok(())
}

pub async fn remove_column_mysql(pool: &MySqlPool, remove: &str) -> Result<(), sqlx::Error> {
    let columns = sqlx::query(&format!("SHOW COLUMNS FROM {}", MYSQL_TABLE))
        .fetch_all(pool)
        .await?;

    // Check if our column exists (MySQL uses "Field" not "name")
    let column_exists = columns.iter().any(|row| {
        let column_name: String = row.try_get("Field").unwrap_or_default();
        column_name == remove
    });

    // Only remove the column if it exists
    if column_exists {
        // Remove the column
        let query = format!("ALTER TABLE {} DROP COLUMN {}", MYSQL_TABLE, remove);
        sqlx::query(&query).execute(pool).await?;
        println!("➖ Removed column: {}", remove);
    } else {
        println!("ℹ️ Column '{}' does not exist", remove);
    }

    Ok(())
}

pub async fn fetch_columns_mysql(pool: &MySqlPool) -> Result<Vec<Arc<str>>, sqlx::Error> {
    let query = format!("SHOW COLUMNS FROM {}", MYSQL_TABLE);

    let mut columns = sqlx::query(&query)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let column_name: String = row.try_get("Field").ok()?; // Use "Field" for MySQL
            if !column_name.starts_with('_') {
                Some(column_name.into())
            } else {
                None
            }
        })
        .collect::<Vec<Arc<str>>>();
    columns.sort();

    Ok(columns)
}
