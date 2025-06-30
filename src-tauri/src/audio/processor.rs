use super::{AudioCapture, WhisperService};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

pub struct AudioProcessor {
    audio_capture: Arc<Mutex<AudioCapture>>,
    whisper_service: Arc<Mutex<WhisperService>>,
    is_processing: Arc<Mutex<bool>>,
    processing_handle: Option<thread::JoinHandle<()>>,
}

impl AudioProcessor {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let audio_capture = Arc::new(Mutex::new(AudioCapture::new()?));
        let whisper_service = Arc::new(Mutex::new(WhisperService::new()));
        
        Ok(Self {
            audio_capture,
            whisper_service,
            is_processing: Arc::new(Mutex::new(false)),
            processing_handle: None,
        })
    }

    pub async fn initialize(&mut self, app_data_dir: std::path::PathBuf) -> Result<(), String> {
        // Clone the whisper service to avoid holding the lock across await
        let whisper_service = Arc::clone(&self.whisper_service);
        
        // Initialize in a blocking task to avoid Send issues
        let result = tokio::task::spawn_blocking(move || {
            let mut whisper = whisper_service.lock().unwrap();
            // We need to use a runtime for the async call
            let rt = tokio::runtime::Handle::current();
            rt.block_on(whisper.initialize(app_data_dir))
                .map_err(|e| e.to_string())
        }).await
        .map_err(|e| e.to_string())?;
        
        result?;
        Ok(())
    }

    pub fn start_realtime_processing(&mut self, app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        if *self.is_processing.lock().unwrap() {
            return Err("Already processing".into());
        }

        // Start audio capture
        let audio_receiver = {
            let mut capture = self.audio_capture.lock().unwrap();
            capture.start_recording()?
        };

        *self.is_processing.lock().unwrap() = true;

        // Clone necessary data for the processing thread
        let whisper_service = Arc::clone(&self.whisper_service);
        let is_processing = Arc::clone(&self.is_processing);
        let app_handle_clone = app_handle.clone();

        // Spawn processing thread
        let handle = thread::spawn(move || {
            let mut accumulated_text = String::new();
            let mut chunk_count = 0;
            let mut last_transcription = String::new();
            let mut transcription_count = 0;
            let mut audio_buffer = Vec::new();

            println!("Audio processing thread started");

            while *is_processing.lock().unwrap() {
                match audio_receiver.try_recv() {
                    Ok(audio_chunk) => {
                        chunk_count += 1;
                        println!("Received audio chunk #{}: {} samples", chunk_count, audio_chunk.len());
                        
                        // Add audio to buffer for context
                        {
                            let whisper = whisper_service.lock().unwrap();
                            whisper.add_audio_to_buffer(audio_chunk.clone());
                        }

                        // Accumulate audio chunks and process more frequently for lower latency
                        audio_buffer.extend_from_slice(&audio_chunk);
                        
                        // Process every 2 chunks (about 2 seconds of audio) for faster response
                        // GPU acceleration should handle the increased frequency better
                        if chunk_count % 2 == 0 {
                            println!("Processing accumulated audio: {} samples", audio_buffer.len());
                            
                            if let Ok(whisper) = whisper_service.lock() {
                                if whisper.is_initialized() {
                                    // Just transcribe the current audio buffer without context for now
                                    match whisper.transcribe_chunk(&audio_buffer) {
                                        Ok(text) => {
                                            transcription_count += 1;
                                            println!("Transcription #{}: '{}'", transcription_count, text);
                                            
                                            if !text.is_empty() {
                                                // Check for duplicates and similar text
                                                let is_duplicate = text == last_transcription || 
                                                    (text.len() > 3 && last_transcription.contains(&text)) ||
                                                    (last_transcription.len() > 3 && text.contains(&last_transcription));
                                                
                                                if !is_duplicate {
                                                    println!("Emitting new transcription: '{}'", text);
                                                    // Emit partial transcription event
                                                    let _ = app_handle_clone.emit("transcription_partial", &text);
                                                    
                                                    // Update accumulated text
                                                    if !accumulated_text.contains(&text) {
                                                        if !accumulated_text.is_empty() {
                                                            accumulated_text.push(' ');
                                                        }
                                                        accumulated_text.push_str(&text);
                                                    }
                                                    
                                                    last_transcription = text;
                                                } else {
                                                    println!("Skipping duplicate transcription: '{}'", text);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("Transcription error: {}", e);
                                            let _ = app_handle_clone.emit("transcription_error", format!("Error: {}", e));
                                        }
                                    }
                                }
                            }
                            
                            // Clear the audio buffer after processing
                            audio_buffer.clear();
                        }
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => {
                        // No audio data available, sleep briefly
                        thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        // Audio capture stopped
                        println!("Audio capture disconnected, stopping processing");
                        break;
                    }
                }
            }

            // Emit final transcription when processing stops
            if !accumulated_text.trim().is_empty() {
                println!("Emitting final transcription: '{}'", accumulated_text.trim());
                let _ = app_handle_clone.emit("transcription_final", accumulated_text.trim());
            }
            
            println!("Audio processing thread ended. Processed {} chunks, {} transcriptions", chunk_count, transcription_count);
        });

        self.processing_handle = Some(handle);

        // Emit recording started event
        app_handle.emit("recording_started", ())?;

        Ok(())
    }

    pub fn stop_processing(&mut self, app_handle: AppHandle) -> Result<String, Box<dyn std::error::Error>> {
        *self.is_processing.lock().unwrap() = false;

        // Stop audio capture
        {
            let mut capture = self.audio_capture.lock().unwrap();
            capture.stop_recording()?;
        }

        // Wait for processing thread to finish
        if let Some(handle) = self.processing_handle.take() {
            let _ = handle.join();
        }

        // Clear audio buffer
        {
            let whisper = self.whisper_service.lock().unwrap();
            whisper.clear_buffer();
        }

        // Emit recording stopped event
        app_handle.emit("recording_stopped", ())?;

        Ok("Recording stopped".to_string())
    }

    pub fn is_processing(&self) -> bool {
        *self.is_processing.lock().unwrap()
    }

    pub fn is_whisper_initialized(&self) -> bool {
        let whisper = self.whisper_service.lock().unwrap();
        whisper.is_initialized()
    }
}

impl Drop for AudioProcessor {
    fn drop(&mut self) {
        *self.is_processing.lock().unwrap() = false;
        if let Some(handle) = self.processing_handle.take() {
            let _ = handle.join();
        }
    }
}
