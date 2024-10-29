#![warn(clippy::all, rust_2018_idioms)]
#![allow(non_snake_case)]

mod app;
pub use app::App;
mod assets;
mod find_replace;
pub use assets::*;
mod config;

// mod dupe_panel;
mod processing;
pub use processing::*;

mod duplicates;
pub use duplicates::*;

mod prelude;

pub use eframe::egui::{self, RichText, Ui};
pub use sqlx::sqlite::SqlitePool;
pub use std::collections::HashSet;
pub use std::sync::Arc;
pub use tokio::sync::mpsc;
