use crate::prelude::*;
use crate::*;
// use ::chromaprint::Chromaprint;
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use bit_set::BitSet;

impl FileRecord {
    pub fn get_chromaprint_fingerprint(&mut self) -> Option<String> {
        let fp = ffcodex_lib::get_fingerprint(&self.get_filepath()).ok();
        if let Some(fingerprint) = fp {
            // println!(
            //     "Generated fingerprint for: {} size; {}\n{}",
            //     self.get_filename(),
            //     fingerprint.len(),
            //     fingerprint
            // );
            if !fingerprint.is_empty() && fingerprint != "FAILED" {
                self.fingerprint = Some(Arc::from(fingerprint.as_str()));
                return Some(fingerprint);
            }
        }
        None
    }

    // pub fn get_chromaprint_fingerprint(&mut self) -> Option<String> {

    //     let pcm_data = match self.get_raw_pcm() {
    //         Ok(data) => data,
    //         Err(e) => {
    //             eprintln!("Failed to convert audio to PCM: {}", e);
    //             return None;
    //         }
    //     };

    //     let samples: Vec<i16> = pcm_data
    //         .chunks(4)
    //         .filter_map(|chunk| {
    //             if chunk.len() == 4 {
    //                 let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
    //                 let float = f32::from_le_bytes(bytes);
    //                 Some((float * 32767.0) as i16)
    //             } else {
    //                 None
    //             }
    //         })
    //         .collect();

    //     const MIN_SAMPLES: usize = 48000; // 1 second minimum at 48kHz

    //     // Check if we have enough samples for Chromaprint
    //     if samples.len() >= MIN_SAMPLES {
    //         // Try Chromaprint fingerprinting
    //         let mut c = Chromaprint::new();
    //         c.start(48000, 1);
    //         c.feed(&samples);
    //         c.finish();

    //         if let Some(fingerprint) = c.raw_fingerprint() {
    //             println!(
    //                 "Generated raw fingerprint for: {} size; {}",
    //                 self.get_filename(),
    //                 fingerprint.len()
    //             );
    //             // Convert Vec<i32> to bytes before encoding
    //             let bytes: Vec<u8> = fingerprint.iter().flat_map(|&x| x.to_le_bytes()).collect();
    //             let encoded = general_purpose::STANDARD.encode(&bytes);
    //             if !encoded.is_empty() {
    //                 self.fingerprint = Some(Arc::from(encoded.as_str()));
    //                 return Some(encoded);
    //             }
    //         }

    //         eprintln!("Chromaprint failed despite sufficient samples");
    //     }

    //     // Fallback to PCM hash if:
    //     // 1. File is too short for Chromaprint, or
    //     // 2. Chromaprint failed to generate a fingerprint
    //     if !samples.is_empty() {
    //         use sha2::{Digest, Sha256};

    //         let mut hasher = Sha256::new();
    //         for sample in &samples {
    //             hasher.update(sample.to_le_bytes());
    //         }

    //         let hash = hasher.finalize();
    //         println!("Generated PCM hash for: {}", self.get_filename());
    //         let fingerprint = format!("PCM:{}", general_purpose::STANDARD.encode(hash));
    //         self.fingerprint = Some(Arc::from(fingerprint.as_str()));
    //         return Some(fingerprint);
    //     }

