#![warn(clippy::all, rust_2018_idioms)]
#![allow(non_snake_case)]

pub mod app;
pub use app::App;

pub mod assets;
pub mod find_replace;
// pub use assets::*;

pub mod duplicates;
// pub use duplicates::*;

pub mod prelude;
// pub use crate::prelude::*;

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
