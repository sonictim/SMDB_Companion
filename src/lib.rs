#![warn(clippy::all, rust_2018_idioms)]
#![allow(non_snake_case)]

mod app;
pub use app::App;
mod assets;
mod config;
mod processing;