    //     eprintln!(
    //         "Failed to generate any fingerprint for: {}",
    //         self.get_filepath()
    //     );
    //     None
    // }
}

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
                println!("Subset Match selected");
                self.subset_match(pref, app).await?;
            }
            WaveformMatchType::Exact => {
                println!("Exact Match selected");
                self.exact_match(pref, app).await?;
            }
            WaveformMatchType::Similar => {
                println!("Similar Match selected");
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
        let Some(pool) = &self.pool else {
            return Err("sqlx::Error::PoolClosed".to_string());
        };

        let table = pool.get_table_name().to_string();
        let mut batch_size: usize = pref.batch_size;
        println!("Batch size: {}", batch_size);
        let total_records = self
            .records
            .iter()
            .filter(|record| {
                record.fingerprint.is_none() || record.fingerprint == Some(Arc::from("FAILED"))
            })
            .count();
        if total_records == 0 {
            println!("No records available for fingerprinting.");
            return Ok(());
        }
        if batch_size > total_records {
            batch_size = total_records;
        }
        let started = AtomicUsize::new(0);
        let completed = AtomicUsize::new(0);

        // let pool = self.get_pool().await;

        // let Some(pool) = pool else {
        //     println!("No database connection pool available, skipping fingerprint storage.");
        //     return Err("Database connection pool not available".to_string());
        // };

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
                    if self.abort.load(Ordering::SeqCst)
                        || (record.fingerprint.is_some()
                            && record.fingerprint != Some(Arc::from("FAILED")))
                        || !path.exists()
                    // || !path.is_file()
                    {
                        return None;
                    }
                    let new_started = started.fetch_add(1, Ordering::SeqCst) + 1;

                    app.status(
                        "Fingerprinting",
                        new_started * 100 / total_records,
                        &format!(
                            "Generating Audio Fingerprint: ({}/{}) {}",
                            new_started,
                            total_records,
                            record.get_filename()
                        ),
                    );
                    let fingerprint_result = record.get_chromaprint_fingerprint();

                    let fingerprint = fingerprint_result.unwrap_or("FAILED".to_string());
                    let new_completed = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    app.substatus(
                        "Fingerprinting",
                        (new_completed % batch_size) * 100 / batch_size,
                        &format!(
                            "Completed Audio Fingerprint in Batch: ({}/{}) {}",
                            (new_completed % batch_size),
                            batch_size,
                            record.get_filename()
                        ),
                    );
                    Some((record.id, fingerprint))
                })
                .collect();

            record_ids_to_store.extend(local_ids);

            if pref.store_waveforms && record_ids_to_store.len() >= pref.batch_size {
                // Store fingerprints in batches to avoid memory issues
                pool.store_fingerprints_batch_optimized(&record_ids_to_store, app, &table)
                    .await;
                record_ids_to_store.clear(); // Clear after storing
            }
        }

        if pref.store_waveforms {
            // Store fingerprints in batches to avoid memory issues
            pool.store_fingerprints_batch_optimized(&record_ids_to_store, app, &table)
                .await;
            record_ids_to_store.clear(); // Clear after storing
        }

        Ok(())
    }

    // async fn subset_match_ai(&mut self, pref: &Preferences, app: &AppHandle) -> Result<(), String> {
    //     app.substatus("Subset Search", 0, "Starting audio subset detection...");

    //     // OPTIMIZATION 1: Cache durations to avoid repeated calculation
    //     let duration_cache: HashMap<usize, f64> = self
    //         .records
    //         .iter()
    //         .map(|record| (record.id, record.get_duration().unwrap_or(0.0)))
    //         .collect();

    //     // Sort by duration (descending) and then by channel count (descending)
    //     self.records.sort_by(|a, b| {
    //         let a_duration = duration_cache.get(&a.id).unwrap_or(&0.0);
    //         let b_duration = duration_cache.get(&b.id).unwrap_or(&0.0);

    //         match b_duration.partial_cmp(a_duration) {
    //             Some(std::cmp::Ordering::Equal) => b.channels.cmp(&a.channels),
    //             Some(order) => order,
    //             None => std::cmp::Ordering::Equal,
    //         }
    //     });
    //     app.substatus("Subset Search", 5, "Decoding fingerprints...");

    //     // Decode fingerprints once to avoid repeated work
    //     let decoded_fingerprints: HashMap<usize, Vec<u32>> = self
    //         .records
    //         .par_iter()
    //         .filter_map(|record| {
    //             if let Some(fp) = &record.fingerprint {
    //                 if let Ok(decoded) = decode_chromaprint(fp) {
    //                     return Some((record.id, decoded));
    //                 }
    //             }
    //             None
    //         })
    //         .collect();

    //     let mut parent_children_map: HashMap<usize, Vec<usize>> = HashMap::new();
    //     let subset_threshold = (pref.similarity_threshold / 100.0) * 0.9;

    //     // OPTIMIZATION: Track which records are already identified as children
    //     let mut identified_children = BitSet::with_capacity(self.records.len());

    //     // Collect record IDs for direct indexing
    //     let record_ids: Vec<usize> = self.records.iter().map(|r| r.id).collect();

    //     let len = self.records.len();

    //     // Process each record - this is the core of the subset_match_tf algorithm
    //     for i in 0..len {
    //         if self.abort.load(Ordering::SeqCst) {
    //             return Err("Aborted".to_string());
    //         }

    //         // Update progress every 10 records or at milestones
    //         // if i % 10 == 0 || i == len - 1 {
    //         app.substatus(
    //             "Subset Search",
    //             10 + (i * 80 / len),
    //             &format!("Finding subset relationships ({}/{})", i + 1, len),
    //         );

    //         let record_id = record_ids[i];

    //         // Skip if already identified as a child
    //         if identified_children.contains(i) {
    //             continue;
    //         }

    //         // Get current record's duration for filtering
    //         let current_duration = *duration_cache.get(&record_id).unwrap_or(&0.0);

    //         // OPTIMIZATION: Find if this record is a child of any previous record
    //         // Filter parents first by duration to reduce comparisons
    //         let result = parent_children_map
    //             .par_iter()
    //             .filter(|(k, _)| {
    //                 let parent_id = record_ids[**k];
    //                 let parent_duration = *duration_cache.get(&parent_id).unwrap_or(&0.0);
    //                 // Only consider records with significantly longer duration as parents
    //                 parent_duration > current_duration * 1.05
    //             })
    //             .find_map_any(|(k, _)| {
    //                 let parent_id = record_ids[*k];

    //                 // Get fingerprints
    //                 let parent_fp = match decoded_fingerprints.get(&parent_id) {
    //                     Some(fp) => fp,
    //                     None => return None,
    //                 };

    //                 let child_fp = match decoded_fingerprints.get(&record_id) {
    //                     Some(fp) => fp,
    //                     None => return None,
    //                 };

    //                 // Skip comparison if lengths don't make sense
    //                 if child_fp.len() > parent_fp.len() {
    //                     return None;
    //                 }

    //                 // Do the actual subset comparison
    //                 if is_fingerprint_subset(child_fp, parent_fp, subset_threshold) {
    //                     Some(*k)
    //                 } else {
    //                     None
    //                 }
    //             });

    //         // Update parent-child relationships based on result
    //         match result {
    //             Some(key) => {
    //                 // Mark parent record
    //                 self.records[key].algorithm.insert(Algorithm::Waveforms);
    //                 self.records[key].algorithm.insert(Algorithm::Keep);

    //                 // Mark child record
    //                 self.records[i].algorithm.insert(Algorithm::Waveforms);
    //                 self.records[i].algorithm.remove(&Algorithm::Keep);

    //                 identified_children.insert(i);
    //                 parent_children_map.entry(key).or_default().push(i);
    //             }
    //             None => {
    //                 parent_children_map.insert(i, vec![]);
    //             }
    //         }

    //         // Allow other tasks to execute periodically
    //         if i % 100 == 99 {
    //             tokio::task::yield_now().await;
    //         }
    //     }
    //     app.substatus(
    //         "Subset Search",
    //         90,
    //         "Organizing records by parent-child relationships...",
    //     );

    //     // Sort records to group parents with children
    //     let mut sorted_records = Vec::with_capacity(self.records.len());
    //     let mut processed_ids = BitSet::with_capacity(self.records.len());

    //     // First pass: Process parents and their children
    //     for (parent_idx, child_indices) in &parent_children_map {
    //         if processed_ids.contains(*parent_idx) {
    //             continue;
    //         }

    //         sorted_records.push(self.records[*parent_idx].clone());
    //         processed_ids.insert(*parent_idx);

    //         for &child_idx in child_indices {
    //             sorted_records.push(self.records[child_idx].clone());
    //             processed_ids.insert(child_idx);
    //         }
    //     }

    //     // Second pass: Add remaining records
    //     for (i, record) in self.records.iter().enumerate() {
    //         if !processed_ids.contains(i) {
    //             sorted_records.push(record.clone());
    //         }
    //     }

    //     self.records = sorted_records;
    //     app.substatus(
    //         "Subset Search",
    //         100,
    //         &format!(
    //             "Subset detection complete: {} parent files, {} subset files",
    //             parent_children_map.len(),
    //             identified_children.len()
    //         ),
    //     );

    //     Ok(())
    // }

    async fn subset_match(&mut self, pref: &Preferences, app: &AppHandle) -> Result<(), String> {
        app.substatus("Subset Search", 0, "Starting audio subset detection...");

        self.records.sort_by(|a, b| {
            let a_duration = a.get_duration().unwrap_or(0.0);
            let b_duration = b.get_duration().unwrap_or(0.0);

            // First compare by duration (descending order)
            match b_duration.partial_cmp(&a_duration) {
                Some(std::cmp::Ordering::Equal) => {
                    // When durations are equal, sort by channel count (descending order)
                    b.channels.cmp(&a.channels)
                }
                Some(order) => order,
                None => std::cmp::Ordering::Equal,
            }
        });
        app.substatus(
            "Subset Search",
            5,
            "Decoding fingerprints for subset analysis...",
        );

        let decoded_fingerprints: HashMap<usize, Vec<u32>> = self
            .records
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

        let mut parent_children_map: HashMap<usize, Vec<usize>> = HashMap::new();
        let subset_threshold = (pref.similarity_threshold / 100.0) * 0.9;

        // Collect record IDs first to avoid repeated lookups
        let record_ids: Vec<usize> = self.records.iter().map(|r| r.id).collect();

        let len = self.records.len();
        for i in 0..len {
            if self.abort.load(Ordering::SeqCst) {
                println!("Aborting fingerprint scan - early exit");
                return Err("Aborted".to_string());
            }
            app.substatus(
                "Subset Search",
                10 + (i * 80 / len),
                &format!("Finding subset relationships ({}/{})", i + 1, len),
            );

            let record_id = record_ids[i];

            let result = parent_children_map
                .par_iter() // Use parallel iterator
                .find_map_any(|(k, _)| {
                    // find_map_any will short-circuit on first Some result
                    let parent_id = record_ids[*k];
                    let parent_fp = match decoded_fingerprints.get(&parent_id) {
                        Some(fp) => fp,
                        None => return None,
                    };

                    let child_fp = match decoded_fingerprints.get(&record_id) {
                        Some(fp) => fp,
                        None => return None,
                    };

                    if is_fingerprint_subset(child_fp, parent_fp, subset_threshold) {
                        Some(*k) // Return the key if we find a match
                    } else {
                        None // No match
                    }
                });
            match result {
                Some(key) => {
                    self.records[key].algorithm.insert(Algorithm::Waveforms);
                    self.records[key].algorithm.insert(Algorithm::Keep);
                    // This is a child file
                    self.records[i].algorithm.insert(Algorithm::Waveforms);
                    self.records[i].algorithm.remove(&Algorithm::Keep);
                    parent_children_map.entry(key).or_default().push(i);
                }
                None => {
                    parent_children_map.insert(i, vec![]);
                }
            }
        }
        // After creating parent_children_map
        let mut sorted_records = Vec::with_capacity(self.records.len());
        let mut processed_ids = BitSet::with_capacity(self.records.len());

        // First pass: Process all parents and their children in order
        for (parent_idx, child_indices) in &parent_children_map {
            // Skip if this record was already processed
            if processed_ids.contains(*parent_idx) {
                continue;
            }

            // Add the parent record
            sorted_records.push(self.records[*parent_idx].clone());
            processed_ids.insert(*parent_idx);

            // Add all children (if any)
            if !child_indices.is_empty() {
                // Add each child in the order they were found
                for &child_idx in child_indices {
                    sorted_records.push(self.records[child_idx].clone());
                    processed_ids.insert(child_idx);
                }
            }
        }

        // Second pass: Add any records not yet processed (not part of any parent-child relationship)
        for (i, record) in self.records.iter().enumerate() {
            if !processed_ids.contains(i) {
                sorted_records.push(record.clone());
            }
        }

        // Replace original records with sorted records
        self.records = sorted_records;

        Ok(())
    }

    // async fn subset_match_old(
    //     &mut self,
    //     pref: &Preferences,
    //     app: &AppHandle,
    // ) -> Result<(), String> {
    //     app.substatus("Subset Search", 0, "Starting audio subset detection...");

    //     // Filter records to only those with valid Chromaprint fingerprints
    //     let valid_records: Vec<&FileRecord> = self
    //         .records
    //         .par_iter()
    //         .filter(|record| {
    //             record.fingerprint.as_ref().is_some_and(|fp| {
    //                 !fp.is_empty() && &**fp != "FAILED" && !fp.starts_with("PCM:")
    //             })
    //         })
    //         .collect();

    //     let total_records = valid_records.len();
    //     println!(
    //         "Found {} records with valid fingerprints for subset analysis",
    //         total_records
    //     );

    //     if total_records == 0 {
    //         return Ok(());
    //     }

    //     // Step 1: Sort records by duration (longer files first)
    //     app.substatus("Subset Search", 5, "Preparing files for subset analysis...");

    //     // Create a sorted list of records by duration
    //     let mut records_by_duration = valid_records.clone();
    //     records_by_duration.sort_by(|a, b| {
    //         let a_duration = a.duration.parse::<f64>().unwrap_or(0.0);
    //         let b_duration = b.duration.parse::<f64>().unwrap_or(0.0);
    //         b_duration
    //             .partial_cmp(&a_duration)
    //             .unwrap_or(std::cmp::Ordering::Equal)
    //     });

    //     // Step 2: Decode all fingerprints once to avoid repeated work
    //     app.substatus("Subset Search", 10, "Decoding fingerprints...");

    //     // Map of record ID to decoded fingerprint
    //     let decoded_fingerprints: HashMap<usize, Vec<u32>> = valid_records
    //         .par_iter()
    //         .filter_map(|record| {
    //             if let Some(fp) = &record.fingerprint {
    //                 if let Ok(decoded) = decode_chromaprint(fp) {
    //                     return Some((record.id, decoded));
    //                 }
    //             }
    //             None
    //         })
    //         .collect();

    //     println!(
    //         "Successfully decoded {} fingerprints",
    //         decoded_fingerprints.len()
    //     );

    //     // Step 3: Find subset relationships
    //     app.substatus("Subset Search", 20, "Finding subset relationships...");

    //     // Track parent-child relationships
    //     let mut parent_children_map: HashMap<usize, Vec<usize>> = HashMap::new();
    //     let mut child_parent_map: HashMap<usize, usize> = HashMap::new();

    //     // For large datasets, process in batches
    //     let batch_size = 1000;
    //     let total_batches = records_by_duration.len().div_ceil(batch_size);

    //     // Use a threshold slightly lower than for similarity matching
    //     let subset_threshold = (pref.similarity_threshold / 100.0) * 0.9;

    //     for batch_idx in 0..total_batches {
    //         let batch_start = batch_idx * batch_size;
    //         let batch_end = ((batch_idx + 1) * batch_size).min(records_by_duration.len());
    //         app.substatus(
    //             "Subset Search",
    //             20 + (batch_idx * 60 / total_batches),
    //             &format!(
    //                 "Processing batch {}/{} (files {}-{})",
    //                 batch_idx + 1,
    //                 total_batches,
    //                 batch_start + 1,
    //                 batch_end
    //             ),
    //         );

    //         // Process this batch of potential parents
    //         for i in batch_start..batch_end {
    //             let parent_record = records_by_duration[i];
    //             let parent_id = parent_record.id;

    //             // Skip if already marked as a child of another file
    //             if child_parent_map.contains_key(&parent_id) {
    //                 continue;
    //             }

    //             // Get parent fingerprint
    //             let parent_fp = match decoded_fingerprints.get(&parent_id) {
    //                 Some(fp) => fp,
    //                 None => continue,
    //             };

    //             let parent_duration = parent_record.duration.parse::<f64>().unwrap_or(0.0);

    //             // Find potential children (shorter duration files)
    //             for child_record in &records_by_duration[(i + 1)..] {
    //                 let child_id = child_record.id;

    //                 // Skip if already identified as a child or if it's the same file
    //                 if child_parent_map.contains_key(&child_id) || child_id == parent_id {
    //                     continue;
    //                 }

    //                 let child_duration = child_record.duration.parse::<f64>().unwrap_or(0.0);

    //                 if child_duration >= parent_duration {
    //                     continue;
    //                 }

    //                 // Get child fingerprint
    //                 let child_fp = match decoded_fingerprints.get(&child_id) {
    //                     Some(fp) => fp,
    //                     None => continue,
    //                 };

    //                 // Check if child is a subset of parent using sliding window approach
    //                 if is_fingerprint_subset(child_fp, parent_fp, subset_threshold) {
    //                     // Add relationship
    //                     parent_children_map
    //                         .entry(parent_id)
    //                         .or_default()
    //                         .push(child_id);
    //                     child_parent_map.insert(child_id, parent_id);
    //                 }
    //             }

    //             // Report progress periodically
    //             if (i - batch_start) % 50 == 0 {
    //                 let batch_progress = (i - batch_start) * 100 / (batch_end - batch_start);
    //                 app.substatus(
    //                     "Subset Search",
    //                     20 + ((batch_idx * 100 + batch_progress) * 60 / (total_batches * 100)),
    //                     &format!(
    //                         "Batch {}/{}: {} subset relationships found",
    //                         batch_idx + 1,
    //                         total_batches,
    //                         parent_children_map.values().map(|v| v.len()).sum::<usize>()
    //                     ),
    //                 );
    //             }
    //         }
    //     }

    //     // Step 4: Apply algorithm markings
    //     app.substatus("Subset Search", 85, "Applying algorithm markings...");

    //     let total_parents = parent_children_map.len();
    //     let total_children = child_parent_map.len();

    //     println!(
    //         "Found {} parent files containing {} child subsets",
    //         total_parents, total_children
    //     );

    //     // Update records with subset relationships
    //     self.records.par_iter_mut().for_each(|record| {
    //         let id = record.id;

    //         if parent_children_map.contains_key(&id) {
    //             // This is a parent file with subsets
    //             record.algorithm.insert(Algorithm::Waveforms);
    //             // Parents keep the Keep algorithm
    //         } else if let Some(_parent_id) = child_parent_map.get(&id) {
    //             // This is a child/subset file
    //             record.algorithm.insert(Algorithm::Waveforms);
    //             record.algorithm.remove(&Algorithm::Keep);
    //         }
    //     });
    //     app.substatus(
    //         "Subset Search",
    //         100,
    //         &format!(
    //             "Subset detection complete: {} parent files, {} subset files",
    //             total_parents, total_children
    //         ),
    //     );

    //     Ok(())
    // }

    async fn exact_match(&mut self, pref: &Preferences, app: &AppHandle) -> Result<(), String> {
        println!("Starting Exact Audio fingerprint analysis");
        app.substatus(
            "Exact Fingerprint Match",
            0,
            "Grouping identical audio fingerprints...",
        );

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
                app.substatus(
                    "Exact Fingerprint Match",
                    (i + 1) * 100 / self.records.len(),
                    &format!("Grouping by fingerprint: {}/{}", i + 1, self.records.len()),
                );
            }

            if let Some(fingerprint) = &record.fingerprint {
                file_groups
                    .entry(fingerprint.clone())
                    .or_default()
                    .push(record.clone());
            }
        }

        println!("Marking duplicate audio files");
        app.substatus(
            "Exact Fingerprint Match",
            0,
            "Marking duplicate audio files...",
        );

        // Process groups
        let group_count = file_groups.len();
        let processed_records: Vec<FileRecord> = file_groups
            .into_iter()
            .enumerate()
            .flat_map(|(i, (_, mut records))| {
                if i % RECORD_DIVISOR == 0 || i == 0 || i == group_count - 1 {
                    app.substatus(
                        "Exact Fingerprint Match",
                        (i + 1) * 100 / group_count,
                        &format!("Processing group: {}/{}", i + 1, group_count),
                    );
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

        app.substatus(
            "Exact Fingerprint Match",
            100,
            "Exact Audio fingerprint analysis complete",
        );

        Ok(())
    }

    async fn similar_match(&mut self, pref: &Preferences, app: &AppHandle) -> Result<(), String> {
        println!("Starting Similar Audio fingerprint analysis");
        let threshold = pref.similarity_threshold / 100.0;
        app.substatus(
            "Similar Fingerprint Match",
            0,
            "Starting similarity analysis...",
        );

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
            app.substatus(
                "Similar Fingerprint Match",
                0,
                &format!("Processing {} PCM hash records...", pcm_hash_records.len()),
            );

            // Group PCM hash records by hash value
            let mut hash_groups: HashMap<Arc<str>, Vec<FileRecord>> = HashMap::new();

            for (i, record) in pcm_hash_records.iter().enumerate() {
                if i % RECORD_DIVISOR == 0 {
                    app.substatus(
                        "Similar Fingerprint Match",
                        (i + 1) * 100 / pcm_hash_records.len(),
                        &format!("Grouping PCM hashes: {}/{}", i + 1, pcm_hash_records.len()),
                    );
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
                    app.substatus(
                        "Similar Fingerprint Match",
                        50 + ((i + 1) * 50 / hash_group_count),
                        &format!("Processing hash groups: {}/{}", i + 1, hash_group_count),
                    );
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
            app.substatus(
                "Similar Fingerprint Match",
                10,
                &format!(
                    "Decoding {} Chromaprint fingerprints...",
                    chromaprint_records.len()
                ),
            );

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
                            app.substatus(
                                "Similar Fingerprint Match",
                                10 + ((i * 30) / total_records),
                                &format!("Decoding fingerprints: {}/{}", i, total_records),
                            );
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
                app.substatus(
                    "Similar Fingerprint Match",
                    40,
                    "Comparing fingerprints for similarities...",
                );

                // Second pass - build groups with similarity with progress updates
                let mut groups: HashMap<usize, Vec<(usize, Vec<u32>)>> = HashMap::new();
                let mut next_group_id = 0;

                // Process in smaller batches and report progress
                for i in 0..total_records {
                    app.substatus(
                        "Similar Fingerprint Match",
                        i * 100 / total_records,
                        &format!("Finding similar audio: {}/{}", i, total_records),
                    );

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
            app.substatus(
                "Similar Fingerprint Match",
                70,
                &format!(
                    "Processing {} similarity groups...",
                    similarity_groups.len()
                ),
            );

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
                        app.substatus(
                            "Similar Fingerprint Match",
                            70 + (groups_processed * 20 / total_groups),
                            &format!(
                                "Processing group {}/{} ({} items)",
                                groups_processed,
                                total_groups,
                                group.len()
                            ),
                        );
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
        app.substatus(
            "Similar Fingerprint Match",
            100,
            "Analysis complete: combined fingerprint and hash processing",
        );

        Ok(())
    }
}

fn decode_chromaprint(raw_fp: &str) -> Result<Vec<u32>, &'static str> {
    if raw_fp.starts_with("PCM:") {
        return Err("Not a Chromaprint fingerprint");
    }

    // Add debugging
    println!(
        "Decoding fingerprint: {} (len: {})",
        if raw_fp.len() > 20 {
            &raw_fp[0..20]
        } else {
            raw_fp
        },
        raw_fp.len()
    );

    match general_purpose::STANDARD.decode(raw_fp) {
        Ok(fp_bytes) => {
            println!("Successfully base64 decoded {} bytes", fp_bytes.len());

            let mut fp = Vec::with_capacity(fp_bytes.len() / 4);
            for chunk in fp_bytes.chunks_exact(4) {
                if chunk.len() == 4 {
                    let mut array = [0u8; 4];
                    array.copy_from_slice(chunk);
                    fp.push(u32::from_le_bytes(array));
                }
            }
            println!("Converted to {} u32 values", fp.len());
            Ok(fp)
        }
        Err(e) => {
            println!("Failed to decode fingerprint: {}", e);
            Err("Failed to decode fingerprint")
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

                let count = (_mm_extract_epi64(xor, 0)).count_ones()
                    + (_mm_extract_epi64(xor, 1)).count_ones();

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

pub fn calculate_similarity(fp1: &[u32], fp2: &[u32]) -> f64 {
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
