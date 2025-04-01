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
    pub async fn process_large_collection(
        &mut self,
        app: &AppHandle,
        pref: &Preferences,
    ) -> Result<(), String> {
        let mut matcher = AudioMatcher::new(pref.waveform_search_type);
        // PHASE 1: Add all files to the database
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "fingerprint".into(),
                progress: 0,
                message: "Building fingerprint database...".into(),
            },
        )
        .ok();

        for (i, record) in self.records.iter().enumerate() {
            let path = &record.get_filepath();
            if i % 100 == 0 {
                // Update progress
                app.emit(
                    "search-sub-status",
                    StatusUpdate {
                        stage: "fingerprint".into(),
                        progress: ((i * 50) / self.records.len()) as u64,
                        message: format!("Adding fingerprints: {}/{}", i, self.records.len()),
                    },
                )
                .ok();
            }

            // Add file to fingerprint database
            match matcher.add_file(record.id, path) {
                Ok(_) => {}
                Err(e) => println!("Error fingerprinting {}: {}", path, e),
            }
        }

        // PHASE 2: Compare each file to find matches
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "matching".into(),
                progress: 50,
                message: "Finding duplicate matches...".into(),
            },
        )
        .ok();

        // Track which files we've already processed to avoid redundant comparisons
        let mut processed_ids = HashSet::new();
        let mut match_groups = Vec::new();

        for (i, record) in self.records.iter().enumerate() {
            if i % 100 == 0 {
                // Update progress
                app.emit(
                    "search-sub-status",
                    StatusUpdate {
                        stage: "matching".into(),
                        progress: (50 + (i * 50) / self.records.len()) as u64,
                        message: format!("Finding matches: {}/{}", i, self.records.len()),
                    },
                )
                .ok();
            }

            // Skip if we've already marked this file as part of a group
            if processed_ids.contains(&record.id) {
                continue;
            }

            let path = record.get_filepath();
            // Find matches for this file
            match matcher.find_matches(path) {
                Ok(matches) => {
                    if !matches.is_empty() {
                        // Found matches - create a group
                        let mut group = vec![record.id];

                        for matched in matches {
                            match matched {
                                MatchResult::Exact(id)
                                | MatchResult::Similar(id, _)
                                | MatchResult::Subset(id, _, _) => {
                                    if id != record.id {
                                        group.push(id);
                                        processed_ids.insert(id);
                                    }
                                }
                            }
                        }

                        if group.len() > 1 {
                            match_groups.push(group);
                        }
                    }
                }
                Err(e) => println!("Error matching {}: {}", path, e),
            }

            // Mark this file as processed
            processed_ids.insert(record.id);
        }

        // PHASE 3: Apply results to your records
        // (Mark duplicates in your database)

        Ok(())
    }

    pub async fn shazam_waveform_search(&mut self, app: &AppHandle) -> Result<(), String> {
        let mut matcher = AudioMatcher::new(MatchType::Similar); // Set to detect segments

        // STEP 1: Sort records by descending duration
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "prepare".into(),
                progress: 0,
                message: "Sorting files by duration...".into(),
            },
        )
        .ok();

        // Extract records with valid durations
        let mut records_with_duration: Vec<(&FileRecord, f64)> = self
            .records
            .iter()
            .filter_map(|record| {
                record
                    .get_duration()
                    .ok()
                    .map(|duration| (record, duration))
            })
            .collect();

        // Sort by duration (longest first)
        records_with_duration
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // STEP 2: Process files in descending duration order
        let total = records_with_duration.len();
        let mut processed = 0;
        let mut match_groups = Vec::new();
        let mut already_matched = HashSet::new();

        for (record, _duration) in records_with_duration {
            if processed % 100 == 0 || processed == total - 1 {
                app.emit(
                    "search-sub-status",
                    StatusUpdate {
                        stage: "matching".into(),
                        progress: (processed * 100 / total) as u64,
                        message: format!("Processing files by duration: {}/{}", processed, total),
                    },
                )
                .ok();
            }

            // Skip files we've already marked as duplicates/segments
            if already_matched.contains(&record.id) {
                processed += 1;
                continue;
            }

            let path = record.get_filepath();
            // First check if this file matches anything already in the database
            match matcher.find_matches(path) {
                Ok(matches) => {
                    if !matches.is_empty() {
                        // This file matches something in the database
                        // We only need to handle subset detection, as this file must be
                        // shorter than all previously processed files
                        let mut is_subset = false;

                        for matched in &matches {
                            if let MatchResult::Subset(parent_id, _, score) = matched {
                                if score >= &0.5 {
                                    // Good confidence in subset match
                                    // Mark as a subset
                                    already_matched.insert(record.id);
                                    is_subset = true;

                                    // Update record to mark it as a subset
                                    // (Your actual code to mark duplicates would go here)
                                    println!("File {} is a segment of {}", record.id, parent_id);
                                    break;
                                }
                            }
                        }

                        // If not a subset, but similar/exact match
                        if !is_subset {
                            // Create new match group (for similar/exact matches)
                            let mut group = vec![record.id];

                            for matched in matches {
                                match matched {
                                    MatchResult::Exact(id) | MatchResult::Similar(id, _) => {
                                        if id != record.id {
                                            group.push(id);
                                            already_matched.insert(id);
                                        }
                                    }
                                    _ => {} // Already handled subsets above
                                }
                            }

                            if group.len() > 1 {
                                match_groups.push(group);
                            }
                        }
                    }
                }
                Err(e) => println!("Error matching {}: {}", path, e),
            }

            // Only add to database if it's not a subset of something else
            if !already_matched.contains(&record.id) {
                match matcher.add_file(record.id, path) {
                    Ok(_) => {}
                    Err(e) => println!("Error fingerprinting {}: {}", path, e),
                }
            }

            processed += 1;
        }

        // Process results and update database

        Ok(())
    }

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

    pub async fn wave_search_shazam(
        &mut self,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), String> {
        println!("Starting Shazam Search");

        let mut matcher = AudioMatcher::new(MatchType::Exact);

        for (i, record) in self.records.iter().enumerate() {
            let _ = matcher.add_file(i, record.get_filepath());
        }

        self.gather_fingerprints(pref, app).await?;

        match pref.waveform_search_type {
            MatchType::Subset => {
                self.subset_match(pref, app).await?;
            }
            MatchType::Exact => {
                self.exact_match(pref, app).await?;
            }
            MatchType::Similar => {
                self.similar_match(pref, app).await?;
            }
        }

        Ok(())
    }

    pub async fn wave_search_chromaprint(
        &mut self,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), String> {
        println!("Starting Waveform Search");
        self.gather_fingerprints(pref, app).await?;

        match pref.waveform_search_type {
            MatchType::Subset => {
                self.subset_match(pref, app).await?;
            }
            MatchType::Exact => {
                self.exact_match(pref, app).await?;
            }
            MatchType::Similar => {
                self.similar_match(pref, app).await?;
            }
        }

        // if pref.exact_waveform {
        //     self.exact_match(pref, app).await?;
        // } else {
        //     self.similar_match(pref, app).await?;
        // }

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
                        audio::chromaprint::get_chromaprint_fingerprint(record.get_filepath());

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

    async fn subset_match(&mut self, pref: &Preferences, app: &AppHandle) -> Result<(), String> {
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "subset".into(),
                progress: 0,
                message: "Starting audio subset detection...".into(),
            },
        )
        .ok();

        // Filter records to only those with valid Chromaprint fingerprints
        let valid_records: Vec<&FileRecord> = self
            .records
            .par_iter()
            .filter(|record| {
                record.fingerprint.as_ref().is_some_and(|fp| {
                    !fp.is_empty() && &**fp != "FAILED" && !fp.starts_with("PCM:")
                })
            })
            .collect();

        let total_records = valid_records.len();
        println!(
            "Found {} records with valid fingerprints for subset analysis",
            total_records
        );

        if total_records == 0 {
            return Ok(());
        }

        // Step 1: Sort records by duration (longer files first)
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "subset".into(),
                progress: 5,
                message: "Preparing files for subset analysis...".into(),
            },
        )
        .ok();

        // Create a sorted list of records by duration
        let mut records_by_duration = valid_records.clone();
        records_by_duration.sort_by(|a, b| {
            let a_duration = a.duration.parse::<f64>().unwrap_or(0.0);
            let b_duration = b.duration.parse::<f64>().unwrap_or(0.0);
            b_duration
                .partial_cmp(&a_duration)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Step 2: Decode all fingerprints once to avoid repeated work
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "subset".into(),
                progress: 10,
                message: "Decoding audio fingerprints...".into(),
            },
        )
        .ok();

        // Map of record ID to decoded fingerprint
        let decoded_fingerprints: HashMap<usize, Vec<u32>> = valid_records
            .par_iter()
            .filter_map(|record| {
                if let Some(fp) = &record.fingerprint {
                    if let Ok(decoded) = decode_chromaprint(fp) {
                        return Some((record.id, decoded));
                    }
                }
                None
            })
            .collect();

        println!(
            "Successfully decoded {} fingerprints",
            decoded_fingerprints.len()
        );

        // Step 3: Find subset relationships
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "subset".into(),
                progress: 20,
                message: "Finding audio subset relationships...".into(),
            },
        )
        .ok();

        // Track parent-child relationships
        let mut parent_children_map: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut child_parent_map: HashMap<usize, usize> = HashMap::new();

        // For large datasets, process in batches
        let batch_size = 1000;
        let total_batches = records_by_duration.len().div_ceil(batch_size);

        // Use a threshold slightly lower than for similarity matching
        let subset_threshold = (pref.similarity_threshold / 100.0) * 0.9;

        for batch_idx in 0..total_batches {
            let batch_start = batch_idx * batch_size;
            let batch_end = ((batch_idx + 1) * batch_size).min(records_by_duration.len());

            app.emit(
                "search-sub-status",
                StatusUpdate {
                    stage: "subset".into(),
                    progress: 20 + (batch_idx * 60 / total_batches) as u64,
                    message: format!(
                        "Processing batch {}/{} (files {}-{})",
                        batch_idx + 1,
                        total_batches,
                        batch_start + 1,
                        batch_end
                    ),
                },
            )
            .ok();

            // Process this batch of potential parents
            for i in batch_start..batch_end {
                let parent_record = records_by_duration[i];
                let parent_id = parent_record.id;

                // Skip if already marked as a child of another file
                if child_parent_map.contains_key(&parent_id) {
                    continue;
                }

                // Get parent fingerprint
                let parent_fp = match decoded_fingerprints.get(&parent_id) {
                    Some(fp) => fp,
                    None => continue,
                };

                let parent_duration = parent_record.duration.parse::<f64>().unwrap_or(0.0);

                // Find potential children (shorter duration files)
                for child_record in &records_by_duration[(i + 1)..] {
                    let child_id = child_record.id;

                    // Skip if already identified as a child or if it's the same file
                    if child_parent_map.contains_key(&child_id) || child_id == parent_id {
                        continue;
                    }

                    let child_duration = child_record.duration.parse::<f64>().unwrap_or(0.0);

                    if child_duration >= parent_duration {
                        continue;
                    }

                    // Get child fingerprint
                    let child_fp = match decoded_fingerprints.get(&child_id) {
                        Some(fp) => fp,
                        None => continue,
                    };

                    // Check if child is a subset of parent using sliding window approach
                    if is_fingerprint_subset(child_fp, parent_fp, subset_threshold) {
                        // Add relationship
                        parent_children_map
                            .entry(parent_id)
                            .or_default()
                            .push(child_id);
                        child_parent_map.insert(child_id, parent_id);
                    }
                }

                // Report progress periodically
                if (i - batch_start) % 50 == 0 {
                    let batch_progress = (i - batch_start) * 100 / (batch_end - batch_start);
                    app.emit(
                        "search-sub-status",
                        StatusUpdate {
                            stage: "subset".into(),
                            progress: 20
                                + ((batch_idx * 100 + batch_progress) * 60 / (total_batches * 100))
                                    as u64,
                            message: format!(
                                "Batch {}/{}: {} subset relationships found",
                                batch_idx + 1,
                                total_batches,
                                parent_children_map.values().map(|v| v.len()).sum::<usize>()
                            ),
                        },
                    )
                    .ok();
                }
            }
        }

        // Step 4: Apply algorithm markings
        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "subset".into(),
                progress: 85,
                message: "Applying algorithm markings to records...".into(),
            },
        )
        .ok();

        let total_parents = parent_children_map.len();
        let total_children = child_parent_map.len();

        println!(
            "Found {} parent files containing {} child subsets",
            total_parents, total_children
        );

        // Update records with subset relationships
        self.records.par_iter_mut().for_each(|record| {
            let id = record.id;

            if parent_children_map.contains_key(&id) {
                // This is a parent file with subsets
                record.algorithm.insert(Algorithm::Waveforms);
                // Parents keep the Keep algorithm
            } else if let Some(_parent_id) = child_parent_map.get(&id) {
                // This is a child/subset file
                record.algorithm.insert(Algorithm::Waveforms);
                record.algorithm.remove(&Algorithm::Keep);
            }
        });

        app.emit(
            "search-sub-status",
            StatusUpdate {
                stage: "subset".into(),
                progress: 100,
                message: format!(
                    "Subset detection complete: {} parent files, {} subset files",
                    total_parents, total_children
                ),
            },
        )
        .ok();

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

