use crate::*;
use ::chromaprint::Chromaprint;
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use bit_set::BitSet;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

impl Database {
    pub async fn wave_search_chromaprint(
        &mut self,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), String> {
        println!("Starting Waveform Search");
        self.gather_fingerprints(pref, app).await?;

        match pref.waveform_search_type {
            WaveformMatchType::Subset => {
                self.subset_match(pref, app).await?;
            }
            WaveformMatchType::Exact => {
                self.exact_match(pref, app).await?;
            }
            WaveformMatchType::Similar => {
                self.similar_match(pref, app).await?;
            }
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
}

pub fn get_chromaprint_fingerprint<P: AsRef<Path>>(file_path: P) -> Option<String> {
    let path_str = file_path.as_ref().to_string_lossy().to_string();
    let pcm_data = match convert_to_raw_pcm(&path_str) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to convert audio to PCM: {}", e);
            return None;
        }
    };

    // Convert to i16 samples
    let samples: Vec<i16> = pcm_data
        .chunks(4)
        .filter_map(|chunk| {
            if chunk.len() == 4 {
                let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
                let float = f32::from_le_bytes(bytes);
                Some((float * 32767.0) as i16)
            } else {
                None
            }
        })
        .collect();

    const MIN_SAMPLES: usize = 48000; // 1 second minimum at 48kHz

    // Check if we have enough samples for Chromaprint
    if samples.len() >= MIN_SAMPLES {
        // Try Chromaprint fingerprinting
        let mut c = Chromaprint::new();
        c.start(48000, 1);
        c.feed(&samples);
        c.finish();

        if let Some(fingerprint) = c.raw_fingerprint() {
            println!(
                "Generated raw fingerprint for: {} size; {}",
                file_path.as_ref().to_string_lossy(),
                fingerprint.len()
            );
            // Convert Vec<i32> to bytes before encoding
            let bytes: Vec<u8> = fingerprint.iter().flat_map(|&x| x.to_le_bytes()).collect();
            let encoded = general_purpose::STANDARD.encode(&bytes);
            if !encoded.is_empty() {
                return Some(encoded);
            }
        }

        eprintln!("Chromaprint failed despite sufficient samples");
    }

    // Fallback to PCM hash if:
    // 1. File is too short for Chromaprint, or
    // 2. Chromaprint failed to generate a fingerprint
    if !samples.is_empty() {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        for sample in &samples {
            hasher.update(sample.to_le_bytes());
        }

        let hash = hasher.finalize();
        println!(
            "Generated PCM hash for: {}",
            file_path.as_ref().to_string_lossy()
        );
        return Some(format!("PCM:{}", general_purpose::STANDARD.encode(hash)));
    }

    eprintln!("Failed to generate any fingerprint for: {}", path_str);
    None
}

