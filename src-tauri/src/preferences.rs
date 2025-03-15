use std::{cmp::Ordering, collections::HashSet, sync::Arc};

pub use Algorithm as A;
pub use OrderOperator as O;
use chrono::{Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::FileRecord;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Preferences {
    pub columns: Vec<Arc<str>>,
    pub match_criteria: Vec<Arc<str>>,
    pub ignore_filetype: bool,
    pub tags: Vec<Arc<str>>,
    pub autoselects: Vec<Arc<str>>,
    pub preservation_order: Vec<PreservationLogic>,
    pub display_all_records: bool,
    pub exact_waveform: bool,
    pub store_waveforms: bool,
    pub fetch_waveforms: bool,
    pub similarity_threshold: f64,
}

impl Preferences {
    pub fn sort_vec(&self, vec: &mut [FileRecord]) {
        for l in self.preservation_order.iter().rev() {
            l.sort(vec);
        }
        vec.sort_by(|a, b| {
            let a_root = self.check_tags(a.get_filestem());
            let b_root = self.check_tags(b.get_filestem());
            a_root.cmp(&b_root)
        });

        vec.sort_by(|a, b| {
            let a_root = if self.ignore_filetype {
                a.get_filestem() == a.root.as_ref()
            } else {
                a.get_filename() == a.root.as_ref()
            };
            let b_root = if self.ignore_filetype {
                b.get_filestem() == b.root.as_ref()
            } else {
                b.get_filename() == b.root.as_ref()
            };
            // Reverse the comparison to prioritize matches
            a_root.cmp(&b_root).reverse()
        });

        vec.sort_by(|a, b| {
            let a_already_marked = a.algorithm.contains(&A::Keep);
            let b_already_marked = b.algorithm.contains(&A::Keep);
            // Reverse the comparison order to prioritize records with A::Keep
            a_already_marked.cmp(&b_already_marked).reverse()
            // Alternatively: b_already_marked.cmp(&a_already_marked).reverse()
        });
    }
    pub fn check_tags(&self, item: &str) -> bool {
        for tag in &self.tags {
            if item.contains(&**tag) {
                return true;
            }
        }
        false
    }

    pub fn get_data_requirements(&self) -> Arc<str> {
        let mut set: HashSet<&str> = HashSet::new();
        for m in &self.match_criteria {
            set.insert(m);
        }
        for m in &self.preservation_order {
            let m = &m.column;
            set.insert(m);
        }
        Arc::from(set.iter().copied().collect::<Vec<_>>().join(","))
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PreservationLogic {
    pub column: Arc<str>,
    pub operator: OrderOperator,
    pub variable: Arc<str>,
}
impl PreservationLogic {
    fn sort(&self, vec: &mut [FileRecord]) {
        match self.operator {
            O::Largest => {
                vec.sort_by(|a, b| {
                    // Get values from each FileRecord
                    let a_value = a.data.get(&self.column).map_or("", |v| v);
                    let b_value = b.data.get(&self.column).map_or("", |v| v);

                    // Parse values
                    let a_num = parse_string(a_value).unwrap_or(ParsedValue::Integer(0));
                    let b_num = parse_string(b_value).unwrap_or(ParsedValue::Integer(0));

                    // Compare b to a for descending order
                    b_num.cmp(&a_num)
                });
            }
            O::Smallest => {}
            O::Is => {
                vec.sort_by(|a, b| {
                    let a_matches = a
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.as_ref() == self.variable.as_ref());
                    let b_matches = b
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.as_ref() == self.variable.as_ref());

                    b_matches.cmp(&a_matches)
                });
            }
            O::IsNot => {
                vec.sort_by(|a, b| {
                    let a_matches = a
                        .data
                        .get(&self.column)
                        .map_or(false, |v| *v == self.variable);
                    let b_matches = b
                        .data
                        .get(&self.column)
                        .map_or(false, |v| *v == self.variable);

                    a_matches.cmp(&b_matches)
                });
            }
            O::IsEmpty => {
                vec.sort_by(|a, b| {
                    let a_empty = a.data.get(&self.column).map_or(false, |v| v.is_empty());
                    let b_empty = b.data.get(&self.column).map_or(false, |v| v.is_empty());
                    b_empty.cmp(&a_empty)
                });
            }
            O::IsNotEmpty => {
                vec.sort_by(|a, b| {
                    let a_empty = a.data.get(&self.column).map_or(false, |v| v.is_empty());
                    let b_empty = b.data.get(&self.column).map_or(false, |v| v.is_empty());
                    a_empty.cmp(&b_empty)
                });
            }
            O::Contains => {
                vec.sort_by(|a, b| {
                    let a_contains = a
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.contains(self.variable.as_ref()));
                    let b_contains = b
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.contains(self.variable.as_ref()));
                    b_contains.cmp(&a_contains)
                });
            }
            O::DoesNotContain => {
                vec.sort_by(|a, b| {
                    let a_contains = a
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.contains(self.variable.as_ref()));
                    let b_contains = b
                        .data
                        .get(&self.column)
                        .map_or(false, |v| v.contains(self.variable.as_ref()));
                    a_contains.cmp(&b_contains)
                });
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ParsedValue {
    Integer(i64),
    Duration(Duration),
    DateTime(NaiveDateTime),
}

impl PartialOrd for ParsedValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (ParsedValue::Integer(a), ParsedValue::Integer(b)) => a.partial_cmp(b),
            (ParsedValue::Duration(a), ParsedValue::Duration(b)) => {
                a.num_milliseconds().partial_cmp(&b.num_milliseconds())
            }
            (ParsedValue::DateTime(a), ParsedValue::DateTime(b)) => a.partial_cmp(b),

            // For different types, we define a custom ordering or return None.
            (ParsedValue::Integer(_), _) => Some(Ordering::Less),
            (ParsedValue::Duration(_), ParsedValue::DateTime(_)) => Some(Ordering::Less),
            (ParsedValue::DateTime(_), ParsedValue::Integer(_)) => Some(Ordering::Greater),
            _ => None,
        }
    }
}

