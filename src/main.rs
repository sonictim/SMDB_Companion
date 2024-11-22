#![warn(clippy::all, rust_2018_idioms)]
#![allow(non_snake_case)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([750.0, 750.0])
            .with_min_inner_size([620.0, 620.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon2.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "SMDB Companion",
        native_options,
        Box::new(|cc| Ok(Box::new(SMDB_Companion::App::new(cc)))),
    )
}
