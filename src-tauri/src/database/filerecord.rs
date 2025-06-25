use ffcodex_lib::dprintln;

pub use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone)] // Need Deserialize to receive it back
pub struct FileRecordFrontend {
    pub id: usize,
    pub path: String,
    pub filename: String,
    pub algorithm: Vec<Algorithm>,
    pub channels: u32,
    pub bitdepth: u32,
    pub samplerate: u32,
    pub duration: String,
    pub description: String,
    // data: HashMap<String, String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DualMono {
    pub id: usize,
    pub path: String,
}

// Change visibility of `FileRecord` fields to private where possible
#[derive(Default, Debug, Serialize, Clone)]
pub struct FileRecord {
    pub id: usize,
    pub path: std::path::PathBuf,      // Made private
    pub root: String,                  // Made private
    pub duration: String,              // Made private
    pub samplerate: u32,               // Made private
    pub bitdepth: u32,                 // Made private
    pub channels: u32,                 // Made private
    pub description: String,           // Made private
    pub data: HashMap<String, String>, // Made private
    pub fingerprint: Option<String>,   // Made private
    pub dual_mono: Option<bool>,       // Made private
    pub algorithm: HashSet<Algorithm>, // Made private
}
impl Hash for FileRecord {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Only hash the fields you want to consider for identity
        self.id.hash(state);
        // Add any other fields you want to include in the hash calculation
        // For example: self.path.hash(state);
    }
}

impl PartialEq for FileRecord {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for FileRecord {}

impl FileRecord {
    pub fn new_from_path(
        path: PathBuf,
        index: usize,
        enabled: &Enabled,
        pref: &Preferences,
    ) -> Option<Self> {
        if !pref.safe_folders.is_empty()
            && pref.safe_folders.iter().any(|folder| {
                path.starts_with(folder) || path.starts_with(folder.trim_end_matches('/'))
            })
        {
            return None;
        }
        let file_info: ffcodex_lib::FileInfo =
            ffcodex_lib::get_basic_metadata(&path.display().to_string()).ok()?;

        let mut algorithm = HashSet::new();
        let mut keep = true;
        if enabled.invalidpath || enabled.dual_mono {
            // Use the path existence check from the PathBuf directly
            // This handles cross-platform path separators correctly
            let path_exists = path.exists();

            if !path_exists {
                algorithm.insert(Algorithm::InvalidPath);
                if enabled.invalidpath {
                    keep = false;
                }
            }
        }
        if enabled.duration && checkduration(&file_info.duration, enabled.min_dur) {
            algorithm.insert(Algorithm::Duration);
            keep = false;
        }
        if enabled.filetags && checktags(&path.display().to_string(), &pref.autoselects) {
            algorithm.insert(Algorithm::FileTags);
            keep = false;
        }
        if keep {
            algorithm.insert(Algorithm::Keep);
        }

        let mut record = Self {
            id: index,
            path,
            root: String::default(),
            duration: file_info.duration,
            data: HashMap::new(),
            fingerprint: None,
            algorithm,
            channels: file_info.channels as u32,
            bitdepth: file_info.bit_depth as u32,
            samplerate: file_info.sample_rate as u32,
            description: file_info.description,
            dual_mono: None,
        };

        record.set_root(enabled, pref);
        Some(record)
    }

