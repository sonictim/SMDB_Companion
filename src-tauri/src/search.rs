// use rayon::slice::ChunkBy;

use ffcodex_lib::dprintln;

pub use crate::prelude::*;

impl Database {
    pub async fn compare_search(&mut self, enabled: &Enabled, pref: &Preferences, app: &AppHandle) {
        let mut cdb = Database::default();
        let _ = cdb.init(enabled.compare_db.clone(), true).await;
        app.substatus("Compare Databases", 0, "Loading Compare Database");

        let _ = cdb.fetch_all_filerecords(enabled, pref, app).await;
        let mut total = cdb.get_size();
        if total == 0 {
            total = 100;
        }
        println!("{} Records Found in Compare Database", total);
        // Use HashSet for O(1) lookup
        let filenames_to_check: HashSet<_> = cdb
            .records
            .iter()
            .enumerate()
            .map(|(count, record)| {
                if count % RECORD_DIVISOR == 0 {
                    app.substatus(
                        "Compare Databases",
                        count * 100 / total,
                        &format!("Processing Records into Memory: {}/{}", count, total),
                    );
                }

                record.get_filename()
            })
            .collect();
        app.substatus(
            "Compare Databases",
            100,
            &format!("Processing Records into Memory: {}/{}", total, total),
        );

        println!("filenames to check: {:?}", filenames_to_check);

        // Convert Arc to Vec, modify in parallel, and convert back
        total = self.records.len();
        self.records
            .par_iter_mut()
            .enumerate()
            .for_each(|(count, record)| {
                if count % RECORD_DIVISOR == 0 {
                    app.substatus(
                        "Compare Databases",
                        count * 100 / total,
                        &format!("Comparing against Database: {}/{}", count, total),
                    );
                }

                if filenames_to_check.contains(record.get_filename()) {
                    record.algorithm.insert(A::Compare);
                    record.algorithm.remove(&A::Keep);
                }
            });
        app.substatus(
            "Compare Databases",
            100,
            &format!("Comparing against Database: {}/{}", total, total),
        );
    }

    pub fn dupe_search(&mut self, pref: &Preferences, enabled: &Enabled, app: &AppHandle) {
        println!("Starting Duplicate Search");

        let mut file_groups: HashMap<Vec<Arc<str>>, Vec<FileRecord>> =
            HashMap::with_capacity(self.records.len() / 2);

        let total = self.records.len();
        let mut count = 0;

        // Group records by root
        for record in &*self.records {
            if self.abort.load(Ordering::SeqCst) {
                println!("Aborting duplicate search - early exit");
                return;
            }
            count += 1;
            if count % RECORD_DIVISOR == 0 {
                app.substatus(
                    "Duplicate Search",
                    count * 100 / total,
                    &format!("Oraginizing Records: {}/{}", count, total),
                );
            }

            let mut key = Vec::new();
            for m in &pref.match_criteria {
                if &**m == "Filename" && (enabled.filename || enabled.audiosuite) {
                    key.push(record.root.clone());
                } else {
                    let value = record.data.get(m).cloned().unwrap_or_else(|| Arc::from(""));
                    key.push(value);
                }
            }
            file_groups.entry(key).or_default().push(record.clone());
        }
        app.substatus(
            "Duplicate Search",
            100,
            &format!("Oraginizing Records: {}/{}", total, total),
        );

        println!("marking dupes");

        // Determine whether to filter out single-record groups
        let processed_records: Vec<FileRecord> = file_groups
            .into_iter()
            .enumerate()
            .flat_map(|(count, (_, mut records))| {
                if self.abort.load(Ordering::SeqCst) {
                    println!("Aborting duplicate search - early exit");
                    return Vec::new();
                }
                if count % RECORD_DIVISOR == 0 {
                    app.substatus(
                        "Duplicate Search",
                        count * 100 / total,
                        &format!("Marking Duplicates: {}/{}", count, total),
                    );
                }
                if records.len() < 2 {
                    return records;
                }
                pref.sort_vec(&mut records);

                records.iter_mut().enumerate().for_each(|(i, record)| {
                    if !(enabled.audiosuite || enabled.filename)
                        || &*record.root == record.get_filename().trim()
                        || &*record.root == record.get_filestem().trim()
                    {
                        record.algorithm.insert(A::Basic);
                        if i > 0 {
                            record.algorithm.remove(&A::Keep);
                        }
                    } else if pref.check_tags(record.get_filestem()) {
                        if enabled.audiosuite {
                            record.algorithm.insert(A::Tags);
                            if i > 0 {
                                record.algorithm.remove(&A::Keep);
                            }
                        }
                    } else if enabled.filename {
                        record.algorithm.insert(A::SimilarFilename);
                        if i > 0 {
                            record.algorithm.remove(&A::Keep);
                        }
                    }
                });

                // If all records have Keep algorithm, simplify them to just Keep
                if records
                    .iter()
                    .all(|record| record.algorithm.contains(&A::Keep))
                {
                    records.iter_mut().for_each(|record| {
                        record.algorithm.clear();
                        record.algorithm.insert(A::Keep);
                    });
                }

                records.into_iter().collect::<Vec<_>>()
            })
            .collect();
        app.substatus(
            "Duplicate Search",
            100,
            &format!("Marking Duplicates: {}/{}", total, total),
        );

        self.records = processed_records;

        println!("all done!");
    }

