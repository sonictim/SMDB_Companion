use crate::*;

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose;
use bit_set::BitSet;
use preferences::*;
use rayon::prelude::*;
pub use sqlx::sqlite::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

use commands::*;

impl Database {
    pub async fn compare_search(&mut self, enabled: &Enabled, pref: &Preferences, app: &AppHandle) {
        let mut cdb = Database::default();
        cdb.init(Some(PathBuf::from(&*enabled.compare_db)), true)
            .await;

        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "compare".into(),
                progress: 0,
                message: "Gathering Records".into(),
            },
        )
        .ok();
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
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "compare".into(),
                            progress: (count * 100 / total) as u64,
                            message: format!("Processing Records into Memory: {}/{}", count, total),
                        },
                    )
                    .ok();
                }

                record.get_filename()
            })
            .collect();
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "compare".into(),
                progress: 100,
                message: format!("Processing Records into Memory: {}/{}", total, total),
            },
        )
        .ok();

        println!("filenames to check: {:?}", filenames_to_check);

        // Convert Arc to Vec, modify in parallel, and convert back
        total = self.records.len();
        self.records
            .par_iter_mut()
            .enumerate()
            .for_each(|(count, record)| {
                if count % RECORD_DIVISOR == 0 {
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "compare".into(),
                            progress: (count * 100 / total) as u64,
                            message: format!("Comparing against Database: {}/{}", count, total),
                        },
                    )
                    .ok();
                }

                if filenames_to_check.contains(record.get_filename()) {
                    record.algorithm.insert(A::Compare);
                    record.algorithm.remove(&A::Keep);
                }
            });
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "compare".into(),
                progress: 100,
                message: format!("Comparing against Database: {}/{}", total, total),
            },
        )
        .ok();
    }

    pub fn dupe_search(&mut self, pref: &Preferences, enabled: &Enabled, app: &AppHandle) {
        println!("Starting Duplicate Search");

        let mut file_groups: HashMap<Vec<Arc<str>>, Vec<FileRecord>> =
            HashMap::with_capacity(self.records.len() / 2);

        let total = self.records.len();
        let mut count = 0;

        // Group records by root
        for record in &*self.records {
            count += 1;
            if count % RECORD_DIVISOR == 0 {
                app.emit(
                    "search-sub-status",
                    StatusUpdate {
                        stage: "dupes".into(),
                        progress: (count * 100 / total) as u64,
                        message: format!("Oraginizing Records: {}/{}", count, total),
                    },
                )
                .ok();
            }
            let mut key = Vec::new();
            for m in &pref.match_criteria {
                if &**m == "Filename" && (enabled.filename || enabled.audiosuite) {
                    key.push(record.root.clone());
                } else {
                    key.push(record.data[m].clone());
                }
            }
            file_groups.entry(key).or_default().push(record.clone());
        }
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "dupes".into(),
                progress: 100,
                message: format!("Oraginizing Records: {}/{}", total, total),
            },
        )
        .ok();

        println!("marking dupes");

        // Determine whether to filter out single-record groups
        let processed_records: Vec<FileRecord> = file_groups
            .into_iter()
            .enumerate()
            .flat_map(|(count, (_, mut records))| {
                if count % RECORD_DIVISOR == 0 {
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "dupes".into(),
                            progress: (count * 100 / total) as u64,
                            message: format!("Marking Duplicates: {}/{}", count, total),
                        },
                    )
                    .ok();
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

        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "dupes".into(),
                progress: 100,
                message: format!("Marking Duplicates: {}/{}", total, total),
            },
        )
        .ok();
        self.records = processed_records;

        println!("all done!");
    }

    async fn exact_match(&mut self, pref: &Preferences, app: &AppHandle) -> Result<(), String> {
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "grouping".into(),
                progress: 0,
                message: "Grouping identical audio fingerprints...".into(),
            },
        )
        .ok();

        let records_without_fingerprints: Vec<FileRecord> = self
            .records
            .iter()
            .filter(|record| record.fingerprint.is_none())
            .cloned()
            .collect();

        let mut file_groups: HashMap<Arc<str>, Vec<FileRecord>> =
            HashMap::with_capacity(self.records.len() / 2);

        // Group records by waveform
        for (i, record) in self.records.iter().enumerate() {
            if *self.abort.read().await {
                return Err("Aborted".to_string());
            };
            if i % RECORD_DIVISOR == 0 || i == 0 || i == self.records.len() - 1 {
                app.emit(
                    "search-sub-status",
                    StatusUpdate {
                        stage: "grouping".into(),
                        progress: ((i + 1) * 100 / self.records.len()) as u64,
                        message: format!(
                            "Grouping by fingerprint: {}/{}",
                            i + 1,
                            self.records.len()
                        ),
                    },
                )
                .ok();
            }

            if let Some(fingerprint) = &record.fingerprint {
                file_groups
                    .entry(fingerprint.clone())
                    .or_default()
                    .push(record.clone());
            }
        }

        println!("Marking duplicate audio files");
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "marking".into(),
                progress: 0,
                message: "Marking duplicate audio files...".into(),
            },
        )
        .ok();

        // Process groups
        let group_count = file_groups.len();
        let processed_records: Vec<FileRecord> = file_groups
            .into_iter()
            .enumerate()
            .flat_map(|(i, (_, mut records))| {
                if i % RECORD_DIVISOR == 0 || i == 0 || i == group_count - 1 {
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "marking".into(),
                            progress: ((i + 1) * 100 / group_count) as u64,
                            message: format!("Processing group: {}/{}", i + 1, group_count),
                        },
                    )
                    .ok();
                }

                if records.len() < 2 {
                    return records;
                }
                pref.sort_vec(&mut records);

                records.iter_mut().enumerate().for_each(|(i, record)| {
                    if i > 0 {
                        record.algorithm.remove(&A::Keep);
                    }
                    record.algorithm.insert(A::Waveforms);
                });

                records
            })
            .collect();

        let mut final_records = processed_records;
        final_records.extend(records_without_fingerprints);
        self.records = final_records;

        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "complete".into(),
                progress: 100,
                message: "Exact Audio fingerprint analysis complete".into(),
            },
        )
        .ok();

        Ok(())
    }

    async fn similar_match(&mut self, pref: &Preferences, app: &AppHandle) -> Result<(), String> {
        // Use lower default threshold if needed
        let threshold = pref.similarity_threshold / 100.0;

        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "similarity".into(),
                progress: 0,
                message: "Starting similarity-based audio comparison...".into(),
            },
        )
        .ok();

        // Keep track of all records, not just ones with fingerprints
        let all_records = self.records.clone();

        // PROGRESS: Update after preparing records
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "similarity".into(),
                progress: 5,
                message: "Finding records with fingerprints...".into(),
            },
        )
        .ok();

        // Filter records with valid fingerprints
        let records_with_fingerprints: Vec<&FileRecord> = self
            .records
            .iter()
            .filter(|record| record.fingerprint.as_ref().is_some_and(|fp| !fp.is_empty()))
            .collect();

        println!(
            "Found {} records with valid fingerprints",
            records_with_fingerprints.len()
        );

        // PROGRESS: Update before starting fingerprint decoding
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "similarity".into(),
                progress: 10,
                message: format!(
                    "Decoding {} fingerprints...",
                    records_with_fingerprints.len()
                ),
            },
        )
        .ok();

        // USE the build_similarity_groups function with progress monitoring
        let total_records = records_with_fingerprints.len();
        let similarity_groups = {
            // First pass - decode all fingerprints with progress
            let decoded_fps: Vec<Option<Vec<u32>>> = records_with_fingerprints
                .par_iter()
                .enumerate()
                .map(|(i, record)| {
                    // Report progress every 5% of records processed
                    if i % (total_records.max(20) / 100) == 0 {
                        app.emit(
                            "search-sub-status",
                            StatusUpdate {
                                stage: "similarity".into(),
                                progress: 10 + ((i * 30) / total_records) as u64,
                                message: format!("Decoding fingerprints: {}/{}", i, total_records),
                            },
                        )
                        .ok();
                    }

                    if let Some(raw_fp) = &record.fingerprint {
                        if let Ok(fp_bytes) = general_purpose::STANDARD.decode(raw_fp.as_ref()) {
                            // Decode fingerprint...
                            let mut fp = Vec::with_capacity(fp_bytes.len() / 4);
                            for chunk in fp_bytes.chunks_exact(4) {
                                if chunk.len() == 4 {
                                    let mut array = [0u8; 4];
                                    array.copy_from_slice(chunk);
                                    fp.push(u32::from_le_bytes(array));
                                }
                            }
                            return Some(fp);
                        }
                    }
                    None
                })
                .collect();

            // PROGRESS: Update before starting comparisons
            app.emit(
                "search-sub-status",
                StatusUpdate {
                    stage: "similarity".into(),
                    progress: 40,
                    message: "Comparing fingerprints for similarities...".into(),
                },
            )
            .ok();

            // Second pass - build groups with similarity with progress updates
            let mut groups: HashMap<usize, Vec<(usize, Vec<u32>)>> = HashMap::new();
            let mut next_group_id = 0;

            // Process in smaller batches and report progress
            for i in 0..total_records {
                if i % (total_records.max(20) / 100) == 0 {
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "similarity".into(),
                            progress: 40 + ((i * 30) / total_records) as u64,
                            message: format!("Finding similar audio: {}/{}", i, total_records),
                        },
                    )
                    .ok();
                }

                let idx = records_with_fingerprints[i].id;
                if let Some(ref fp_i) = decoded_fps[i] {
                    // Comparison logic...
                    let mut found_group = None;

                    for (&group_id, group_members) in &groups {
                        for (_, group_fp) in group_members {
                            let similarity = calculate_similarity_simd(fp_i, group_fp);
                            if similarity >= threshold {
                                found_group = Some(group_id);
                                break;
                            }
                        }
                        if found_group.is_some() {
                            break;
                        }
                    }

                    if let Some(group_id) = found_group {
                        groups.get_mut(&group_id).unwrap().push((idx, fp_i.clone()));
                    } else {
                        groups.insert(next_group_id, vec![(idx, fp_i.clone())]);
                        next_group_id += 1;
                    }
                }
            }

            groups
        };

        // PROGRESS: Update before processing groups
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "similarity".into(),
                progress: 70,
                message: format!(
                    "Processing {} similarity groups...",
                    similarity_groups.len()
                ),
            },
        )
        .ok();

        // Process groups with progress updates
        let mut processed_records = Vec::with_capacity(self.records.len());
        let mut processed_ids = BitSet::with_capacity(self.records.len());

        // Add all records from similarity groups with progress
        let total_groups = similarity_groups.len();
        for (i, (_, group)) in similarity_groups.iter().enumerate() {
            // Update every 5% of groups processed
            if i % (total_groups.max(20) / 20) == 0 {
                app.emit(
                    "search-sub-status",
                    StatusUpdate {
                        stage: "similarity".into(),
                        progress: 70 + ((i * 20) / total_groups) as u64,
                        message: format!("Sorting similar groups: {}/{}", i, total_groups),
                    },
                )
                .ok();
            }

            if group.len() > 1 {
                // Sort and prepare group records
                let mut group_records: Vec<FileRecord> = group
                    .iter()
                    .filter_map(|(idx, _)| all_records.iter().find(|r| r.id == *idx).cloned())
                    .collect();

                // Sort by preference
                pref.sort_vec(&mut group_records);

                // Process and mark duplicates
                for (j, record) in group_records.iter_mut().enumerate() {
                    // Mark as processed
                    processed_ids.insert(record.id);

                    // Add algorithm flag
                    record.algorithm.insert(Algorithm::SimilarAudio);

                    // Remove Keep from duplicates
                    if j > 0 {
                        record.algorithm.remove(&A::Keep);
                    }

                    // Add to processed records
                    processed_records.push(record.clone());
                }
            }
        }

        // PROGRESS: Update before final record collection
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "similarity".into(),
                progress: 90,
                message: "Finalizing records...".into(),
            },
        )
        .ok();

        // Add any records not in a group
        for record in all_records {
            if !processed_ids.contains(record.id) {
                processed_records.push(record);
            }
        }

        self.records = processed_records;

        // PROGRESS: Final completion update
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "similarity".into(),
                progress: 100,
                message: format!(
                    "Similarity analysis complete: found {} groups with similar audio",
                    similarity_groups.len()
                ),
            },
        )
        .ok();

        Ok(())
    }

    pub async fn wave_search_chromaprint(
        &mut self,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), String> {
        println!("Starting Waveform Search");
        self.gather_fingerprints3(pref, app).await?;
        if *self.abort.read().await {
            return Err("Aborted".to_string());
        };
        if pref.exact_waveform {
            self.exact_match(pref, app).await?;
        } else {
            self.similar_match(pref, app).await?;
        }

        Ok(())
    }

    async fn gather_fingerprints3(
        &mut self,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), String> {
        println!("Starting fingerprint collection");

        // MORE AGGRESSIVE OPTIMIZATIONS
        const BATCH_SIZE: usize = 100; // Larger batch size for fewer transactions
        const STORAGE_INTERVAL: usize = 250; // Store every 250 records processed
        const PROGRESS_INTERVAL: usize = RECORD_DIVISOR * 5; // Less frequent progress updates

        // 1. PRE-FILTERING OPTIMIZATION: Use indexed access for faster filtering
        let mut records_needing_fingerprints = BitSet::with_capacity(self.records.len());
        for (idx, record) in self.records.iter().enumerate() {
            // Skip invalid paths completely - no need to fingerprint them
            // if record.algorithm.contains(&Algorithm::InvalidPath) {
            //     continue;
            // }

            let path = PathBuf::from(record.get_filepath());
            if !path.exists() || !path.is_file() {
                // Skip non-file paths
                continue;
            }

            if record.fingerprint.is_none() {
                records_needing_fingerprints.insert(idx);
            }
        }

        if records_needing_fingerprints.is_empty() {
            println!("No fingerprints needed - all files already processed");
            return Ok(());
        }

        let total_to_process = records_needing_fingerprints.len();
        println!("Found {} files requiring fingerprinting", total_to_process);
        println!("{:#?}", records_needing_fingerprints);

        // 2. CHECKPOINT OPTIMIZATION: Save progress marker to restart from
        let mut checkpoint_file = std::env::temp_dir();
        checkpoint_file.push("smdb_fingerprint_progress.json");

        let mut start_from = 0;
        if checkpoint_file.exists() && pref.store_waveforms {
            if let Ok(contents) = std::fs::read_to_string(&checkpoint_file) {
                if let Ok(checkpoint) = serde_json::from_str::<usize>(&contents) {
                    if checkpoint < total_to_process {
                        start_from = checkpoint;
                        println!(
                            "Resuming from checkpoint: {} of {}",
                            start_from, total_to_process
                        );
                    }
                }
            }
        }

        // 3. MEMORY OPTIMIZATION: Store fingerprints more frequently in smaller chunks
        let mut fingerprints_to_store = Vec::with_capacity(STORAGE_INTERVAL);
        let mut processed = start_from;

        // 4. CONNECTION POOL OPTIMIZATION: Get connection pool once, with retry logic
        let db_pool = if pref.store_waveforms {
            match self.get_pool().await {
                Some(pool) => Some(pool),
                None => {
                    // Retry connection once after a short delay
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    self.get_pool().await
                }
            }
        } else {
            None
        };

        // 5. CPU UTILIZATION OPTIMIZATION: Limit parallelism based on available cores
        let cpu_count = num_cpus::get();
        let thread_count = std::cmp::min(cpu_count, 8); // Cap at 8 threads to avoid I/O thrashing
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

        println!("Processing with {} threads", thread_count);

        // 6. BATCH PROCESSING OPTIMIZATION: Process in chunks
        for chunk_idx in (start_from..total_to_process).step_by(BATCH_SIZE) {
            // Check abort with non-blocking access
            if let Ok(guard) = self.abort.try_read() {
                if *guard {
                    // Handle cancellation with checkpoint
                    if !fingerprints_to_store.is_empty() && pref.store_waveforms {
                        app.emit(
                            "search-sub-status",
                            StatusUpdate {
                                stage: "saving".into(),
                                progress: 100,
                                message: "Cancelled - saving checkpoint...".into(),
                            },
                        )
                        .ok();

                        // Store current fingerprints
                        if let Some(ref pool) = db_pool {
                            store_fingerprints_batch_optimized(pool, &fingerprints_to_store, app)
                                .await;
                        }

                        // Save checkpoint
                        let _ = std::fs::write(&checkpoint_file, processed.to_string());
                    }
                    return Err("Aborted".to_string());
                }
            }

            // Calculate end of current chunk (capped at total)
            let end_idx = std::cmp::min(chunk_idx + BATCH_SIZE, total_to_process);
            // Extract indices from BitSet for the current chunk's range
            let current_indices: Vec<usize> = (chunk_idx..end_idx)
                .filter(|&idx| records_needing_fingerprints.contains(idx))
                .collect();

            // 7. PARALLEL PROCESSING OPTIMIZATION: Use thread pool for controlled parallelism
            let batch_results: Vec<_> = thread_pool.install(|| {
                current_indices
                    .par_iter()
                    .filter_map(|&idx| {
                        let record = &self.records[idx];

                        // Skip invalid paths
                        if record.algorithm.contains(&Algorithm::InvalidPath) {
                            return None;
                        }

                        // 8. FAST PATH OPTIMIZATION: Skip if already has fingerprints
                        if record.fingerprint.is_some() {
                            return None;
                        }

                        // Convert Option<String> to Option<Arc<str>> and return the result
                        let fingerprint = match audio::get_chromaprint_fingerprint(&record.path) {
                            Some(fp) => {
                                println!("SUCCESS fingerprint for: {}", record.get_filepath());
                                Some(Arc::from(fp.as_str()))
                            }
                            None => {
                                println!(
                                    "FAILED fingerprint for: {} (exists: {}, size: {})",
                                    record.get_filepath(),
                                    Path::new(record.get_filepath()).exists(),
                                    Path::new(record.get_filepath())
                                        .metadata()
                                        .map_or(0, |m| m.len())
                                );
                                None
                            }
                        };

                        Some((idx, fingerprint))
                    })
                    .collect()
            });

            // 11. IN-MEMORY UPDATE OPTIMIZATION: Update records directly
            for (idx, fingerprint) in &batch_results {
                let record = &mut self.records[*idx];
                record.fingerprint = fingerprint.clone();
            }

            // Add to storage queue
            fingerprints_to_store.extend(batch_results.iter().map(|(idx, fingerprint)| {
                (
                    self.records[*idx].id,
                    fingerprint.clone().unwrap_or_default().to_string(),
                )
            }));

            // 12. STORAGE OPTIMIZATION: Store in smaller, more frequent batches
            println!(
                "Storage conditions: store_waveforms={}, empty={}, interval={}, processed={}, total={}, store_waveforms={}",
                pref.store_waveforms,
                fingerprints_to_store.is_empty(),
                processed % STORAGE_INTERVAL,
                processed,
                total_to_process,
                pref.store_waveforms
            );
            processed += batch_results.len();
            if pref.store_waveforms
                && !fingerprints_to_store.is_empty()
                && (processed % STORAGE_INTERVAL == 0 || processed == total_to_process)
            {
                if let Some(ref pool) = db_pool {
                    // Less frequent progress updates for storage
                    if processed % PROGRESS_INTERVAL == 0 || processed == total_to_process {
                        app.emit(
                            "search-sub-status",
                            StatusUpdate {
                                stage: "saving".into(),
                                progress: (processed * 100 / total_to_process) as u64,
                                message: format!(
                                    "Saving batch: {}/{} processed",
                                    processed, total_to_process
                                ),
                            },
                        )
                        .ok();
                    }

                    // 13. DATABASE OPTIMIZATION: Use optimized batch storage
                    store_fingerprints_batch_optimized(pool, &fingerprints_to_store, app).await; // Pass app here
                    fingerprints_to_store.clear();

                    // 14. CHECKPOINT OPTIMIZATION: Save progress periodically
                    let _ = std::fs::write(&checkpoint_file, processed.to_string());
                }
            }

            // 15. PROGRESS REPORTING OPTIMIZATION: Less frequent updates
            if processed % 2 == 0 || processed == total_to_process {
                // if processed % PROGRESS_INTERVAL == 0 || processed == total_to_process {
                // Get current filename being processed (from last batch)
                let current_file = if let Some(&idx) = current_indices.first() {
                    self.records[idx].get_filename()
                } else {
                    "Unknown"
                };

                app.emit(
                    "search-sub-status",
                    StatusUpdate {
                        stage: "fingerprinting".into(),
                        progress: (processed * 100 / total_to_process) as u64,
                        message: format!(
                            "Processing: ({}/{}) {} ",
                            processed, total_to_process, current_file
                        ),
                    },
                )
                .ok();
            }
        }

        // Remove checkpoint file on successful completion
        if processed >= total_to_process {
            let _ = std::fs::remove_file(checkpoint_file);
        }

        Ok(())
    }
}