    pub fn new_sqlite(
        row: &SqliteRow,
        enabled: &Enabled,
        pref: &Preferences,
        is_compare: bool,
    ) -> Option<Self> {
        let _ = is_compare;
        let id = row.get::<u32, _>(0) as usize;
        let path_str: &str = row.get(1);

        #[cfg(not(target_os = "windows"))]
        let path = PathBuf::from(path_str);

        #[cfg(target_os = "windows")]
        let mut path = PathBuf::from(path_str);
        #[cfg(target_os = "windows")]
        if let Some(p) = windows::auto_convert_macos_path_to_windows(path_str) {
            path = p;
        }

        if !pref.safe_folders.is_empty()
            && pref.safe_folders.iter().any(|folder| {
                path.starts_with(folder) || path.starts_with(folder.trim_end_matches('/'))
            })
        {
            println!("Skipping record with ID {}: Path is in a safe folder", id);
            return None;
        }

        let duration_str: &str = row.get(2);
        let description: &str = row.get(4);
        let channels = row.get(5);
        let bitdepth = row.get(6);
        let samplerate = row.get(7);

        let mut algorithm = HashSet::new();
        let mut keep = true;
        if enabled.invalidpath || enabled.dual_mono {
            // Use the path existence check from the PathBuf directly
            // This handles cross-platform path separators correctly
            let path_exists = path.exists();

            if !path_exists {
                algorithm.insert(Algorithm::InvalidPath);
                if enabled.invalidpath {
                    keep = false;
                }
            }
        }
        if enabled.duration && checkduration(duration_str, enabled.min_dur) {
            algorithm.insert(Algorithm::Duration);
            keep = false;
        }
        if enabled.filetags && checktags(path_str, &pref.autoselects) {
            algorithm.insert(Algorithm::FileTags);
            keep = false;
        }
        if keep {
            algorithm.insert(Algorithm::Keep);
        }

        let mut data = HashMap::new();

        // Gather required columns from preferences
        for column in &pref.match_criteria {
            if let Some(value) = get_column_as_string_sqlite(row, column) {
                data.insert(column.clone(), value);
            }
        }
        dprintln!("Data after match_criteria: {:?}", data);

        // Gather columns from preservation logic
        for logic in &pref.preservation_order {
            let column = &logic.column;
            if let Some(value) = get_column_as_string_sqlite(row, column) {
                data.insert(column.clone(), value);
            }
        }

        let f = row.try_get::<&str, _>("_fingerprint").unwrap_or_default();

        let fingerprint = if f.is_empty() || !pref.fetch_waveforms {
            None
        } else {
            Some(String::from(f))
        };

        let mut dual_mono = None;

        if pref.fetch_waveforms {
            let dm = row.try_get::<&str, _>("_DualMono").ok();
            dual_mono = match dm {
                Some("1") => Some(true),
                Some("0") => Some(false),
                _ => None,
            };
        }

        let mut record = Self {
            id,
            path,
            root: String::default(),
            duration: String::from(duration_str),
            data,
            fingerprint,
            algorithm,
            channels,
            bitdepth,
            samplerate,
            description: String::from(description),
            dual_mono,
        };

        record.set_root(enabled, pref);
        Some(record)
    }
    pub fn new_mysql(
        row: &MySqlRow,
        enabled: &Enabled,
        pref: &Preferences,
        is_compare: bool,
    ) -> Option<Self> {
        let _ = is_compare;
        let id = row.get::<u32, _>(0) as usize;
        let path_str: &str = row.get(1);

        #[cfg(not(target_os = "windows"))]
        let path = PathBuf::from(path_str);

        #[cfg(target_os = "windows")]
        let mut path = PathBuf::from(path_str);
        #[cfg(target_os = "windows")]
        if let Some(p) = windows::auto_convert_macos_path_to_windows(path_str) {
            path = p;
        }

        if !pref.safe_folders.is_empty()
            && pref.safe_folders.iter().any(|folder| {
                path.starts_with(folder) || path.starts_with(folder.trim_end_matches('/'))
            })
        {
            println!("Skipping record with ID {}: Path is in a safe folder", id);
            return None;
        }

        let duration_str: &str = row.try_get(2).unwrap_or("0:00.000");
        let description: &str = row.try_get(4).unwrap_or("");
        let channels = row.try_get(5).unwrap_or(0i64);
        let bitdepth = row.try_get(6).unwrap_or(0i64);
        let samplerate = row.try_get(7).unwrap_or(0i64);

        let mut algorithm = HashSet::new();
        let mut keep = true;
        if enabled.invalidpath || enabled.dual_mono {
            // Use the path existence check from the PathBuf directly
            // This handles cross-platform path separators correctly
            let path_exists = path.exists();

            if !path_exists {
                algorithm.insert(Algorithm::InvalidPath);
                if enabled.invalidpath {
                    keep = false;
                }
            }
        }
        if enabled.duration && checkduration(duration_str, enabled.min_dur) {
            algorithm.insert(Algorithm::Duration);
            keep = false;
        }
        if enabled.filetags && checktags(path_str, &pref.autoselects) {
            algorithm.insert(Algorithm::FileTags);
            keep = false;
        }
        if keep {
            algorithm.insert(Algorithm::Keep);
        }

        let mut data = HashMap::new();

        // Gather required columns from preferences
        for column in &pref.match_criteria {
            if let Some(value) = get_column_as_string_mysql(row, column) {
                data.insert(column.clone(), value);
            }
        }

        // Gather columns from preservation logic
        for logic in &pref.preservation_order {
            let column = &logic.column;
            if let Some(value) = get_column_as_string_mysql(row, column) {
                data.insert(column.clone(), value);
            }
        }

        let f = row.try_get::<&str, _>("_fingerprint").unwrap_or_default();

        let fingerprint = if f.is_empty() || !pref.fetch_waveforms {
            None
        } else {
            Some(String::from(f))
        };

        let mut dual_mono = None;

        if pref.fetch_waveforms {
            let dm = row.try_get::<&str, _>("_DualMono").ok();
            dual_mono = match dm {
                Some("1") => Some(true),
                Some("0") => Some(false),
                _ => None,
            };
        }

        let mut record = Self {
            id,
            path,
            root: String::default(),
            duration: String::from(duration_str),
            data,
            fingerprint,
            algorithm,
            channels: channels as u32,
            bitdepth: bitdepth as u32,
            samplerate: samplerate as u32,
            description: String::from(description),
            dual_mono,
        };

        record.set_root(enabled, pref);
        Some(record)
    }

