use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use tauri::{AppHandle, Emitter};

use crate::*;
use crate::{FileRecord, preferences::*};

use super::shazam_fingerprint_processing::generate_landmarks;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioLandmark {
    pub freq_bin: u16,    // Frequency bin (0-512 for a 1024-point FFT)
    pub time_offset: f32, // Time in seconds
    pub amplitude: f32,   // Intensity of this peak
}

impl AudioLandmark {
    pub fn new(freq_bin: u16, time_offset: f32, amplitude: f32) -> Self {
        Self {
            freq_bin,
            time_offset,
            amplitude,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LandmarkPair {
    pub anchor: AudioLandmark, // First point (anchor)
    pub target: AudioLandmark, // Second point (target)
    pub time_delta: f32,       // Time difference between points
    pub hash: u64,             // Hash of this landmark pair
}

impl LandmarkPair {
    pub fn new(anchor: AudioLandmark, target: AudioLandmark) -> Self {
        let time_delta = target.time_offset - anchor.time_offset;

        // Create a hash of this landmark pair
        // The hash combines frequency bins and the time delta
        let hash = compute_landmark_hash(anchor.freq_bin, target.freq_bin, time_delta);

        Self {
            anchor,
            target,
            time_delta,
            hash,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchResult {
    Exact(usize),            // file_id
    Similar(usize, f64),     // file_id, similarity_score
    Subset(usize, f32, f64), // file_id, time_offset, similarity_score
}

// Compute a hash from landmark pair properties
fn compute_landmark_hash(freq1: u16, freq2: u16, time_delta: f32) -> u64 {
    // Quantize time delta to reduce sensitivity to small timing differences
    // This makes matching more robust
    let quantized_delta = (time_delta * 100.0).round() as u32;

    let mut hasher = DefaultHasher::new();
    // Combine the frequency bins and quantized time delta
    (freq1 as u32).hash(&mut hasher);
    (freq2 as u32).hash(&mut hasher);
    quantized_delta.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Clone)]
pub struct AudioFileInfo {
    pub id: usize,
    pub path: String,
    pub duration: f32,
    pub landmark_count: usize,
}

#[derive(Debug)]
pub struct FingerprintDatabase {
    // Hash → Vec<(file_id, time_offset)>
    landmarks: HashMap<u64, Vec<(usize, f32)>>,
    // File ID → metadata
    files: HashMap<usize, AudioFileInfo>,
    // Match typeå
    match_type: WaveformMatchType,
}
impl FingerprintDatabase {
    pub fn new(match_type: WaveformMatchType) -> Self {
        Self {
            landmarks: HashMap::new(),
            files: HashMap::new(),
            match_type,
        }
    }

    pub fn add_file(&mut self, file_id: usize, path: &str, landmarks: Vec<LandmarkPair>) -> usize {
        // Store file metadata
        let file_info = AudioFileInfo {
            id: file_id,
            path: path.to_string(),
            duration: if !landmarks.is_empty() {
                landmarks.last().unwrap().anchor.time_offset
            } else {
                0.0
            },
            landmark_count: landmarks.len(),
        };

        self.files.insert(file_id, file_info);

        // Add landmarks to index
        for landmark in &landmarks {
            self.landmarks
                .entry(landmark.hash)
                .or_default()
                .push((file_id, landmark.anchor.time_offset));
        }

        landmarks.len() // Return number of landmarks indexed
    }

    pub fn remove_file(&mut self, file_id: usize) {
        if let Some(info) = self.files.remove(&file_id) {
            // For each hash entry, remove all references to this file
            for landmarks in self.landmarks.values_mut() {
                landmarks.retain(|(id, _)| *id != file_id);
            }

            // Clean up empty hash entries
            self.landmarks.retain(|_, landmarks| !landmarks.is_empty());

            println!(
                "Removed file {} with {} landmarks",
                file_id, info.landmark_count
            );
        }
    }

    pub fn find_matches(&self, query_landmarks: &[LandmarkPair]) -> Vec<MatchResult> {
        if query_landmarks.is_empty() {
            return Vec::new();
        }

        // Count matching landmarks by file and time offset
        let mut match_counts: HashMap<(usize, i32), usize> = HashMap::new();
        let total_query_landmarks = query_landmarks.len();

        // Find all matching hashes in our database
        for landmark in query_landmarks {
            if let Some(matches) = self.landmarks.get(&landmark.hash) {
                for &(file_id, time_in_file) in matches {
                    // Calculate alignment bucket (quantized time offset)
                    // This accounts for the time position within each file
                    let query_time = landmark.anchor.time_offset;
                    let alignment = ((time_in_file - query_time) * 1000.0).round() as i32;

                    // Group matches by file_id and alignment
                    *match_counts.entry((file_id, alignment)).or_default() += 1;
                }
            }
        }

        match self.match_type {
            WaveformMatchType::Exact => {
                // For exact matches, we want a high percentage of landmarks to match
                // with consistent alignment

                // First group by file to find the best alignment for each
                let mut best_matches: HashMap<usize, (i32, usize)> = HashMap::new();

                for ((file_id, alignment), count) in match_counts {
                    if let Some((_, best_count)) = best_matches.get(&file_id) {
                        if count > *best_count {
                            best_matches.insert(file_id, (alignment, count));
                        }
                    } else {
                        best_matches.insert(file_id, (alignment, count));
                    }
                }

                // Filter to only keep files with high match percentage
                best_matches
                    .into_iter()
                    .filter(|(file_id, (_, count))| {
                        // If we have file info, compare against actual landmark count
                        if let Some(info) = self.files.get(file_id) {
                            // Match at least 15% of file's landmarks AND 40% of query landmarks
                            *count >= info.landmark_count.max(20) / 7
                                && *count >= total_query_landmarks * 2 / 5
                        } else {
                            // Without file info, just use query landmark count
                            *count >= total_query_landmarks * 2 / 5
                        }
                    })
                    .map(|(file_id, _)| MatchResult::Exact(file_id))
                    .collect()
            }
            WaveformMatchType::Similar => {
                // For similarity, we're more lenient with the match percentage

                // Group by file to find best alignment
                let mut best_matches: HashMap<usize, (i32, usize)> = HashMap::new();

                for ((file_id, alignment), count) in match_counts {
                    if let Some((_, best_count)) = best_matches.get(&file_id) {
                        if count > *best_count {
                            best_matches.insert(file_id, (alignment, count));
                        }
                    } else {
                        best_matches.insert(file_id, (alignment, count));
                    }
                }

                // Calculate similarity scores
                best_matches
                    .into_iter()
                    .filter(|(_, (_, count))| *count >= total_query_landmarks / 4)
                    .map(|(file_id, (_, count))| {
                        let score = count as f64 / total_query_landmarks as f64;
                        MatchResult::Similar(file_id, score)
                    })
                    .collect()
            }
            WaveformMatchType::Subset => {
                // For subset detection, we focus on consistent alignment

                // Group by file to find best alignment
                let mut best_matches: HashMap<usize, (i32, usize)> = HashMap::new();

                for ((file_id, alignment), count) in match_counts {
                    if let Some((_, best_count)) = best_matches.get(&file_id) {
                        if count > *best_count {
                            best_matches.insert(file_id, (alignment, count));
                        }
                    } else {
                        best_matches.insert(file_id, (alignment, count));
                    }
                }

                // Return files with a significant number of aligned matches
                best_matches
                    .into_iter()
                    .filter(|(_, (_, count))| *count >= total_query_landmarks / 3)
                    .map(|(file_id, (alignment, count))| {
                        MatchResult::Subset(
                            file_id,
                            alignment as f32 / 1000.0, // Convert alignment to seconds
                            count as f64 / total_query_landmarks as f64, // Match score
                        )
                    })
                    .collect()
            }
        }
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn landmark_count(&self) -> usize {
        self.landmarks.values().map(|v| v.len()).sum()
    }
}

pub struct AudioMatcher {
    database: FingerprintDatabase,
}

impl AudioMatcher {
    pub fn new(match_type: WaveformMatchType) -> Self {
        Self {
            database: FingerprintDatabase::new(match_type),
        }
    }

    pub fn add_file(&mut self, file_id: usize, path: &str) -> Result<usize, String> {
        println!("Fingerprinting file: {}", path);
        let landmarks = generate_landmarks(path);

        if landmarks.is_empty() {
            return Err(format!("Failed to generate landmarks for {}", path));
        }

        let count = self.database.add_file(file_id, path, landmarks);
        Ok(count)
    }

    pub fn find_matches(&self, query_path: &str) -> Result<Vec<MatchResult>, String> {
        println!("Searching for matches to: {}", query_path);
        let landmarks: Vec<LandmarkPair> = generate_landmarks(query_path);

        if landmarks.is_empty() {
            return Err(format!("Failed to generate landmarks for {}", query_path));
        }

        println!("Generated {} landmarks for query", landmarks.len());
        let matches = self.database.find_matches(&landmarks);
        Ok(matches)
    }

    pub fn set_match_type(&mut self, match_type: WaveformMatchType) {
        self.database.match_type = match_type;
    }

    pub fn stats(&self) -> (usize, usize) {
        (self.database.file_count(), self.database.landmark_count())
    }
}

impl Database {
    pub async fn wave_search_shazam(
        &mut self,
        pref: &Preferences,
        app: &AppHandle,
    ) -> Result<(), String> {
        println!("Starting Shazam Search");

        let mut matcher = AudioMatcher::new(WaveformMatchType::Exact);

        for (i, record) in self.records.iter().enumerate() {
            let _ = matcher.add_file(i, record.get_filepath());
        }

        Ok(())
    }

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
        let mut matcher = AudioMatcher::new(WaveformMatchType::Similar); // Set to detect segments

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
}