// 16. OPTIMIZED DATABASE STORAGE: Efficient batch insertion
/// Optimized batch storage of audio fingerprints to database
async fn store_fingerprints_batch_optimized(
    pool: &SqlitePool,
    fingerprints: &[(usize, String)],
    app: &AppHandle,
) {
    if fingerprints.is_empty() {
        println!("No fingerprints to store");
        return;
    }

    println!("Storing {} fingerprints in database", fingerprints.len());

    // Emit initial status
    app.emit(
        "search-sub-status",
        StatusUpdate {
            stage: "db-storage".into(),
            progress: 0,
            message: format!("Storing {} fingerprints in database...", fingerprints.len()),
        },
    )
    .ok();

    // Begin a single transaction for all updates
    match pool.begin().await {
        Ok(mut tx) => {
            // Set optimized pragmas for better throughput
            let _ = sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&mut *tx)
                .await;
            let _ = sqlx::query("PRAGMA synchronous = NORMAL")
                .execute(&mut *tx)
                .await;

            // Process all fingerprints in the batch
            let total = fingerprints.len();
            let mut success_count = 0;
            let mut error_count = 0;

            for (i, (id, fingerprint)) in fingerprints.iter().enumerate() {
                // Emit progress updates periodically
                if i % 25 == 0 || i == total - 1 {
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "db-storage".into(),
                            progress: ((i + 1) * 100 / total) as u64,
                            message: format!("Storing fingerprints: {}/{}", i + 1, total),
                        },
                    )
                    .ok();
                }

                let result = sqlx::query(&format!(
                    "UPDATE {} SET _fingerprint = ? WHERE rowid = ?",
                    TABLE
                ))
                .bind(fingerprint)
                .bind(*id as i64)
                .execute(&mut *tx)
                .await;

                match result {
                    Ok(result) => {
                        if result.rows_affected() == 0 {
                            println!(
                                "WARNING: No rows affected when updating fingerprints for ID {}",
                                id
                            );
                        } else {
                            success_count += 1;
                        }
                    }
                    Err(e) => {
                        println!("ERROR updating fingerprints for ID {}: {}", id, e);
                        error_count += 1;
                    }
                }
            }

            // Final update before commit
            app.emit(
                "search-sub-status",
                StatusUpdate {
                    stage: "db-storage".into(),
                    progress: 99,
                    message: "Committing all changes to database...".to_string(),
                },
            )
            .ok();

            // Commit the transaction
            match tx.commit().await {
                Ok(_) => {
                    println!(
                        "Transaction committed successfully: {} fingerprints updated, {} errors",
                        success_count, error_count
                    );

                    // Force a checkpoint to update the main DB file
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

            // Confirm completion
            app.emit(
                "search-sub-status",
                StatusUpdate {
                    stage: "db-storage".into(),
                    progress: 100,
                    message: format!(
                        "Database update complete: {} fingerprints stored",
                        success_count
                    ),
                },
            )
            .ok();
        }
        Err(e) => {
            println!("ERROR: Failed to start transaction: {}", e);
            app.emit(
                "search-sub-status",
                StatusUpdate {
                    stage: "db-storage".into(),
                    progress: 100,
                    message: format!("ERROR: Database update failed: {}", e),
                },
            )
            .ok();
        }
    }
}

