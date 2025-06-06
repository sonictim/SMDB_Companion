use sqlx::{mysql::MySqlRow, sqlite::SqliteRow};

pub use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone)] // Need Deserialize to receive it back
pub struct FileRecordFrontend {
    pub id: usize,
    pub path: Arc<str>,
    pub filename: Arc<str>,
    pub algorithm: Vec<Algorithm>,
    pub channels: i64,
    pub bitdepth: i64,
    pub samplerate: i64,
    pub duration: Arc<str>,
    pub description: Arc<str>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DualMono {
    pub id: usize,
    pub path: String,
}

#[derive(Default, Debug, Serialize, Clone)]
pub struct FileRecord {
    pub id: usize,
    pub path: std::path::PathBuf,          // Made private
    pub root: Arc<str>,                    // Made private
    pub duration: Arc<str>,                // Made private
    pub samplerate: i64,                   // Made private
    pub bitdepth: i64,                     // Made private
    pub channels: i64,                     // Made private
    pub description: Arc<str>,             // Made private
    pub data: HashMap<Arc<str>, Arc<str>>, // Made private
    pub fingerprint: Option<Arc<str>>,     // Made private
    pub dual_mono: Option<bool>,           // Made private
    pub algorithm: HashSet<Algorithm>,     // Made private
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
    pub fn new_mysql(
        row: &MySqlRow,
        enabled: &Enabled,
        pref: &Preferences,
        is_compare: bool,
    ) -> Option<Self> {
        let _ = is_compare;
        let id = row.try_get::<i64, _>(0).unwrap_or(0) as usize;
        let path_str: &str = row.try_get(1).unwrap_or("");

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
            Some(Arc::from(f))
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
            root: Arc::default(),
            duration: Arc::from(duration_str),
            data,
            fingerprint,
            algorithm,
            channels,
            bitdepth,
            samplerate,
            description: Arc::from(description),
            dual_mono,
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
        let id = row.try_get::<i64, _>(0).unwrap_or(0) as usize;
        let path_str: &str = row.try_get(1).unwrap_or("");

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
            if let Some(value) = get_column_as_string_sqlite(row, column) {
                data.insert(column.clone(), value);
            }
        }

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
            Some(Arc::from(f))
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
            root: Arc::default(),
            duration: Arc::from(duration_str),
            data,
            fingerprint,
            algorithm,
            channels,
            bitdepth,
            samplerate,
            description: Arc::from(description),
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
            Arc::from(name.as_ref())
        } else {
            Arc::from(format!("{}.{}", name.as_ref(), self.get_extension()))
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
            Arc::from(name)
        } else {
            Arc::from(format!("{}.{}", name, self.get_extension()))
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

fn get_column_as_string_sqlite(row: &SqliteRow, column: &str) -> Option<Arc<str>> {
    // Try getting as text first (most common case)
    if let Ok(value) = row.try_get::<&str, _>(column) {
        return Some(Arc::from(value));
    }

    // Then try numeric types
    if let Ok(value) = row.try_get::<i64, _>(column) {
        return Some(Arc::from(value.to_string()));
    }

    if let Ok(value) = row.try_get::<f64, _>(column) {
        return Some(Arc::from(value.to_string()));
    }

    // Handle null or other types
    None
}
fn get_column_as_string_mysql(row: &MySqlRow, column: &str) -> Option<Arc<str>> {
    // Try getting as text first (most common case)
    if let Ok(value) = row.try_get::<&str, _>(column) {
        return Some(Arc::from(value));
    }

    // Then try numeric types
    if let Ok(value) = row.try_get::<i64, _>(column) {
        return Some(Arc::from(value.to_string()));
    }

    if let Ok(value) = row.try_get::<f64, _>(column) {
        return Some(Arc::from(value.to_string()));
    }

    // Handle null or other types
    None
}
