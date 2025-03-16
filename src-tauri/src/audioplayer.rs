// Replace your current AudioState implementation with this:

use once_cell::sync::Lazy;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

// Thread-safe audio commands
enum AudioCommand {
    Play(String),
    Stop,
    Pause,
    Resume,
}

// Thread-safe responses
enum AudioResponse {
    Success(String),
    Error(String),
}

struct AudioManager {
    sender: Sender<(AudioCommand, Sender<AudioResponse>)>,
}

impl AudioManager {
    fn new() -> Self {
        println!("Initializing audio manager");
        let (tx, rx) = mpsc::channel::<(AudioCommand, Sender<AudioResponse>)>();

        // Spawn dedicated audio thread
        thread::spawn(move || {
            // Audio state stays in this thread
            match OutputStream::try_default() {
                Ok((stream, stream_handle)) => {
                    println!("✅ Audio device initialized successfully");
                    let mut current_sink: Option<Sink> = None;

                    // Process audio commands
                    while let Ok((command, response_tx)) = rx.recv() {
                        match command {
                            AudioCommand::Play(path) => {
                                // Stop any existing playback
                                if let Some(sink) = &current_sink {
                                    sink.stop();
                                }

                                // Open and play new file
                                match File::open(&path) {
                                    Ok(file) => {
                                        let buf_reader = BufReader::new(file);
                                        match Decoder::new(buf_reader) {
                                            Ok(source) => match Sink::try_new(&stream_handle) {
                                                Ok(sink) => {
                                                    sink.append(source);
                                                    sink.play();
                                                    current_sink = Some(sink);
                                                    let _ =
                                                        response_tx.send(AudioResponse::Success(
                                                            format!("Playing audio: {}", path),
                                                        ));
                                                }
                                                Err(e) => {
                                                    let _ = response_tx.send(AudioResponse::Error(
                                                        format!("Failed to create sink: {}", e),
                                                    ));
                                                }
                                            },
                                            Err(e) => {
                                                let _ = response_tx.send(AudioResponse::Error(
                                                    format!("Failed to decode audio: {}", e),
                                                ));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = response_tx.send(AudioResponse::Error(format!(
                                            "Failed to open file: {}",
                                            e
                                        )));
                                    }
                                }
                            }
                            AudioCommand::Stop => {
                                if let Some(sink) = &current_sink {
                                    sink.stop();
                                    current_sink = None;
                                    let _ = response_tx
                                        .send(AudioResponse::Success("Audio stopped".to_string()));
                                } else {
                                    let _ = response_tx.send(AudioResponse::Success(
                                        "No audio is playing".to_string(),
                                    ));
                                }
                            }
                            AudioCommand::Pause => {
                                if let Some(sink) = &current_sink {
                                    sink.pause();
                                    let _ = response_tx
                                        .send(AudioResponse::Success("Audio paused".to_string()));
                                } else {
                                    let _ = response_tx.send(AudioResponse::Success(
                                        "No audio is playing".to_string(),
                                    ));
                                }
                            }
                            AudioCommand::Resume => {
                                if let Some(sink) = &current_sink {
                                    sink.play();
                                    let _ = response_tx
                                        .send(AudioResponse::Success("Audio resumed".to_string()));
                                } else {
                                    let _ = response_tx.send(AudioResponse::Success(
                                        "No audio is playing".to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to initialize audio device: {}", e);
                    // Just let the thread end if we can't initialize audio
                }
            }
        });

        Self { sender: tx }
    }

    fn send_command(&self, command: AudioCommand) -> Result<String, String> {
        let (tx, rx) = mpsc::channel();
        self.sender
            .send((command, tx))
            .map_err(|_| "Audio thread disconnected")?;
        match rx.recv().map_err(|_| "No response from audio thread")? {
            AudioResponse::Success(msg) => Ok(msg),
            AudioResponse::Error(err) => Err(err),
        }
    }
}

// Thread-safe global audio manager
static AUDIO_MANAGER: Lazy<AudioManager> = Lazy::new(|| {
    println!("Initializing audio manager");
    AudioManager::new()
});

// Thread-safe command handlers
#[tauri::command]
pub fn play_audio(path: String) -> Result<String, String> {
    AUDIO_MANAGER.send_command(AudioCommand::Play(path))
}

#[tauri::command]
pub fn stop_audio() -> Result<String, String> {
    AUDIO_MANAGER.send_command(AudioCommand::Stop)
}

#[tauri::command]
pub fn pause_audio() -> Result<String, String> {
    AUDIO_MANAGER.send_command(AudioCommand::Pause)
}

#[tauri::command]
pub fn resume_audio() -> Result<String, String> {
    AUDIO_MANAGER.send_command(AudioCommand::Resume)
}

/// Explicitly initializes the audio system.
/// This ensures the audio thread is created during application startup
/// rather than waiting for the first audio operation.
pub fn init_audio_system() {
    println!("Explicitly initializing audio system...");

    // Access the AUDIO_MANAGER to force initialization
    let result = AUDIO_MANAGER.send_command(AudioCommand::Stop);

    match result {
        Ok(_) => println!("Audio system initialized successfully"),
        Err(e) => println!("Audio system initialization warning: {}", e),
    }
}
