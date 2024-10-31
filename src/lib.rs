#![warn(clippy::all, rust_2018_idioms)]
#![allow(non_snake_case)]

mod app;
pub use app::App;
mod find_replace;
// pub use find_replace::*;
mod assets;
pub use assets::*;
mod config;
pub use config::*;
// mod dupe_panel;
mod processing;
pub use processing::*;

mod prelude;