fn is_fingerprint_subset(shorter_fp: &[u32], longer_fp: &[u32], threshold: f64) -> bool {
    if shorter_fp.is_empty() || longer_fp.is_empty() || shorter_fp.len() > longer_fp.len() {
        return false;
    }

    // Apply a dynamic threshold based on fingerprint length
    let adjusted_threshold = if shorter_fp.len() < 30 {
        // For very short fingerprints, use a lower threshold
        threshold * 0.75
    } else if shorter_fp.len() < 60 {
        // For medium-short fingerprints
        threshold * 0.8
    } else {
        // For normal fingerprints
        threshold * 0.85
    };

    // For extremely short fingerprints, use feature-based matching
    if shorter_fp.len() < 15 {
        return feature_based_match(shorter_fp, longer_fp, adjusted_threshold);
    }

    // Use a more efficient window scanning approach
    // For long fingerprints, use larger step sizes
    let step_size = if longer_fp.len() > 1000 {
        // Use larger steps for very long fingerprints
        (longer_fp.len() / 200).max(1)
    } else {
        // Use smaller steps for moderate length fingerprints
        (longer_fp.len() / 400).max(1)
    };

    // Try multiple window sizes to account for tempo/speed differences
    let window_sizes = [
        shorter_fp.len(),
        (shorter_fp.len() as f64 * 1.1) as usize, // 10% longer
        (shorter_fp.len() as f64 * 0.9) as usize, // 10% shorter
        (shorter_fp.len() as f64 * 1.2) as usize, // 20% longer
        (shorter_fp.len() as f64 * 0.8) as usize, // 20% shorter
    ];

    let mut best_similarity = 0.0;

    // Try each window size
    for &window_size in &window_sizes {
        // Skip if window size is too large for the longer fingerprint
        if window_size > longer_fp.len() {
            continue;
        }

        // Scan through the longer fingerprint with the determined step size
        for window_start in (0..=(longer_fp.len() - window_size)).step_by(step_size) {
            let window = &longer_fp[window_start..window_start + window_size];

            // Calculate similarity based on window size
            let similarity = if window_size == shorter_fp.len() {
                // For exact size, use the optimized SIMD function
                calculate_similarity_simd(shorter_fp, window)
            } else {
                // For different sizes, use flexible similarity
                calculate_flexible_similarity(shorter_fp, window)
            };

            best_similarity = f64::max(best_similarity, similarity);

            // Early exit for strong matches
            if best_similarity > 0.9 {
                return true;
            }
        }
    }

    best_similarity >= adjusted_threshold
}

