// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    println!(
        "Current Working Dir: {}",
        std::env::current_dir().unwrap().display()
    );

    smdbc_lib::run()
}