    pub async fn records_2_frontend(&self, app: &AppHandle) -> Vec<Vec<FileRecordFrontend>> {
        // First convert all records to frontend format
        dprintln!("Converting Records to Frontend Format");
        app.substatus(
            "Converting Records",
            0,
            "Converting records to frontend format",
        );
        let frontend: Vec<FileRecordFrontend> = self
            .records
            .par_iter()
            .enumerate()
            .map(|(i, record)| {
                if i % RECORD_DIVISOR == 0 {
                    app.substatus(
                        "Converting Records",
                        i * 100 / self.records.len(),
                        &format!("Converting Records: {}/{}", i, self.records.len()),
                    );
                }
                // dprintln!(
                //     "Converting Record {}: {}",
                //     i,
                //     record.get_filename().to_string()
                // );
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
            .collect();

        app.substatus("Converting Records", 100, "Conversion complete");
        dprintln!("Conversion to Frontend Format Complete");
        // Group records efficiently
        let mut results = Vec::new();
        let mut current_group = Vec::new();
        let mut first_in_group = true;
        for (i, record) in frontend.into_iter().enumerate() {
            if i % RECORD_DIVISOR == 0 {
                // Update status every RECORD_DIVISOR iterations
                app.status(
                    "Final Checks",
                    i * 100 / self.records.len(),
                    &format!("Grouping records for results: {}/{}", i, self.records.len()),
                );
            }

            // dprintln!("Processing Record {}: {}", i, record.filename.to_string());
            // If this record has Keep and isn't the first record being processed
            // (which would make current_group empty), end the current group
            if record.algorithm.contains(&A::Keep) && !first_in_group {
                // Only push non-empty groups
                if !current_group.is_empty() {
                    results.push(std::mem::take(&mut current_group));
                }
            }

            // Add record to current group
            current_group.push(record);
            first_in_group = false;
        }
        dprintln!("Finalizing Groups");

        // Don't forget to add the last group
        if !current_group.is_empty() {
            results.push(current_group);
        }
        dprintln!("Returning Results");

        results
    }

    pub async fn dual_mono_search(&mut self, pref: &Preferences, app: &AppHandle) {
        println!(
            "Dual Mono Search started for db: {}",
            self.get_name().unwrap_or_default()
        );
        let pool = self.get_pool_sqlite().await.ok();
        println!("Starting Dual Mono Search");
        let total = self.records.len();
        let completed = AtomicUsize::new(0);
        // let batch_size = 2000;
        let mut chunks_completed = 0;
        let mut records_batch = Vec::with_capacity(pref.batch_size);

        app.status("Dual Mono Search", 0, "Starting Dual Mono Search");
        for chunk in self.records.chunks_mut(pref.batch_size) {
            if self.abort.load(Ordering::SeqCst) {
                println!("Aborting dual mono search - early exit");
                break;
            }
            // First collect results from parallel processing
            let records_to_update = {
                chunk
                    .par_iter_mut()
                    .filter_map(|record: &mut FileRecord| {
                        let new_completed = completed.fetch_add(1, Ordering::SeqCst) + 1;

                        if record.algorithm.contains(&A::InvalidPath) {
                            return None;
                        }
                        // if !record.check_path() {
                        //     return None;
                        // }
                        if let Some(b) = record.dual_mono {
                            if b {
                                record.algorithm.insert(A::DualMono);
                            }
                            return None;
                        }

                        app.substatus(
                            "Dual Mono Search",
                            new_completed % pref.batch_size * 100 / pref.batch_size,
                            &format!("Dual Mono Search: {}/{}", new_completed, total),
                        );
                        let is_identical = audio::decode::are_channels_identical(&record.path);
                        record.dual_mono = Some(is_identical);
                        if is_identical {
                            record.algorithm.insert(A::DualMono);
                        } else {
                            record.algorithm.remove(&A::DualMono);
                        }
                        Some((record.id, is_identical))
                    })
                    .collect::<Vec<(usize, bool)>>()
            }; // Mutable borrow of self.records (through chunk) ends here

            records_batch.extend(records_to_update);

            if pref.store_waveforms && records_batch.len() >= pref.batch_size {
                let Some(pool) = &pool else {
                    app.substatus("Dual Mono Search", 0, "No database connection");
                    continue;
                };
                app.substatus("Dual Mono Search", 0, "storing chunk to database");
                let to_db: Vec<(usize, &str)> = records_batch
                    .iter()
                    .map(|(id, is_identical)| (*id, if *is_identical { "1" } else { "0" }))
                    .collect();

                crate::batch_store_data_optimized(pool, &to_db, "_DualMono", app).await;
                records_batch.clear();
            }
            chunks_completed += pref.batch_size;
            app.status(
                "Dual Mono Search",
                100 * chunks_completed / total,
                &format!("Dual Mono Search: {}/{}", chunks_completed, total),
            );
            // Then transform the results into the format needed for batch_store_data_optimized
        }
        if pref.store_waveforms && !records_batch.is_empty() {
            let Some(pool) = &pool else {
                app.substatus("Dual Mono Search", 0, "No database connection");
                return;
            };
            app.substatus("Dual Mono Search", 0, "storing chunk to database");
            let to_db: Vec<(usize, &str)> = records_batch
                .iter()
                .map(|(id, is_identical)| (*id, if *is_identical { "1" } else { "0" }))
                .collect();

            crate::batch_store_data_optimized(pool, &to_db, "_DualMono", app).await;
            records_batch.clear();
        }
    }
    pub async fn dual_mono_search_seq(&mut self, app: &AppHandle) {
        println!("Starting Dual Mono Search");
        let total = self.records.len();
        let mut completed = 0;

        // Store paths of records identified as dual mono
        let mut dual_mono_paths = std::collections::HashSet::new();

        // Process in smaller batches to allow other functions to run
        let batch_size = 100; // Adjust based on your system

        {
            // Use immutable reference to avoid multiple mutable borrows
            let records = &self.records;

            for i in (0..records.len()).step_by(batch_size) {
                if self.abort.load(Ordering::SeqCst) {
                    println!("Aborting dual mono search - early exit");
                    break;
                }

                let chunk_end = std::cmp::min(i + batch_size, records.len());
                let mut futures = Vec::new();

                // Process each record in the batch concurrently
                for record in &records[i..chunk_end] {
                    let record_path = record.path.clone();
                    let record_filename = record.get_filename().to_string();
                    let channels = record.channels;
                    let can_check_path = record.check_path();

                    // This properly moves blocking work to another thread
                    let future = tokio::task::spawn_blocking(move || {
                        let identical = audio::decode::are_channels_identical(&record_path);
                        println!("Checking: {} result: {}", record_filename, identical);
                        (record_path, channels > 1 && can_check_path && identical)
                    });

                    futures.push(future);
                }

                // Wait for all tasks in this batch to complete
                for future in futures {
                    if let Ok((path, should_mark)) = future.await {
                        completed += 1;
                        app.substatus(
                            "Duplicate Search",
                            completed * 100 / total,
                            &format!("Dual Mono Search: {}/{}", completed, total),
                        );

                        if should_mark {
                            dual_mono_paths.insert(path);
                        }
                    }
                }

                // This critical line yields control back to the runtime
                tokio::task::yield_now().await;
            }
        } // End of immutable borrow

        // Now update all the records that were identified as dual mono
        for record in &mut self.records {
            if dual_mono_paths.contains(&record.path) {
                record.algorithm.insert(A::DualMono);
            }
        }
    }
}