// Add this new function specifically for very short fingerprints
fn feature_based_match(short_fp: &[u32], long_fp: &[u32], threshold: f64) -> bool {
    // Extract distinctive bit patterns from short fingerprint
    let short_patterns: Vec<u32> = short_fp.windows(2).map(|w| w[0] & w[1]).collect();

    // Count how many of these patterns appear in the long fingerprint
    let mut matches = 0;
    for window in long_fp.windows(2) {
        let pattern = window[0] & window[1];
        if short_patterns.contains(&pattern) {
            matches += 1;
        }
    }

    // Calculate match percentage
    let match_ratio = matches as f64 / short_patterns.len() as f64;
    match_ratio >= threshold
}

// Compare fingerprints of different lengths by aligning features
fn calculate_flexible_similarity(fp1: &[u32], fp2: &[u32]) -> f64 {
    // Extract feature sequences from both fingerprints
    let features1 = extract_features(fp1);
    let features2 = extract_features(fp2);

    // Find longest common subsequence of features
    let lcs_length = longest_common_subsequence(&features1, &features2);

    // Return ratio of matching features
    lcs_length as f64 / features1.len().min(features2.len()) as f64
}

/// Extract distinctive features from a fingerprint
/// Returns a vector of "features" which are small hashes derived from fingerprint blocks
fn extract_features(fp: &[u32]) -> Vec<u16> {
    if fp.len() < 2 {
        return Vec::new();
    }

    // Use sliding window to extract overlapping features
    let mut features = Vec::with_capacity(fp.len() - 1);

    for window in fp.windows(2) {
        // Create a compact feature from consecutive blocks
        // We use XOR and AND operations to create distinctive patterns
        let feature = ((window[0] & 0xFFFF) ^ (window[1] >> 16)) as u16;
        features.push(feature);
    }

    // For longer fingerprints, also add some wider-range features
    if fp.len() >= 4 {
        for window in fp.windows(4) {
            let feature = ((window[0] & 0xFF) | ((window[3] & 0xFF) << 8)) as u16;
            features.push(feature);
        }
    }

    features
}

