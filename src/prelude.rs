pub use eframe::egui::{self, RichText, Ui};
pub use rayon::prelude::*;
pub use serde::Deserialize;

pub use sqlx::{sqlite::SqlitePool, Row};
pub use std::collections::HashSet;
pub use std::path::Path;
pub use std::result::Result;
pub use std::sync::Arc;
pub use tokio::sync::mpsc;

pub use crate::assets::*;
pub use crate::config::*;
pub use crate::duplicates::nodes::*;
pub use crate::duplicates::*;
pub use crate::find_replace::FindPanel;

pub const TABLE: &str = "justinmetadata";