// Target-specific SIMD optimizations
#[cfg(target_arch = "x86_64")]
fn calculate_similarity_simd(fp1: &[u32], fp2: &[u32]) -> f64 {
    use std::arch::x86_64::*;

    let min_len = fp1.len().min(fp2.len());
    if min_len == 0 {
        return 0.0;
    }

    if is_x86_feature_detected!("sse2") {
        unsafe {
            // Original x86_64 implementation
            let chunk_size = 4;
            let mut matching_bits = 0;
            let mut chunks_processed = 0;

            for i in (0..min_len - (min_len % chunk_size)).step_by(chunk_size) {
                let a = _mm_loadu_si128(fp1[i..].as_ptr() as *const __m128i);
                let b = _mm_loadu_si128(fp2[i..].as_ptr() as *const __m128i);
                let xor = _mm_xor_si128(a, b);

                let count = (_mm_extract_epi64(xor, 0) as u64).count_ones()
                    + (_mm_extract_epi64(xor, 1) as u64).count_ones();

                matching_bits += 128 - count as usize;
                chunks_processed += 1;
            }

            // Process remaining elements
            for i in (chunks_processed * chunk_size)..min_len {
                let xor_result = fp1[i] ^ fp2[i];
                matching_bits += 32 - xor_result.count_ones() as usize;
            }

            return matching_bits as f64 / (min_len * 32) as f64;
        }
    }

    // Fallback to scalar implementation
    calculate_similarity(fp1, fp2)
}

