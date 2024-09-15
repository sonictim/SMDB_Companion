#![warn(clippy::all, rust_2018_idioms)]
#![allow(non_snake_case)]


mod app;
pub use app::TemplateApp;
mod assets;
mod processing;