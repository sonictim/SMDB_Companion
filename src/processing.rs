#![allow(non_snake_case)]
use crate::prelude::*;

use clipboard::{ClipboardContext, ClipboardProvider};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::error::Error;

pub fn generate_license_key(username: &str, email: &str) -> String {
    let salt = "Valhalla Delay";
    let mut hasher = Sha256::new();
    hasher.update(format!("{}{}{}", username, email, salt).as_bytes());
    let hash = hasher.finalize();
    hex::encode_upper(hash)
}

pub fn copy_to_clipboard(text: String) {
    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();

    ctx.set_contents(text).unwrap();
}

// pub fn remove_all_spaces(input: &str) -> String {
//     input.chars().filter(|c| !c.is_whitespace()).collect()
// }

pub async fn fetch_latest_version() -> Result<String, Box<dyn Error>> {
    let file_id = "1C8jyVjkMgeglYK-FnmTuoRqwf5Nd6PGG";
    let download_url = format!("https://drive.google.com/uc?export=download&id={}", file_id);
    let client = Client::new();

    let response = client.get(&download_url).send().await?;

    if response.status().is_success() {
        let content = response.text().await?;
        Ok(content.trim().to_string())
    } else {
        Err(format!("Failed to retrieve the file: {}", response.status()).into())
    }
}

pub fn open_download_url() {
    let url = r#"https://drive.google.com/open?id=1qdGqoUMqq_xCrbA6IxUTYliZUmd3Tn3i&usp=drive_fs"#;
    let _ = webbrowser::open(url).is_ok();
}

static FILENAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(?P<base>.+?)(?:\.(?:\d+|M))*$").unwrap());

fn get_root_filename(filename: &str, ignore_extension: bool) -> Option<String> {
    let path = Path::new(filename);
    let mut name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    // Use the regex to capture the base name
    if let Some(caps) = FILENAME_REGEX.captures(&name) {
        name = caps["base"].to_string();
    } else {
        println!("{} Did not match Regex", filename);
    }

    if ignore_extension {
        return Some(name);
    }

    // Reattach the extension if it's not being ignored
    Some(format!("{name}.{extension}"))
}

pub async fn get_audio_file_types(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query("SELECT DISTINCT AudioFileType FROM justinmetadata")
        .fetch_all(pool)
        .await?;

    let audio_file_types: Vec<String> = rows
        .iter()
        .filter_map(|row| row.get::<Option<String>, _>("AudioFileType")) // Access the column directly
        .collect();

    Ok(audio_file_types)
}