/// Find the longest common subsequence between two feature sequences
/// This is crucial for determining if one fingerprint is contained within another
fn longest_common_subsequence(seq1: &[u16], seq2: &[u16]) -> usize {
    if seq1.is_empty() || seq2.is_empty() {
        return 0;
    }

    // For very large sequences, use a faster approximate method
    if seq1.len() * seq2.len() > 1_000_000 {
        return approximate_lcs(seq1, seq2);
    }

    // Classic dynamic programming approach for LCS
    let mut dp = vec![vec![0; seq2.len() + 1]; seq1.len() + 1];

    for i in 1..=seq1.len() {
        for j in 1..=seq2.len() {
            if seq1[i - 1] == seq2[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    dp[seq1.len()][seq2.len()]
}

/// A more memory-efficient approximation of LCS for very large sequences
fn approximate_lcs(seq1: &[u16], seq2: &[u16]) -> usize {
    // Use feature hashing to avoid O(n²) memory usage
    let shorter = if seq1.len() <= seq2.len() { seq1 } else { seq2 };
    let longer = if seq1.len() <= seq2.len() { seq2 } else { seq1 };

    // Create a hash set of features from the shorter sequence
    let feature_set: HashSet<u16> = shorter.iter().copied().collect();

    // Count matches in the longer sequence
    let matches = longer.iter().filter(|&x| feature_set.contains(x)).count();

    // Normalize to account for random matches
    (matches as f64 * 0.8) as usize
}
// Helper function to decode a Chromaprint fingerprint from base64 to u32 vector
fn decode_chromaprint(raw_fp: &str) -> Result<Vec<u32>, &'static str> {
    if raw_fp.starts_with("PCM:") {
        return Err("Not a Chromaprint fingerprint");
    }

    match general_purpose::STANDARD.decode(raw_fp) {
        Ok(fp_bytes) => {
            let mut fp = Vec::with_capacity(fp_bytes.len() / 4);
            for chunk in fp_bytes.chunks_exact(4) {
                if chunk.len() == 4 {
                    let mut array = [0u8; 4];
                    array.copy_from_slice(chunk);
                    fp.push(u32::from_le_bytes(array));
                }
            }
            Ok(fp)
        }
        Err(_) => Err("Failed to decode fingerprint"),
    }
}
