use crate::*;

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose;
use bit_set::BitSet;
use preferences::*;
use rayon::prelude::*;
pub use sqlx::sqlite::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::{AppHandle, Emitter};
// use tokio::task;

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
            if self.abort.load(Ordering::SeqCst) {
                println!("Aborting duplicate search - early exit");
                return;
            }
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
                if self.abort.load(Ordering::SeqCst) {
                    println!("Aborting duplicate search - early exit");
                    return Vec::new();
                }
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

    pub async fn records_2_frontend(&self) -> Vec<FileRecordFrontend> {
        let results: Vec<FileRecordFrontend> = self
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
            .collect();
        results
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
            .filter(|record| {
                record.fingerprint.is_none()
                    || record
                        .fingerprint
                        .as_ref()
                        .is_some_and(|fp| &**fp == "FAILED")
            })
            .cloned()
            .collect();

        let mut file_groups: HashMap<Arc<str>, Vec<FileRecord>> =
            HashMap::with_capacity(self.records.len() / 2);

        // Group records by waveform
        for (i, record) in self.records.iter().enumerate() {
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
        let threshold = pref.similarity_threshold / 100.0;

        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "similarity".into(),
                progress: 0,
                message: "Starting combined similarity/hash analysis...".into(),
            },
        )
        .ok();

        // Keep track of all records, not just ones with fingerprints
        let all_records = self.records.clone();

        // STEP 1: Separate fingerprints into Chromaprint vs PCM hash groups
        let (chromaprint_records, pcm_hash_records): (Vec<&FileRecord>, Vec<&FileRecord>) = self
            .records
            .par_iter() // Changed from par_iter_mut() to par_iter() to match the expected immutable references
            .filter(|record| {
                // Add check to exclude FAILED fingerprints
                record
                    .fingerprint
                    .as_ref()
                    .is_some_and(|fp| !fp.is_empty() && &**fp != "FAILED")
            })
            .partition(|record| {
                let fp = record.fingerprint.as_ref().unwrap();
                !fp.starts_with("PCM:")
            });

        println!(
            "Found {} records with Chromaprint fingerprints and {} with PCM hashes",
            chromaprint_records.len(),
            pcm_hash_records.len()
        );

        // Create a BitSet to track which records have been processed
        let mut processed_ids = BitSet::with_capacity(self.records.len());
        let mut processed_records = Vec::with_capacity(self.records.len());

        // STEP 2: Process PCM hashes with exact matching (similar to exact_match function)
        if !pcm_hash_records.is_empty() {
            app.emit(
                "search-sub-status",
                StatusUpdate {
                    stage: "pcm_hash".into(),
                    progress: 0,
                    message: format!("Processing {} PCM hash records...", pcm_hash_records.len()),
                },
            )
            .ok();

            // Group PCM hash records by hash value
            let mut hash_groups: HashMap<Arc<str>, Vec<FileRecord>> = HashMap::new();

            for (i, record) in pcm_hash_records.iter().enumerate() {
                if i % RECORD_DIVISOR == 0 {
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "pcm_hash".into(),
                            progress: ((i + 1) * 50 / pcm_hash_records.len()) as u64,
                            message: format!(
                                "Grouping PCM hashes: {}/{}",
                                i + 1,
                                pcm_hash_records.len()
                            ),
                        },
                    )
                    .ok();
                }

                if let Some(hash) = &record.fingerprint {
                    hash_groups
                        .entry(hash.clone())
                        .or_default()
                        .push((*record).clone());
                }
            }

            // Process hash groups - mark duplicates
            let hash_group_count = hash_groups.len();

            for (i, (_, mut records)) in hash_groups.into_iter().enumerate() {
                if i % RECORD_DIVISOR == 0 {
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "pcm_hash".into(),
                            progress: 50 + ((i + 1) * 50 / hash_group_count) as u64,
                            message: format!(
                                "Processing hash groups: {}/{}",
                                i + 1,
                                hash_group_count
                            ),
                        },
                    )
                    .ok();
                }

                if records.len() < 2 {
                    // Single record in group - just mark it as processed
                    for record in records {
                        processed_ids.insert(record.id);
                        processed_records.push(record);
                    }
                    continue;
                }

                // Sort and mark duplicates
                pref.sort_vec(&mut records);

                for (j, mut record) in records.into_iter().enumerate() {
                    processed_ids.insert(record.id);

                    // Mark as PCM duplicate
                    record.algorithm.insert(Algorithm::Waveforms);

                    // Remove Keep from duplicates
                    if j > 0 {
                        record.algorithm.remove(&A::Keep);
                    }

                    processed_records.push(record);
                }
            }
        }

        // STEP 3: Process Chromaprint fingerprints with similarity matching
        if !chromaprint_records.is_empty() {
            // Existing similarity match code...
            app.emit(
                "search-sub-status",
                StatusUpdate {
                    stage: "similarity".into(),
                    progress: 10,
                    message: format!(
                        "Decoding {} Chromaprint fingerprints...",
                        chromaprint_records.len()
                    ),
                },
            )
            .ok();

            // Your existing code for processing fingerprints
            let total_records = chromaprint_records.len();
            let similarity_groups = {
                // First pass - decode all fingerprints with progress
                let decoded_fps: Vec<Option<Vec<u32>>> = chromaprint_records
                    .par_iter()
                    .enumerate()
                    .map(|(i, record)| {
                        // Progress reporting code...
                        let update_interval = (total_records.max(20) / 100).max(1);
                        if i % update_interval == 0 {
                            app.emit(
                                "search-sub-status",
                                StatusUpdate {
                                    stage: "similarity".into(),
                                    progress: 10 + ((i * 30) / total_records) as u64,
                                    message: format!(
                                        "Decoding fingerprints: {}/{}",
                                        i, total_records
                                    ),
                                },
                            )
                            .ok();
                        }

                        if let Some(raw_fp) = &record.fingerprint {
                            if let Ok(fp_bytes) = general_purpose::STANDARD.decode(raw_fp.as_ref())
                            {
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
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "similarity".into(),
                            progress: (i * 100 / total_records) as u64,
                            message: format!("Finding similar audio: {}/{}", i, total_records),
                        },
                    )
                    .ok();

                    let idx = chromaprint_records[i].id;
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

            // Process groups with progress updates - optimized for large databases
            let total_groups = similarity_groups.len();
            let mut groups_processed = 0;
            let max_group_size = 1000; // Limit extremely large groups

            // Use chunks to process groups in batches
            for groups_chunk in similarity_groups.iter().collect::<Vec<_>>().chunks(50) {
                // Process this batch of groups
                for (_, group) in groups_chunk {
                    groups_processed += 1;

                    // Update progress more frequently
                    if groups_processed % 10 == 0 || groups_processed == total_groups {
                        app.emit(
                            "search-sub-status",
                            StatusUpdate {
                                stage: "similarity".into(),
                                progress: 70 + ((groups_processed * 20) / total_groups) as u64,
                                message: format!(
                                    "Processing group {}/{} ({} items)",
                                    groups_processed,
                                    total_groups,
                                    group.len()
                                ),
                            },
                        )
                        .ok();
                    }

                    if group.len() > 1 {
                        // Skip excessively large groups or process them differently
                        if group.len() > max_group_size {
                            println!(
                                "Limiting oversized similarity group: {} items (max: {})",
                                group.len(),
                                max_group_size
                            );

                            // Take a sample of the large group instead
                            let mut group_records: Vec<FileRecord> = group
                                .iter()
                                .take(max_group_size)
                                .filter_map(|(idx, _)| {
                                    all_records.iter().find(|r| r.id == *idx).cloned()
                                })
                                .collect();

                            pref.sort_vec(&mut group_records);

                            for (j, mut record) in group_records.into_iter().enumerate() {
                                processed_ids.insert(record.id);
                                record.algorithm.insert(Algorithm::SimilarAudio);
                                if j > 0 {
                                    record.algorithm.remove(&A::Keep);
                                }
                                processed_records.push(record);
                            }
                        } else {
                            // Process normal-sized group
                            let mut group_records: Vec<FileRecord> = group
                                .iter()
                                .filter_map(|(idx, _)| {
                                    all_records.iter().find(|r| r.id == *idx).cloned()
                                })
                                .collect();

                            pref.sort_vec(&mut group_records);

                            for (j, mut record) in group_records.into_iter().enumerate() {
                                processed_ids.insert(record.id);
                                record.algorithm.insert(Algorithm::SimilarAudio);
                                if j > 0 {
                                    record.algorithm.remove(&A::Keep);
                                }
                                processed_records.push(record);
                            }
                        }
                    }
                }
            }
        }

        // STEP 4: Add any records not in a group
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
                message: "Analysis complete: combined fingerprint and hash processing".into(),
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
        self.gather_fingerprints(pref, app).await?;

        if pref.exact_waveform {
            self.exact_match(pref, app).await?;
        } else {
            self.similar_match(pref, app).await?;
        }

        Ok(())
    }

    async fn gather_fingerprints(
        &mut self,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), String> {
        let mut batch_size: usize = 1000;
        let total_records = self.records.len();
        if batch_size > total_records {
            batch_size = total_records;
        }
        let completed = AtomicUsize::new(0);
        const STORE_MIN_INTERVAL: usize = 200;

        let pool = self.get_pool().await;

        let mut record_ids_to_store: Vec<(usize, String)> = Vec::with_capacity(batch_size);

        for chunk in self.records.chunks_mut(batch_size) {
            if self.abort.load(Ordering::SeqCst) {
                println!("Aborting fingerprint scan - early exit");
                return Err("Aborted".to_string());
            }
            let local_ids: Vec<(usize, String)> = chunk
                .par_iter_mut()
                .filter_map(|record| {
                    let path = PathBuf::from(record.get_filepath());
                    let new_completed = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    if !path.exists()
                        || !path.is_file()
                        || record.fingerprint.is_some()
                        || self.abort.load(Ordering::SeqCst)
                    {
                        return None;
                    }
                    app.emit(
                        "search-status",
                        StatusUpdate {
                            stage: "fingerprinting".into(),
                            progress: (new_completed * 100 / total_records) as u64,
                            message: format!(
                                "Generating Audio Fingerprints: ({}/{}) ",
                                new_completed, total_records
                            ),
                        },
                    )
                    .ok();
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "fingerprinting".into(),
                            progress: ((new_completed % batch_size) * 100 / batch_size) as u64,
                            message: format!("{} ", record.get_filename()),
                        },
                    )
                    .ok();

                    let fingerprint_result =
                        audio::get_chromaprint_fingerprint(record.get_filepath());

                    let fingerprint = fingerprint_result.unwrap_or("FAILED".to_string());

                    record.fingerprint = Some(Arc::from(fingerprint.as_str()));

                    Some((record.id, fingerprint))
                })
                .collect();

            record_ids_to_store.extend(local_ids);

            if pref.store_waveforms && record_ids_to_store.len() >= STORE_MIN_INTERVAL {
                // Store fingerprints in batches to avoid memory issues
                if let Some(pool) = &pool {
                    store_fingerprints_batch_optimized(pool, &record_ids_to_store, app).await;
                }
                record_ids_to_store.clear(); // Clear after storing
            }
        }

        if pref.store_waveforms {
            // Store fingerprints in batches to avoid memory issues
            if let Some(pool) = &pool {
                store_fingerprints_batch_optimized(pool, &record_ids_to_store, app).await;
            }
            record_ids_to_store.clear(); // Clear after storing
        }

        Ok(())
    }
}

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

    app.emit(
        "search-sub-status",
        StatusUpdate {
            stage: "db-storage".into(),
            progress: 0,
            message: format!("Storing {} fingerprints in database...", fingerprints.len()),
        },
    )
    .ok();

    match pool.begin().await {
        Ok(mut tx) => {
            let _ = sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&mut *tx)
                .await;
            let _ = sqlx::query("PRAGMA synchronous = NORMAL")
                .execute(&mut *tx)
                .await;

            let total = fingerprints.len();
            let mut success_count = 0;
            let mut error_count = 0;

            for (i, (id, fingerprint)) in fingerprints.iter().enumerate() {
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

            app.emit(
                "search-sub-status",
                StatusUpdate {
                    stage: "db-storage".into(),
                    progress: 99,
                    message: "Committing all changes to database...".to_string(),
                },
            )
            .ok();

            match tx.commit().await {
                Ok(_) => {
                    println!(
                        "Transaction committed successfully: {} fingerprints updated, {} errors",
                        success_count, error_count
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

#[cfg(target_arch = "x86_64")]
fn calculate_similarity_simd(fp1: &[u32], fp2: &[u32]) -> f64 {
    use std::arch::x86_64::*;

    let min_len = fp1.len().min(fp2.len());
    if min_len == 0 {
        return 0.0;
    }

    if is_x86_feature_detected!("sse2") {
        unsafe {
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

            for i in (chunks_processed * chunk_size)..min_len {
                let xor_result = fp1[i] ^ fp2[i];
                matching_bits += 32 - xor_result.count_ones() as usize;
            }

            return matching_bits as f64 / (min_len * 32) as f64;
        }
    }

    calculate_similarity(fp1, fp2)
}

#[cfg(target_arch = "aarch64")]
fn calculate_similarity_simd(fp1: &[u32], fp2: &[u32]) -> f64 {
    use std::arch::aarch64::*;

    let min_len = fp1.len().min(fp2.len());
    if min_len == 0 {
        return 0.0;
    }

    unsafe {
        let chunk_size = 4;
        let mut matching_bits = 0;
        let mut chunks_processed = 0;

        for i in (0..min_len - (min_len % chunk_size)).step_by(chunk_size) {
            let a = vld1q_u32(fp1[i..].as_ptr());
            let b = vld1q_u32(fp2[i..].as_ptr());

            let xor_result = veorq_u32(a, b);

            let mut cnt = vcntq_u8(vreinterpretq_u8_u32(xor_result));
            let sum = vaddv_u8(vget_low_u8(cnt)) + vaddv_u8(vget_high_u8(cnt));

            matching_bits += 128 - sum as usize;
            chunks_processed += 1;
        }

        for i in (chunks_processed * chunk_size)..min_len {
            let xor_result = fp1[i] ^ fp2[i];
            matching_bits += 32 - xor_result.count_ones() as usize;
        }

        return matching_bits as f64 / (min_len * 32) as f64;
    }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn calculate_similarity_simd(fp1: &[u32], fp2: &[u32]) -> f64 {
    calculate_similarity(fp1, fp2)
}

fn calculate_similarity(fp1: &[u32], fp2: &[u32]) -> f64 {
    let min_len = fp1.len().min(fp2.len());
    if min_len == 0 {
        return 0.0;
    }

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

// Function to decode a base64 fingerprint into a vector of u32
fn decode_fingerprint(raw_fp: &str) -> Vec<u32> {
    if let Ok(fp_bytes) = general_purpose::STANDARD.decode(raw_fp) {
        let mut fp = Vec::with_capacity(fp_bytes.len() / 4);
        for chunk in fp_bytes.chunks_exact(4) {
            if chunk.len() == 4 {
                let mut array = [0u8; 4];
                array.copy_from_slice(chunk);
                fp.push(u32::from_le_bytes(array));
            }
        }
        fp
    } else {
        Vec::new()
    }
}

// Function to create a hash from a fingerprint feature (4 blocks)
fn fingerprint_feature_hash(feature: &[u32]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for &block in feature {
        std::hash::Hash::hash(&block, &mut hasher);
    }
    std::hash::Hasher::finish(&hasher)
}

async fn verify_segment_matches(
    &self,
    candidates: HashMap<(usize, usize), usize>,
    threshold: f64,
    app: &AppHandle,
) -> Result<HashMap<usize, Vec<(usize, f64)>>, String> {
    app.emit(
        "search-sub-status",
        StatusUpdate {
            stage: "segment".into(),
            progress: 50,
            message: format!(
                "Verifying {} potential segment matches...",
                candidates.len()
            ),
        },
    )
    .ok();

    // Group that stores verified segments: container_id -> [(segment_id, similarity)]
    let mut segment_groups: HashMap<usize, Vec<(usize, f64)>> = HashMap::new();
    let mut processed = 0;
    let total_candidates = candidates.len();

    // Cache decoded fingerprints to avoid repeated decoding
    let mut fingerprint_cache: HashMap<usize, Vec<u32>> = HashMap::with_capacity(1000);

    // Process verification in batches
    for candidate_batch in candidates.into_iter().collect::<Vec<_>>().chunks(100) {
        let verified_batch = tokio::task::spawn_blocking({
            let fingerprint_cache = &mut fingerprint_cache; // Borrow
            let records = &self.records; // Borrow

            move || {
                let mut batch_results: Vec<(usize, usize, f64)> = Vec::new();

                for &((file1, file2), match_count) in candidate_batch {
                    // Skip if the match count is too low (already filtered, but double-check)
                    if match_count < 15 {
                        continue;
                    }

                    // Get or decode the fingerprints
                    let fp1 = get_or_decode_fingerprint(file1, records, fingerprint_cache);
                    let fp2 = get_or_decode_fingerprint(file2, records, fingerprint_cache);

                    if fp1.is_empty() || fp2.is_empty() {
                        continue;
                    }

                    // Determine which file might be a segment of the other
                    // Generally, the shorter one is a segment of the longer one
                    let (container_id, segment_id, similarity) = if fp1.len() > fp2.len() {
                        // Check if fp2 is a segment of fp1
                        let sim = detect_segment(&fp2, &fp1, threshold)
                            .map(|m| m.similarity)
                            .unwrap_or(0.0);
                        (file1, file2, sim)
                    } else {
                        // Check if fp1 is a segment of fp2
                        let sim = detect_segment(&fp1, &fp2, threshold)
                            .map(|m| m.similarity)
                            .unwrap_or(0.0);
                        (file2, file1, sim)
                    };

                    // If similarity is above threshold, add to results
                    if similarity >= threshold {
                        batch_results.push((container_id, segment_id, similarity));
                    }
                }

                batch_results
            }
        })
        .await
        .map_err(|e| format!("Verification failed: {:?}", e))?;

        // Add verified segments to the groups
        for (container_id, segment_id, similarity) in verified_batch {
            segment_groups
                .entry(container_id)
                .or_default()
                .push((segment_id, similarity));
        }

        processed += candidate_batch.len();

        // Update progress
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "segment".into(),
                progress: 50 + ((processed * 20) / total_candidates) as u64,
                message: format!("Verifying matches: {}/{}", processed, total_candidates),
            },
        )
        .ok();

        // Clear cache if it gets too large
        if fingerprint_cache.len() > 5000 {
            fingerprint_cache.clear();
        }
    }

    // Sort segments by similarity within each container
    for segments in segment_groups.values_mut() {
        segments.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    }

    println!("Found {} containers with segments", segment_groups.len());
    Ok(segment_groups)
}

// Helper function for caching fingerprint decoding
fn get_or_decode_fingerprint(
    id: usize,
    records: &[FileRecord],
    cache: &mut HashMap<usize, Vec<u32>>,
) -> Vec<u32> {
    if let Some(fp) = cache.get(&id) {
        return fp.clone();
    }

    if let Some(record) = records.iter().find(|r| r.id == id) {
        if let Some(raw_fp) = &record.fingerprint {
            if raw_fp != "FAILED" && !raw_fp.starts_with("PCM:") {
                let decoded = decode_fingerprint(raw_fp);
                cache.insert(id, decoded.clone());
                return decoded;
            }
        }
    }

    Vec::new()
}

/// Detects if a shorter audio segment appears within a longer one
fn detect_segment(short_fp: &[u32], long_fp: &[u32], threshold: f64) -> Option<SegmentMatch> {
    if short_fp.is_empty() || long_fp.is_empty() || short_fp.len() > long_fp.len() {
        return None;
    }

    // Skip if the short fingerprint is too short (causes too many false positives)
    if short_fp.len() < 20 {
        return None;
    }

    let mut best_match = SegmentMatch {
        position: 0,
        similarity: 0.0,
    };

    // For very large fingerprints, use sparse sampling to improve performance
    let positions_to_check = if long_fp.len() > 1000 {
        // For very long fingerprints, sample at intervals
        // Skip exponentially more positions as the fingerprint gets longer
        let step = ((long_fp.len() as f64).sqrt() as usize).max(5);
        (0..=long_fp.len() - short_fp.len())
            .step_by(step)
            .collect::<Vec<_>>()
    } else {
        // For shorter fingerprints, check every position or use a small step
        let step = if long_fp.len() > 500 { 4 } else { 1 };
        (0..=long_fp.len() - short_fp.len()).step_by(step).collect()
    };

    for start_pos in positions_to_check {
        let mut matching_bits = 0;
        let total_bits = short_fp.len() * 32;

        // Use SIMD for faster comparison when available
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("sse2") {
                unsafe {
                    use std::arch::x86_64::*;
                    let chunk_size = 4;
                    let mut chunks_processed = 0;

                    for i in (0..short_fp.len() - (short_fp.len() % chunk_size)).step_by(chunk_size)
                    {
                        let a = _mm_loadu_si128(short_fp[i..].as_ptr() as *const __m128i);
                        let b =
                            _mm_loadu_si128(long_fp[start_pos + i..].as_ptr() as *const __m128i);
                        let xor = _mm_xor_si128(a, b);

                        let count = (_mm_extract_epi64(xor, 0) as u64).count_ones()
                            + (_mm_extract_epi64(xor, 1) as u64).count_ones();

                        matching_bits += 128 - count as usize;
                        chunks_processed += 1;
                    }

                    // Process remaining elements
                    for i in (chunks_processed * chunk_size)..short_fp.len() {
                        let xor_result = short_fp[i] ^ long_fp[start_pos + i];
                        matching_bits += 32 - xor_result.count_ones() as usize;
                    }
                }
            } else {
                for i in 0..short_fp.len() {
                    let xor_result = short_fp[i] ^ long_fp[start_pos + i];
                    matching_bits += 32 - xor_result.count_ones() as usize;
                }
            }
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            for i in 0..short_fp.len() {
                let xor_result = short_fp[i] ^ long_fp[start_pos + i];
                matching_bits += 32 - xor_result.count_ones() as usize;
            }
        }

        let similarity = matching_bits as f64 / total_bits as f64;

        // Keep track of best match
        if similarity > best_match.similarity {
            best_match.similarity = similarity;
            best_match.position = start_pos;
        }

        // Early exit if we found an excellent match
        if similarity >= 0.92 {
            break;
        }
    }

    // Only return matches that exceed the threshold
    if best_match.similarity >= threshold {
        Some(best_match)
    } else {
        None
    }
}

async fn mark_segment_groups(
    &mut self,
    segment_groups: HashMap<usize, Vec<(usize, f64)>>,
    app: &AppHandle,
) -> Result<(), String> {
    app.emit(
        "search-sub-status",
        StatusUpdate {
            stage: "segment".into(),
            progress: 80,
            message: "Marking segment relationships...",
        },
    )
    .ok();

    // Keep track of all records
    let all_records = self.records.clone();

    // Create a BitSet to track which records have been processed
    let mut processed_ids = BitSet::with_capacity(self.records.len());
    let mut processed_records = Vec::with_capacity(self.records.len());

    // Process all segment groups
    let total_groups = segment_groups.len();
    let mut processed_groups = 0;

    for (container_id, segments) in segment_groups {
        // Find the container record
        if let Some(container_record) = all_records.iter().find(|r| r.id == container_id) {
            // Clone and mark the container record
            let mut container_record = container_record.clone();
            container_record.algorithm.insert(Algorithm::Segments);
            processed_ids.insert(container_record.id);
            processed_records.push(container_record);

            // Process all segments for this container
            for (segment_id, _similarity) in segments {
                if let Some(segment_record) = all_records.iter().find(|r| r.id == segment_id) {
                    let mut segment_record = segment_record.clone();
                    segment_record.algorithm.insert(Algorithm::Segments);
                    segment_record.algorithm.remove(&A::Keep); // Segments are marked as duplicates
                    processed_ids.insert(segment_record.id);
                    processed_records.push(segment_record);
                }
            }
        }

        processed_groups += 1;

        // Update progress every 10 groups or for the last one
        if processed_groups % 10 == 0 || processed_groups == total_groups {
            app.emit(
                "search-sub-status",
                StatusUpdate {
                    stage: "segment".into(),
                    progress: 80 + ((processed_groups * 15) / total_groups) as u64,
                    message: format!("Marking groups: {}/{}", processed_groups, total_groups),
                },
            )
            .ok();
        }
    }

    // Add any records not in a segment group
    for record in all_records {
        if !processed_ids.contains(record.id) {
            processed_records.push(record);
        }
    }

    self.records = processed_records;

    // Final completion update
    app.emit(
        "search-sub-status",
        StatusUpdate {
            stage: "segment".into(),
            progress: 100,
            message: "Segment analysis complete".into(),
        },
    )
    .ok();

    Ok(())
}
