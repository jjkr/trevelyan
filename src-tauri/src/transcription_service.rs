use crate::audio_capture::AudioCapture;
use crate::whisper_engine::WhisperEngine;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::time::interval;

#[derive(Clone, serde::Serialize)]
struct TranscriptionEvent {
    text: String,
    is_final: bool,
    audio_level: f32,
}

#[derive(Clone, serde::Serialize)]
struct VoiceStatusEvent {
    is_recording: bool,
    audio_level: f32,
}

#[derive(Clone)]
pub struct TranscriptionService {
    audio_capture: Option<AudioCapture>,
    whisper_engine: Arc<Mutex<Option<WhisperEngine>>>,
    app_handle: AppHandle,
    is_running: Arc<Mutex<bool>>,
}

impl TranscriptionService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            audio_capture: None,
            whisper_engine: Arc::new(Mutex::new(None)),
            app_handle,
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        println!("🔧 [transcription_service] Starting transcription service initialization...");

        // Initialize audio capture
        println!("🔧 [transcription_service] Creating AudioCapture...");
        let audio_capture = AudioCapture::new()?;
        self.audio_capture = Some(audio_capture);
        println!("✅ [transcription_service] AudioCapture created successfully");

        // Initialize Whisper engine
        println!("🔧 [transcription_service] Creating WhisperEngine...");
        let whisper_engine = WhisperEngine::new().await?;
        {
            let mut engine_guard = self.whisper_engine.lock().unwrap();
            *engine_guard = Some(whisper_engine);
        }
        println!("✅ [transcription_service] WhisperEngine created successfully");

        println!("✅ [transcription_service] Transcription service initialized successfully");
        Ok(())
    }

    pub async fn start_recording(&self) -> Result<()> {
        {
            let mut is_running = self.is_running.lock().unwrap();
            if *is_running {
                return Ok(()); // Already running
            }
            *is_running = true;
        }

        // Start audio capture
        if let Some(ref capture) = self.audio_capture {
            capture.start_recording()?;
        }

        // Start transcription loop
        self.start_transcription_loop().await;

        Ok(())
    }

    pub async fn stop_recording(&self) -> Result<()> {
        {
            let mut is_running = self.is_running.lock().unwrap();
            *is_running = false;
        }

        // Stop audio capture
        if let Some(ref capture) = self.audio_capture {
            capture.stop_recording()?;
        }

        // Emit final status
        self.emit_voice_status(false, 0.0);

        Ok(())
    }

    async fn start_transcription_loop(&self) {
        let whisper_engine = self.whisper_engine.clone();
        let app_handle = self.app_handle.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(500)); // Run every 500ms
            let mut last_transcription = String::new();
            let mut last_transcription_time = Instant::now();

            while *is_running.lock().unwrap() {
                interval.tick().await;

                // Create a temporary AudioCapture to access global buffer
                let temp_capture = AudioCapture;
                let audio_data = temp_capture.get_buffer_data();
                let audio_level = temp_capture.get_audio_level();
                let is_recording = temp_capture.is_recording();

                // Emit voice status
                let _ = app_handle.emit("voice-status", VoiceStatusEvent {
                    is_recording,
                    audio_level,
                });

                if !is_recording || audio_data.len() < 8000 {
                    // Need at least 0.5 seconds of audio
                    continue;
                }

                // Only transcribe if we have enough new audio or it's been a while
                let should_transcribe = audio_level > 0.01 || // Voice activity detected
                    last_transcription_time.elapsed() > Duration::from_secs(2); // Force update every 2 seconds

                if !should_transcribe {
                    continue;
                }

                // Run transcription
                if let Some(ref mut engine) = *whisper_engine.lock().unwrap() {
                    match engine.transcribe(&audio_data) {
                        Ok(text) => {
                            if !text.is_empty() && text != last_transcription {
                                last_transcription = text.clone();
                                last_transcription_time = Instant::now();

                                let _ = app_handle.emit("transcription", TranscriptionEvent {
                                    text,
                                    is_final: false,
                                    audio_level,
                                });
                            }
                        }
                        Err(e) => {
                            eprintln!("Transcription error: {}", e);
                        }
                    }
                }
            }
        });
    }

    pub fn get_final_transcription(&self) -> Result<String> {
        let audio_data = {
            if let Some(ref capture) = self.audio_capture {
                capture.get_buffer_data()
            } else {
                return Ok(String::new());
            }
        };

        if audio_data.is_empty() {
            return Ok(String::new());
        }

        // Run final transcription on the entire buffer
        if let Some(ref mut engine) = *self.whisper_engine.lock().unwrap() {
            let text = engine.transcribe(&audio_data)?;
            
            // Emit final transcription
            self.app_handle.emit("transcription", TranscriptionEvent {
                text: text.clone(),
                is_final: true,
                audio_level: 0.0,
            }).unwrap_or(());

            Ok(text)
        } else {
            Ok(String::new())
        }
    }

    fn emit_voice_status(&self, is_recording: bool, audio_level: f32) {
        let _ = self.app_handle.emit("voice-status", VoiceStatusEvent {
            is_recording,
            audio_level,
        });
    }

    pub fn is_recording(&self) -> bool {
        if let Some(ref capture) = self.audio_capture {
            capture.is_recording()
        } else {
            false
        }
    }
}
