use crate::audio::{
    AudioCapture, CandleWhisperService, TranscriptionConfig, TranscriptionError,
    TranscriptionResult, TranscriptionStream, TranscriptionUpdate,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};
use tracing::{error, info, warn};

/// Audio processor using Candle + Distil-Whisper
pub struct CandleAudioProcessor {
    audio_capture: Arc<Mutex<AudioCapture>>,
    transcription_service: Arc<Mutex<CandleWhisperService>>,
    is_processing: Arc<Mutex<bool>>,
    is_capturing: Arc<Mutex<bool>>,
    processing_handle: Option<thread::JoinHandle<()>>,
    capture_handle: Option<thread::JoinHandle<()>>,
}

impl CandleAudioProcessor {
    /// Create a new audio processor
    pub fn new(app_data_dir: std::path::PathBuf) -> TranscriptionResult<Self> {
        let audio_capture = Arc::new(Mutex::new(AudioCapture::new()?));
        
        // Create transcription service with default config
        let config = TranscriptionConfig::default();
        let transcription_service = Arc::new(Mutex::new(
            CandleWhisperService::new(app_data_dir, config)?
        ));

        // Initialize transcription service in a separate thread to avoid blocking
        let service_clone = Arc::clone(&transcription_service);
        thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");
            
            rt.block_on(async {
                let mut service = service_clone.lock().unwrap();
                if let Err(e) = service.initialize().await {
                    error!("Failed to initialize transcription service: {}", e);
                } else {
                    info!("Transcription service initialized successfully");
                }
            });
        });

        Ok(Self {
            audio_capture,
            transcription_service,
            is_processing: Arc::new(Mutex::new(false)),
            is_capturing: Arc::new(Mutex::new(false)),
            processing_handle: None,
            capture_handle: None,
        })
    }

    /// Start audio capture and buffering
    pub fn start_audio_capture(&mut self) -> TranscriptionResult<()> {
        if *self.is_capturing.lock().unwrap() {
            return Ok(()); // Already capturing
        }

        *self.is_capturing.lock().unwrap() = true;

        let audio_capture_clone = Arc::clone(&self.audio_capture);
        let transcription_service_clone = Arc::clone(&self.transcription_service);
        let is_capturing = Arc::clone(&self.is_capturing);

        let handle = thread::spawn(move || {
            let audio_receiver = {
                let mut capture = audio_capture_clone.lock().unwrap();
                match capture.start_recording() {
                    Ok(receiver) => receiver,
                    Err(e) => {
                        error!("Failed to start audio capture: {}", e);
                        *is_capturing.lock().unwrap() = false;
                        return;
                    }
                }
            };

            info!("Audio capture thread started");

            while *is_capturing.lock().unwrap() {
                match audio_receiver.try_recv() {
                    Ok(audio_chunk) => {
                        let service = transcription_service_clone.lock().unwrap();
                        service.add_audio_to_buffer(audio_chunk);
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => {
                        thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        info!("Audio capture disconnected, stopping buffering thread");
                        break;
                    }
                }
            }

            // Stop audio capture when exiting
            {
                let mut capture = audio_capture_clone.lock().unwrap();
                let _ = capture.stop_recording();
            }

            info!("Audio capture thread ended");
        });

        self.capture_handle = Some(handle);
        Ok(())
    }

    /// Stop audio capture
    pub fn stop_audio_capture(&mut self) -> TranscriptionResult<()> {
        *self.is_capturing.lock().unwrap() = false;

        // Wait for capture thread to finish
        if let Some(handle) = self.capture_handle.take() {
            handle.join().expect("Failed to join capture thread");
        }

        Ok(())
    }

    /// Start real-time transcription processing
    pub fn start_realtime_processing(&mut self, app_handle: AppHandle) -> TranscriptionResult<()> {
        if *self.is_processing.lock().unwrap() {
            return Err(TranscriptionError::ConfigError("Already processing".to_string()));
        }

        // Start audio capture if not already started
        self.start_audio_capture()?;

        *self.is_processing.lock().unwrap() = true;

        // Clone necessary data for the processing thread
        let transcription_service = Arc::clone(&self.transcription_service);
        let is_processing = Arc::clone(&self.is_processing);
        let app_handle_clone = app_handle.clone();

        // Spawn processing thread
        let handle = thread::spawn(move || {
            info!("Transcription processing thread started");

            // Create async runtime for this thread
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            while *is_processing.lock().unwrap() {
                let audio_buffer = {
                    let service = transcription_service.lock().unwrap();
                    service.get_context_audio(2.0) // Get last 2 seconds of audio
                };

                if audio_buffer.is_empty() {
                    thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }

                // Process audio chunk
                let result = rt.block_on(async {
                    let mut service = transcription_service.lock().unwrap();
                    if service.is_initialized() {
                        service.process_audio_chunk(&audio_buffer).await
                    } else {
                        Ok(None)
                    }
                });

                match result {
                    Ok(Some(stream)) => {
                        // Emit transcription stream event
                        if let Err(e) = app_handle_clone.emit("transcription_stream", &stream) {
                            error!("Failed to emit transcription_stream event: {}", e);
                        }

                        // For backward compatibility, also emit the old events
                        if !stream.segments.is_empty() {
                            let latest_segment = &stream.segments[stream.segments.len() - 1];
                            match latest_segment.status {
                                crate::audio::SegmentStatus::Provisional => {
                                    let _ = app_handle_clone.emit("transcription_partial", &latest_segment.text);
                                }
                                crate::audio::SegmentStatus::Stable | crate::audio::SegmentStatus::Final => {
                                    let _ = app_handle_clone.emit("transcription_final", &latest_segment.text);
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        // No transcription result, continue
                    }
                    Err(e) => {
                        error!("Transcription error: {}", e);
                        let _ = app_handle_clone.emit("transcription_error", format!("Error: {}", e));
                    }
                }

                // Sleep to avoid overwhelming the system
                thread::sleep(std::time::Duration::from_millis(500));
            }

            // Finalize session when processing stops
            let final_text = {
                let mut service = transcription_service.lock().unwrap();
                service.finalize_session().unwrap_or_default()
            };

            if !final_text.trim().is_empty() {
                info!("Emitting final transcription: '{}'", final_text.trim());
                let _ = app_handle_clone.emit("transcription_complete", final_text.trim());
            }

            info!("Transcription processing thread ended");
        });

        self.processing_handle = Some(handle);

        // Emit recording started event
        app_handle.emit("recording_started", ())?;

        Ok(())
    }

    /// Stop transcription processing
    pub fn stop_processing(&mut self, app_handle: AppHandle) -> TranscriptionResult<String> {
        *self.is_processing.lock().unwrap() = false;

        // Wait for processing thread to finish
        if let Some(handle) = self.processing_handle.take() {
            handle.join().expect("Failed to join processing thread");
        }

        // Stop audio capture
        self.stop_audio_capture()?;

        // Clear audio buffer and start new session
        {
            let mut service = self.transcription_service.lock().unwrap();
            service.start_new_session();
        }

        // Emit recording stopped event
        app_handle.emit("recording_stopped", ())?;

        Ok("Recording stopped".to_string())
    }

    /// Check if currently processing
    pub fn is_processing(&self) -> bool {
        *self.is_processing.lock().unwrap()
    }

    /// Check if transcription service is initialized
    pub fn is_transcription_initialized(&self) -> bool {
        let service = self.transcription_service.lock().unwrap();
        service.is_initialized()
    }

    /// Get current session information
    pub fn get_session_info(&self) -> Option<SessionInfo> {
        let service = self.transcription_service.lock().unwrap();
        if service.is_initialized() {
            Some(SessionInfo {
                session_id: service.session_id().to_string(),
                segment_count: service.segments().len(),
                stats: service.get_stats(),
            })
        } else {
            None
        }
    }

    /// Update transcription configuration
    pub fn update_config(&self, config: TranscriptionConfig) -> TranscriptionResult<()> {
        // Note: In a full implementation, you'd want to recreate the service with new config
        // For now, we'll just log that config update was requested
        info!("Config update requested: {:?}", config);
        warn!("Config updates require service restart - not implemented yet");
        Ok(())
    }
}

impl Drop for CandleAudioProcessor {
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

/// Session information for debugging and monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub segment_count: usize,
    pub stats: crate::audio::candle_whisper::ProcessingStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_processor_creation() {
        let temp_dir = TempDir::new().unwrap();
        let processor = CandleAudioProcessor::new(temp_dir.path().to_path_buf());
        assert!(processor.is_ok());
    }

    #[test]
    fn test_processor_state() {
        let temp_dir = TempDir::new().unwrap();
        let processor = CandleAudioProcessor::new(temp_dir.path().to_path_buf()).unwrap();
        
        assert!(!processor.is_processing());
        // Note: transcription service may not be initialized immediately due to async loading
    }
}