impl Ord for ParsedValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

// Parse a string into an appropriate `ParsedValue`
fn parse_string(value: &str) -> Result<ParsedValue, &'static str> {
    // Try to parse as an integer
    if let Ok(int_value) = value.parse::<i64>() {
        return Ok(ParsedValue::Integer(int_value));
    }

    // Try to parse as a duration in "MM:SS.mmm" or "HH:MM:SS.mmm" format
    if let Some(parsed_duration) = parse_duration(value) {
        return Ok(ParsedValue::Duration(parsed_duration));
    }

    // Try to parse as a datetime in "YYYY-MM-DD HH:MM:SS" format
    if let Ok(date_time) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Ok(ParsedValue::DateTime(date_time));
    }

    Err("Invalid format")
}

fn parse_duration(value: &str) -> Option<Duration> {
    let parts: Vec<&str> = value.split(&[':', '.'][..]).collect();
    match parts.len() {
        3 => {
            // "MM:SS.mmm"
            let minutes: i64 = parts[0].parse().ok()?;
            let seconds: i64 = parts[1].parse().ok()?;
            let millis: i64 = parts[2].parse().ok()?;
            Some(
                Duration::minutes(minutes)
                    + Duration::seconds(seconds)
                    + Duration::milliseconds(millis),
            )
        }
        4 => {
            // "HH:MM:SS.mmm"
            let hours: i64 = parts[0].parse().ok()?;
            let minutes: i64 = parts[1].parse().ok()?;
            let seconds: i64 = parts[2].parse().ok()?;
            let millis: i64 = parts[3].parse().ok()?;
            Some(
                Duration::hours(hours)
                    + Duration::minutes(minutes)
                    + Duration::seconds(seconds)
                    + Duration::milliseconds(millis),
            )
        }
        _ => None,
    }
}

#[derive(Debug, PartialEq, serde::Serialize, Deserialize, Clone, Copy, Default)]
pub enum OrderOperator {
    Largest,
    Smallest,
    #[default]
    Contains,
    DoesNotContain,
    Is,
    IsNot,
    IsEmpty,
    IsNotEmpty,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Enabled {
    pub basic: bool,
    pub invalidpath: bool,
    pub filename: bool,
    pub filetags: bool,
    pub audiosuite: bool,
    pub waveform: bool,
    pub duration: bool,
    pub dbcompare: bool,
    pub min_dur: f64,
    pub compare_db: Arc<str>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Registration {
    pub name: Arc<str>,
    pub email: Arc<str>,
    pub license: Arc<str>,
}

pub fn generate_license_key(name: &str, email: &str) -> Arc<str> {
    let salt = "Valhalla Delay";
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}{}", name.to_lowercase(), email.to_lowercase(), salt).as_bytes());
    let hash = hasher.finalize();

    // Option 1: Take first 16 bytes (32 characters) of the hash
    let shortened = &hash[..16];

    // Format as XXXX-XXXX-XXXX-XXXX for readability
    let formatted = format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}",
        shortened[0],
        shortened[1],
        shortened[2],
        shortened[3],
        shortened[4],
        shortened[5],
        shortened[6],
        shortened[7],
        shortened[8],
        shortened[9],
        shortened[10],
        shortened[11],
        shortened[12],
        shortened[13],
        shortened[14],
        shortened[15]
    );

    formatted.into()
}

pub fn generate_license_key_old(name: &str, email: &str) -> Arc<str> {
    let salt = "Valhalla Delay";
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}{}", name, email, salt).as_bytes());
    let hash = hasher.finalize();
    hex::encode_upper(hash).into()
}

#[derive(Clone, serde::Serialize)]
pub struct StatusUpdate {
    stage: String,
    progress: u64,
    message: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Algorithm {
    All,
    Basic,
    SimilarFilename,
    Waveforms,
    Compare,
    Tags,
    FileTags,
    InvalidPath,
    Duration,
    Replace,
    Manual,
    #[default]
    Keep,
}
