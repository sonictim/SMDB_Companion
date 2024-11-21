#![warn(clippy::all, rust_2018_idioms)]
#![allow(non_snake_case)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use crate::prelude::*;
use SMDB_Companion::*;

// const VERSION: &str = env!("CARGO_PKG_VERSION");

// #[tokio::main]
// async fn main() -> eframe::Result {
//     env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

//     let native_options = eframe::NativeOptions {
//         viewport: egui::ViewportBuilder::default()
//             .with_inner_size([750.0, 750.0])
//             .with_min_inner_size([620.0, 620.0])
//             .with_icon(
//                 // NOTE: Adding an icon is optional
//                 eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-1024.png")[..])
//                     .expect("Failed to load icon"),
//             ),
//         ..Default::default()
//     };
//     eframe::run_native(
//         "SMDB Companion",
//         native_options,
//         Box::new(|cc| Ok(Box::new(SMDB_Companion::App::new(cc)))),
//     )
// }

pub trait AppWindow: Default + eframe::App {
    fn window_title() -> &'static str;
    fn arg_name() -> String {
        format!(
            "--{}-window",
            Self::window_title().to_lowercase().replace(" ", "-")
        )
    }
    fn window_options() -> eframe::NativeOptions {
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([750.0, 750.0])
                .with_min_inner_size([620.0, 620.0]),
            ..Default::default()
        }
    }
}

fn main() -> Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let args: Vec<String> = std::env::args().collect();

    match args
        .iter()
        .find(|arg| arg.starts_with("--") && arg.ends_with("-window"))
    {
        // Some(arg) if arg == &SecondWindow::arg_name() => run_window::<SecondWindow>()?,
        Some(arg) if arg == &FindWindow::arg_name() => run_window::<FindWindow>()?,
        _ => run_window::<App>()?,
    }

    Ok(())
}

fn run_window<T: AppWindow + 'static>() -> anyhow::Result<()> {
    eframe::run_native(
        T::window_title(),
        T::window_options(),
        Box::new(|_cc| Ok(Box::new(T::default()))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run native window: {:?}", e))?;
    Ok(())
}

impl AppWindow for App {
    fn window_title() -> &'static str {
        "SMDB Companion"
    }
    fn window_options() -> eframe::NativeOptions {
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([750.0, 750.0])
                .with_min_inner_size([620.0, 620.0])
                .with_icon(
                    // NOTE: Adding an icon is optional
                    eframe::icon_data::from_png_bytes(
                        &include_bytes!("../assets/icon-1024.png")[..],
                    )
                    .expect("Failed to load icon"),
                ),
            ..Default::default()
        }
    }
}
