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
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::{AppHandle, Emitter};

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
            .iter()
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
                    // Progress reporting code...
                    // let update_interval = (total_records.max(20) / 100).max(1);
                    // if i % update_interval == 0 {
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "similarity".into(),
                            // progress: 40 + ((i * 30) / total_records) as u64,
                            progress: (i * 100 / total_records) as u64,
                            message: format!("Finding similar audio: {}/{}", i, total_records),
                        },
                    )
                    .ok();
                    // }

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
            let total_groups = similarity_groups.len();
            for (i, (_, group)) in similarity_groups.iter().enumerate() {
                // Update every 5% of groups processed
                let update_interval = (total_groups.max(20) / 100).max(1);
                if i % update_interval == 0 {
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
                    let mut group_records: Vec<FileRecord> = group
                        .iter()
                        .filter_map(|(idx, _)| all_records.iter().find(|r| r.id == *idx).cloned())
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

        // Initial status message so user knows process has started
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "fingerprinting".into(),
                progress: 0,
                message: "Starting audio fingerprint analysis...".into(),
            },
        )
        .ok();

        // OPTIMIZATIONS
        let mut batch_size: usize = 1000;
        const STORAGE_INTERVAL: usize = 200;
        const PROGRESS_INTERVAL: usize = RECORD_DIVISOR * 5;

        // Pre-scan status update
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "fingerprinting".into(),
                progress: 2,
                message: format!("Pre-scanning {} audio files...", self.records.len()),
            },
        )
        .ok();

        // 1. PRE-FILTERING OPTIMIZATION: Use indexed access for faster filtering
        let mut records_needing_fingerprints = BitSet::with_capacity(self.records.len());
        let update_frequency = std::cmp::max(self.records.len() / 100, 1);

        for (idx, record) in self.records.iter().enumerate() {
            if idx % 100 == 0 && *self.abort.read().await {
                return Err("Aborted during pre-scan".to_string());
            }

            if idx % update_frequency == 0 || idx == self.records.len() - 1 {
                app.emit(
                    "search-sub-status",
                    StatusUpdate {
                        stage: "fingerprinting".into(),
                        progress: (idx * 100 / self.records.len()) as u64,
                        message: format!("Scanning files: {}/{}", idx, self.records.len()),
                    },
                )
                .ok();
            }

            let path = PathBuf::from(record.get_filepath());
            if !path.exists() || !path.is_file() {
                continue;
            }

            if record.fingerprint.is_none() {
                records_needing_fingerprints.insert(idx);
            }
        }

        if records_needing_fingerprints.is_empty() {
            println!("No fingerprints needed - all files already processed");
            app.emit(
                "search-sub-status",
                StatusUpdate {
                    stage: "fingerprinting".into(),
                    progress: 100,
                    message: "All files already have fingerprints - skipping analysis".into(),
                },
            )
            .ok();
            return Ok(());
        }

        let total_to_process = records_needing_fingerprints.len();

        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "fingerprinting".into(),
                progress: 100,
                message: format!("Found {} files requiring fingerprinting", total_to_process),
            },
        )
        .ok();

        // Initial status update
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "fingerprinting".into(),
                progress: 0,
                message: format!("Preparing to process {} audio files...", total_to_process),
            },
        )
        .ok();

        println!("Found {} files requiring fingerprinting", total_to_process);
        println!("{:#?}", records_needing_fingerprints);

        let start_from = 0;

        let mut fingerprints_to_store = Vec::with_capacity(STORAGE_INTERVAL);
        let mut processed = start_from;

        let db_pool = if pref.store_waveforms {
            match self.get_pool().await {
                Some(pool) => Some(pool),
                None => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    self.get_pool().await
                }
            }
        } else {
            None
        };

        let cpu_count = num_cpus::get();
        let thread_count = std::cmp::min(cpu_count, 8);
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(thread_count)
            .build()
            .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().build().unwrap());

        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "fingerprinting".into(),
                progress: 0,
                message: format!(
                    "Starting fingerprint analysis with {} threads",
                    thread_count
                ),
            },
        )
        .ok();

        println!("Processing with {} threads", thread_count);

        if total_to_process < batch_size {
            batch_size = total_to_process;
        } else {
            app.emit(
                "search-status",
                StatusUpdate {
                    stage: "fingerprinting".into(),
                    progress: (processed * 100 / total_to_process) as u64,
                    message: format!(
                        "Analyzing audio content for waveform analysis: ({}/{}) ",
                        processed, total_to_process
                    ),
                },
            )
            .ok();
        };

        // Main processing loop with simplified abort checking
        for chunk_idx in (start_from..total_to_process).step_by(batch_size) {
            if *self.abort.read().await {
                // Save any processed fingerprints before aborting
                if !fingerprints_to_store.is_empty() && pref.store_waveforms {
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "saving".into(),
                            progress: 100,
                            message: "Cancelled - saving processed fingerprints...".into(),
                        },
                    )
                    .ok();

                    if let Some(ref pool) = db_pool {
                        store_fingerprints_batch_optimized(pool, &fingerprints_to_store, app).await;
                    }
                }

                return Err("Aborted".to_string());
            }

            let end_idx = std::cmp::min(chunk_idx + batch_size, total_to_process);
            let current_indices: Vec<usize> = (chunk_idx..end_idx)
                .filter(|&idx| records_needing_fingerprints.contains(idx))
                .collect();

            let completed = AtomicUsize::new(0);
            let batch_results: Vec<_> = thread_pool.install(|| {
                current_indices
                    .par_iter()
                    .filter_map(|&idx| {
                        let record = &self.records[idx];

                        if record.algorithm.contains(&Algorithm::InvalidPath) {
                            return None;
                        }

                        if record.fingerprint.is_some() {
                            return None;
                        }

                        let fingerprint =
                            match audio::get_chromaprint_fingerprint(record.get_filepath()) {
                                Some(fp) => {
                                    println!("SUCCESS fingerprint for: {}", record.get_filepath());

                                    let new_completed =
                                        completed.fetch_add(1, Ordering::SeqCst) + 1;
                                    app.emit(
                                        "search-sub-status",
                                        StatusUpdate {
                                            stage: "fingerprinting".into(),
                                            progress: ((new_completed % batch_size) * 100
                                                / batch_size)
                                                as u64,
                                            message: format!("{} ", record.get_filename()),
                                        },
                                    )
                                    .ok();
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
                                    Some(Arc::from("FAILED"))
                                }
                            };

                        Some((idx, fingerprint))
                    })
                    .collect()
            });

            if *self.abort.read().await {
                return Err("Aborted during batch processing".to_string());
            }

            for (idx, fingerprint) in &batch_results {
                let record = &mut self.records[*idx];
                record.fingerprint = fingerprint.clone();
            }

            fingerprints_to_store.extend(batch_results.iter().map(|(idx, fingerprint)| {
                (
                    self.records[*idx].id,
                    fingerprint.clone().unwrap_or_default().to_string(),
                )
            }));

            processed += batch_results.len();
            if pref.store_waveforms
                && !fingerprints_to_store.is_empty()
                && (processed % STORAGE_INTERVAL == 0 || processed == total_to_process)
            {
                if let Some(ref pool) = db_pool {
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

                    store_fingerprints_batch_optimized(pool, &fingerprints_to_store, app).await;
                    fingerprints_to_store.clear();
                }
            }

            app.emit(
                "search-status",
                StatusUpdate {
                    stage: "fingerprinting".into(),
                    progress: (processed * 100 / total_to_process) as u64,
                    message: format!(
                        "Analyzing audio content for waveform analysis: ({}/{}) ",
                        processed, total_to_process
                    ),
                },
            )
            .ok();
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