// ARM (Apple Silicon) implementation
#[cfg(target_arch = "aarch64")]
fn calculate_similarity_simd(fp1: &[u32], fp2: &[u32]) -> f64 {
    use std::arch::aarch64::*;

    let min_len = fp1.len().min(fp2.len());
    if min_len == 0 {
        return 0.0;
    }

    unsafe {
        // Process 4 u32s (128 bits) at once with NEON
        let chunk_size = 4;
        let mut matching_bits = 0;
        let mut chunks_processed = 0;

        for i in (0..min_len - (min_len % chunk_size)).step_by(chunk_size) {
            // Load 128 bits from each fingerprint
            let a = vld1q_u32(fp1[i..].as_ptr());
            let b = vld1q_u32(fp2[i..].as_ptr());

            // XOR the values
            let xor_result = veorq_u32(a, b);

            // Count bits using ARM's NEON instructions
            let mut cnt = vcntq_u8(vreinterpretq_u8_u32(xor_result));
            let sum = vaddv_u8(vget_low_u8(cnt)) + vaddv_u8(vget_high_u8(cnt));

            matching_bits += 128 - sum as usize;
            chunks_processed += 1;
        }

        // Process remaining elements
        for i in (chunks_processed * chunk_size)..min_len {
            let xor_result = fp1[i] ^ fp2[i];
            matching_bits += 32 - xor_result.count_ones() as usize;
        }

        return matching_bits as f64 / (min_len * 32) as f64;
    }
}

