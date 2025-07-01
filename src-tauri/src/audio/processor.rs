use super::{AudioCapture, WhisperService};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

pub struct AudioProcessor {
    audio_capture: Arc<Mutex<AudioCapture>>,
    whisper_service: Arc<Mutex<WhisperService>>,
    is_processing: Arc<Mutex<bool>>,
    is_capturing: Arc<Mutex<bool>>,
    processing_handle: Option<thread::JoinHandle<()>>,
    capture_handle: Option<thread::JoinHandle<()>>,
}

impl AudioProcessor {
    pub fn new(app_data_dir: std::path::PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let audio_capture = Arc::new(Mutex::new(AudioCapture::new()?));
        let whisper_service = Arc::new(Mutex::new(WhisperService::new()));

        // Initialize Whisper service in a separate thread to avoid blocking
        let whisper_service_clone = Arc::clone(&whisper_service);
        thread::spawn(move || {
            let mut whisper = whisper_service_clone.lock().unwrap();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(whisper.initialize(app_data_dir))
                .expect("Failed to initialize whisper");
        });

        Ok(Self {
            audio_capture,
            whisper_service,
            is_processing: Arc::new(Mutex::new(false)),
            is_capturing: Arc::new(Mutex::new(false)),
            processing_handle: None,
            capture_handle: None,
        })
    }

    pub fn start_audio_capture(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if *self.is_capturing.lock().unwrap() {
            return Ok(()); // Already capturing
        }

        *self.is_capturing.lock().unwrap() = true;

        // Start audio capture and buffering
        let audio_capture_clone = Arc::clone(&self.audio_capture);
        let whisper_service_clone = Arc::clone(&self.whisper_service);
        let is_capturing = Arc::clone(&self.is_capturing);

        let handle = thread::spawn(move || {
            let audio_receiver = {
                let mut capture = audio_capture_clone.lock().unwrap();
                match capture.start_recording() {
                    Ok(receiver) => receiver,
                    Err(e) => {
                        eprintln!("Failed to start audio capture: {}", e);
                        *is_capturing.lock().unwrap() = false;
                        return;
                    }
                }
            };

            println!("Audio capture thread started");

            while *is_capturing.lock().unwrap() {
                match audio_receiver.try_recv() {
                    Ok(audio_chunk) => {
                        let whisper = whisper_service_clone.lock().unwrap();
                        whisper.add_audio_to_buffer(audio_chunk);
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => {
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        println!("Audio capture disconnected, stopping buffering thread");
                        break;
                    }
                }
            }

            // Stop audio capture when exiting
            {
                let mut capture = audio_capture_clone.lock().unwrap();
                let _ = capture.stop_recording();
            }

            println!("Audio capture thread ended");
        });

        self.capture_handle = Some(handle);
        Ok(())
    }

    pub fn stop_audio_capture(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        *self.is_capturing.lock().unwrap() = false;

        // Wait for capture thread to finish
        if let Some(handle) = self.capture_handle.take() {
            handle.join().expect("Failed to join capture thread");
        }

        Ok(())
    }

    pub fn start_realtime_processing(&mut self, app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        if *self.is_processing.lock().unwrap() {
            return Err("Already processing".into());
        }

        // Start audio capture if not already started
        self.start_audio_capture()?;

        *self.is_processing.lock().unwrap() = true;

        // Clone necessary data for the processing thread
        let whisper_service = Arc::clone(&self.whisper_service);
        let is_processing = Arc::clone(&self.is_processing);
        let app_handle_clone = app_handle.clone();

        // Spawn processing thread
        let handle = thread::spawn(move || {
            let mut accumulated_text = String::new();
            let mut last_transcription = String::new();

            println!("Audio processing thread started");

            while *is_processing.lock().unwrap() {
                let audio_buffer = {
                    let whisper = whisper_service.lock().unwrap();
                    whisper.get_context_audio(5.0) // Get last 5 seconds of audio
                };

                if audio_buffer.is_empty() {
                    thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                
                if let Ok(whisper) = whisper_service.lock() {
                    if whisper.is_initialized() {
                        match whisper.transcribe_chunk(&audio_buffer) {
                            Ok(text) => {
                                if !text.is_empty() {
                                    let is_duplicate = text == last_transcription || 
                                        (text.len() > 3 && last_transcription.contains(&text)) ||
                                        (last_transcription.len() > 3 && text.contains(&last_transcription));
                                    
                                    if !is_duplicate {
                                        println!("Emitting new transcription: '{}'", text);
                                        let _ = app_handle_clone.emit("transcription_partial", &text);
                                        
                                        if !accumulated_text.contains(&text) {
                                            if !accumulated_text.is_empty() {
                                                accumulated_text.push(' ');
                                            }
                                            accumulated_text.push_str(&text);
                                        }
                                        
                                        last_transcription = text;
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
                
                // Sleep for a bit to avoid pegging the CPU
                thread::sleep(std::time::Duration::from_millis(500));
            }

            // Emit final transcription when processing stops
            if !accumulated_text.trim().is_empty() {
                println!("Emitting final transcription: '{}'", accumulated_text.trim());
                let _ = app_handle_clone.emit("transcription_final", accumulated_text.trim());
            }
            
            println!("Audio processing thread ended.");
        });

        self.processing_handle = Some(handle);

        // Emit recording started event
        app_handle.emit("recording_started", ())?;

        Ok(())
    }

    pub fn stop_processing(&mut self, app_handle: AppHandle) -> Result<String, Box<dyn std::error::Error>> {
        *self.is_processing.lock().unwrap() = false;

        // Wait for processing thread to finish
        if let Some(handle) = self.processing_handle.take() {
            handle.join().expect("Failed to join processing thread");
        }

        // Stop audio capture
        self.stop_audio_capture()?;

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
        // Stop both processing and capturing
        *self.is_processing.lock().unwrap() = false;
        *self.is_capturing.lock().unwrap() = false;
        
        // Wait for both threads to finish
        if let Some(handle) = self.processing_handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.capture_handle.take() {
            let _ = handle.join();
        }
    }
}
