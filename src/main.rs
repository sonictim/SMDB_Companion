#![warn(clippy::all, rust_2018_idioms)]
#![allow(non_snake_case)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod window_manager;

use std::sync::{Arc, Mutex};
use window_manager::SharedState;

#[tokio::main]
async fn main() -> eframe::Result {
    env_logger::init();

    let shared_state = Arc::new(Mutex::new(SharedState::new()));

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([750.0, 750.0])
            .with_min_inner_size([620.0, 620.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-1024.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };

    eframe::run_native(
        "SMDB Companion",
        native_options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc, shared_state.clone())))),
    )
}