// Fallback for other architectures
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn calculate_similarity_simd(fp1: &[u32], fp2: &[u32]) -> f64 {
    calculate_similarity(fp1, fp2)
}

// Helper function to calculate similarity between two fingerprints
fn calculate_similarity(fp1: &[u32], fp2: &[u32]) -> f64 {
    let min_len = fp1.len().min(fp2.len());
    if min_len == 0 {
        return 0.0;
    }

    // Try different offsets for better matching
    const MAX_OFFSET: usize = 3;
    let mut best_similarity = 0.0;

    for offset in 0..=MAX_OFFSET {
        if offset >= min_len {
            break;
        }

        let matching_len = min_len - offset;
        let mut matching_bits = 0;
        let total_bits = matching_len * 32;

        for i in 0..matching_len {
            let xor_result = fp1[i] ^ fp2[i + offset];
            matching_bits += 32 - xor_result.count_ones() as usize;
        }

        let similarity = matching_bits as f64 / total_bits as f64;
        best_similarity = f64::max(best_similarity, similarity);

        // Try the reverse offset too
        if offset > 0 && offset < fp2.len() {
            let matching_bits = (0..matching_len)
                .map(|i| 32 - (fp1[i + offset] ^ fp2[i]).count_ones() as usize)
                .sum::<usize>();

            let similarity = matching_bits as f64 / total_bits as f64;
            best_similarity = f64::max(best_similarity, similarity);
        }
    }

    best_similarity
}