    pub fn set_root(&mut self, _enabled: &Enabled, pref: &Preferences) {
        // Use Cow to avoid unnecessary string allocations
        let mut name = Cow::Borrowed(self.get_filestem().trim());

        // Optimize tag processing with minimal allocations
        if let Some((base, _)) = pref.tags.iter().find_map(|tag| name.split_once(&**tag)) {
            name = Cow::Owned(base.to_string());
        }

        // Use const array to avoid repeated allocations
        const COPY_TAGS: [&str; 6] = [" copy.", " Copy.", " COPY.", ".copy.", ".Copy.", ".COPY."];

        // Find and split with minimal allocations
        if let Some((base, _)) = COPY_TAGS.iter().find_map(|&tag| name.split_once(tag)) {
            name = Cow::Owned(base.to_string());
        }

        // Efficient regex capture
        name = Cow::Owned(
            FILENAME_REGEX
                .captures(&name)
                .and_then(|caps| caps.name("base").map(|m| m.as_str().to_string()))
                .unwrap_or_else(|| name.into_owned()),
        );

        // Minimize string allocations for root
        self.root = if pref.ignore_filetype {
            String::from(name.as_ref())
        } else {
            String::from(format!("{}.{}", name.as_ref(), self.get_extension()))
        };
    }

    pub fn get_filename(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Invalid Filename")
    }

    pub fn get_filestem(&self) -> &str {
        self.path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Invalid Filestem")
    }

    pub fn get_extension(&self) -> &str {
        self.path
            .extension()
            .and_then(|n| n.to_str())
            .unwrap_or("Invalid Extension")
    }

    pub fn get_path(&self) -> &str {
        self.path
            .parent()
            .and_then(|n| n.to_str())
            .unwrap_or("Invalid Path")
    }

    pub fn check_path(&self) -> bool {
        self.path.exists()
    }

    pub fn get_filepath(&self) -> String {
        // Use display() which handles both Windows and macOS paths correctly
        // and converts to lossy UTF-8 representation when needed
        self.path.display().to_string()
    }

    pub fn get_root(&self) -> String {
        let name = self.get_filestem();
        if let Some(caps) = FILENAME_REGEX.captures(name) {
            caps["base"].to_string()
        } else {
            name.to_string()
        }
    }
    pub fn set_root_old(&mut self, enabled: &Enabled, pref: &Preferences) {
        let mut name = self.get_filestem();
        println!("Source Root: {}", name);
        if enabled.audiosuite {
            for tag in &pref.tags {
                if let Some((base, _)) = name.split_once(&**tag) {
                    name = base;
                    println!("Tag Found: {} Root: {}", tag, name);
                }
            }
        };
        if enabled.filename {
            let copy = [" copy.", " Copy.", " COPY.", ".copy.", ".Copy.", ".COPY."];
            for tag in &copy {
                if let Some((base, _)) = name.split_once(tag) {
                    name = base;
                    println!("Filename Root: {}", name);
                }
            }
            name = FILENAME_REGEX
                .captures(name) // Use reference to avoid cloning
                .and_then(|caps| caps.name("base"))
                .map(|m| m.as_str())
                .unwrap_or_else(|| name);
            println!("similar filename Root: {}", name);
        }

        self.root = if pref.ignore_filetype {
            String::from(name)
        } else {
            String::from(format!("{}.{}", name, self.get_extension()))
        };
        println!("Final Root: {}", self.root);
    }

    pub fn get_duration(&self) -> Result<f64, String> {
        if let Some((minutes, rest)) = self.duration.split_once(':') {
            if let (Ok(mins), Ok(secs)) = (minutes.parse::<f64>(), rest.parse::<f64>()) {
                return Ok((mins * 60.0) + secs);
            }
        }
        Err("Unable to parse duration".to_string())
    }
}