fn convert_to_raw_pcm(input_path: &str) -> Result<Vec<u8>> {
    use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType};

    let file = std::fs::File::open(input_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint to help the format registry guess what format the file is
    let mut hint = Hint::new();
    if let Some(extension) = Path::new(input_path).extension() {
        if let Some(ext_str) = extension.to_str() {
            hint.with_extension(ext_str);
        }
    }

    // Use the default options for format and metadata
    let format_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };
    let metadata_opts = MetadataOptions::default();

    // Probe the media source to determine its format
    let mut probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .context("Failed to probe media format")?;

    // Get the default track
    let track = probed
        .format
        .default_track()
        .ok_or_else(|| anyhow::anyhow!("No default track found"))?;

    // Create a decoder for the track
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create decoder")?;

    // Store the decoded PCM data
    let mut pcm_data = Vec::with_capacity(1_000_000); // Pre-allocate 1MB

    // Decode the track
    let mut sample_count = 0;
    let target_sample_rate = 48000; // Target sample rate for fingerprinting

    // Initialize resampler storage (only created if needed)
    let mut resampler = None;
    let mut last_spec = None;

    loop {
        // Get the next packet from the format reader
        let packet = match probed.format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::ResetRequired) => {
                // Reset the decoder when required
                decoder.reset();
                continue;
            }
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                // End of file reached
                break;
            }
            Err(e) => {
                // Some other error occurred
                return Err(anyhow::anyhow!("Error reading packet: {}", e));
            }
        };

        // Decode the packet
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(symphonia::core::errors::Error::IoError(_)) => {
                // Skip decoding errors
                continue;
            }
            Err(e) => {
                eprintln!("Error decoding packet: {}", e);
                continue;
            }
        };

        // Get the decoded audio buffer
        let spec = *decoded.spec();
        last_spec = Some(spec);

        // Create a buffer for the decoded audio
        let mut sample_buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);

        // Copy the decoded audio to the sample buffer
        sample_buffer.copy_interleaved_ref(decoded);
        let samples = sample_buffer.samples();

        // Check if we need to resample
        if spec.rate != target_sample_rate {
            // Create resampler if this is the first packet or if format changed
            if resampler.is_none() {
                println!(
                    "Resampling from {}Hz to {}Hz for {}",
                    spec.rate, target_sample_rate, input_path
                );

                // Calculate frames (samples per channel)
                let frames = samples.len() / spec.channels.count();

                // Create the resampler
                let resampler_result = SincFixedIn::<f32>::new(
                    target_sample_rate as f64 / spec.rate as f64,
                    2.0, // Oversampling factor
                    SincInterpolationParameters {
                        sinc_len: 256,
                        f_cutoff: 0.95,
                        interpolation: SincInterpolationType::Linear,
                        oversampling_factor: 256,
                        window: rubato::WindowFunction::Blackman,
                    },
                    frames,
                    spec.channels.count(),
                );

                match resampler_result {
                    Ok(r) => resampler = Some(r),
                    Err(e) => {
                        eprintln!("Failed to create resampler: {}", e);
                        resampler = None;
                    }
                }
            }

            // Prepare samples for resampling (convert interleaved to per-channel)
            let channels = spec.channels.count();
            let frames = samples.len() / channels;

            // Split interleaved samples into separate channel vectors
            let mut channel_samples = vec![Vec::with_capacity(frames); channels];
            for (i, &sample) in samples.iter().enumerate() {
                channel_samples[i % channels].push(sample);
            }

            // Perform resampling
            if let Some(resampler) = resampler.as_mut() {
                match resampler.process(&channel_samples, None) {
                    Ok(resampled) => {
                        // Calculate how many samples we have after resampling
                        let resampled_frames = resampled[0].len();
                        let _total_resampled_samples = resampled_frames * channels;

                        // Add resampled samples to PCM data (converting back to interleaved)
                        for frame in 0..resampled_frames {
                            for channel_data in resampled.iter() {
                                let sample = channel_data[frame];
                                pcm_data.extend_from_slice(&sample.to_le_bytes());
                                sample_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Resampling error: {}", e);
                        // Fall back to original samples if resampling fails
                        for &sample in samples {
                            pcm_data.extend_from_slice(&sample.to_le_bytes());
                            sample_count += 1;
                        }
                    }
                }
            }
        } else {
            // No resampling needed, use original samples
            for &sample in samples {
                pcm_data.extend_from_slice(&sample.to_le_bytes());
                sample_count += 1;
            }
        }

        // Apply a limit to prevent excessive memory usage (equivalent to 10 minutes at 48kHz)
        if sample_count > 10 * 60 * target_sample_rate {
            break;
        }
    }

    // Print audio format info for debugging
    if let Some(spec) = last_spec {
        println!(
            "Processed audio: {} channels, {}Hz, {} samples ({:.1} seconds)",
            spec.channels.count(),
            spec.rate,
            sample_count,
            sample_count as f32 / target_sample_rate as f32
        );
    }

    Ok(pcm_data)
}

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