// // The group-building functionality should be moved to a method or function
// fn build_similarity_groups(
//     records_with_fingerprints: &[&FileRecord],
//     threshold: f64,
// ) -> HashMap<usize, Vec<(usize, Vec<u32>)>> {
//     // Create multi-level grouping
//     let mut groups: HashMap<usize, Vec<(usize, Vec<u32>)>> = HashMap::new();
//     let mut next_group_id = 0;
//     let total_records = records_with_fingerprints.len();

//     // First pass - decode all fingerprints
//     let decoded_fps: Vec<Option<Vec<u32>>> = records_with_fingerprints
//         .par_iter()
//         .map(|record| {
//             if let Some(raw_fp) = &record.fingerprint {
//                 if let Ok(fp_bytes) = general_purpose::STANDARD.decode(raw_fp.as_ref()) {
//                     let mut fp = Vec::with_capacity(fp_bytes.len() / 4);
//                     for chunk in fp_bytes.chunks_exact(4) {
//                         if chunk.len() == 4 {
//                             let mut array = [0u8; 4];
//                             array.copy_from_slice(chunk);
//                             fp.push(u32::from_le_bytes(array));
//                         }
//                     }
//                     return Some(fp);
//                 }
//             }
//             None
//         })
//         .collect();

//     // Second pass - build groups with similarity
//     for i in 0..total_records {
//         let idx = records_with_fingerprints[i].id;

//         if let Some(ref fp_i) = decoded_fps[i] {
//             let mut found_group = None;

//             // Try to find an existing group that this record belongs to
//             for (&group_id, group_members) in &groups {
//                 for (_, group_fp) in group_members {
//                     let similarity = calculate_similarity_simd(fp_i, group_fp);
//                     if similarity >= threshold {
//                         found_group = Some(group_id);
//                         break;
//                     }
//                 }
//                 if found_group.is_some() {
//                     break;
//                 }
//             }

//             // If we found a matching group, add the record to it
//             if let Some(group_id) = found_group {
//                 groups.get_mut(&group_id).unwrap().push((idx, fp_i.clone()));
//             } else {
//                 // If no matching group, create a new one
//                 groups.insert(next_group_id, vec![(idx, fp_i.clone())]);
//                 next_group_id += 1;
//             }
//         }
//     }

//     groups
// }
