use crate::audio::{CandleAudioProcessor, SessionInfo, TranscriptionConfig};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State, Manager};
use serde::{Deserialize, Serialize};

pub struct VoiceState {
    pub processor: Arc<Mutex<CandleAudioProcessor>>,
}

impl VoiceState {
    pub fn new(app_handle: AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        println!("🔧 [VoiceState] Creating new VoiceState...");
        
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("Failed to get app data directory");
        
        println!("🔧 [VoiceState] App data directory: {:?}", app_data_dir);

        println!("🔧 [VoiceState] Creating CandleAudioProcessor...");
        let processor = Arc::new(Mutex::new(CandleAudioProcessor::new(app_data_dir)?));
        println!("🔧 [VoiceState] VoiceState created successfully");
        
        Ok(Self { processor })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptionStatus {
    pub initialized: bool,
    pub recording: bool,
    pub ready: bool,
    pub session_info: Option<SessionInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStatus {
    pub variant: String,
    pub device: String,
    pub cache_size_mb: f64,
}

/// Ensure the transcription service is initialized
#[tauri::command]
pub async fn ensure_initialized(state: State<'_, VoiceState>) -> Result<bool, String> {
    println!("🔧 [voice.rs] ensure_initialized called");
    let processor = state.processor.lock().unwrap();
    
    let is_initialized = processor.is_transcription_initialized();
    println!("🔧 [voice.rs] Current initialization status: {}", is_initialized);
    
    // If not initialized, try to initialize the transcription service
    if !is_initialized {
        println!("🔧 [voice.rs] Service not initialized, returning false");
        // We need to trigger initialization by accessing the service
        // This will happen automatically when start_recording is called
        // For now, just return the current state
        Ok(false)
    } else {
        println!("🔧 [voice.rs] Service already initialized, returning true");
        Ok(true)
    }
}

/// Start real-time recording with transcription
#[tauri::command]
pub async fn start_recording(
    app_handle: AppHandle,
    state: State<'_, VoiceState>,
) -> Result<String, String> {
    println!("🔧 [voice.rs] start_recording called");
    let mut processor = state.processor.lock().unwrap();
    
    let is_initialized = processor.is_transcription_initialized();
    println!("🔧 [voice.rs] Transcription initialized: {}", is_initialized);
    
    if !is_initialized {
        println!("🔧 [voice.rs] Cannot start recording - service not initialized");
        return Err("Transcription service not initialized. Please wait for initialization to complete.".to_string());
    }

    println!("🔧 [voice.rs] Starting realtime processing...");
    processor
        .start_realtime_processing(app_handle)
        .map_err(|e| {
            println!("🔧 [voice.rs] Failed to start realtime processing: {}", e);
            format!("Failed to start recording: {}", e)
        })?;

    println!("🔧 [voice.rs] Recording started successfully");
    Ok("Recording started with Candle transcription".to_string())
}

/// Stop recording
#[tauri::command]
pub async fn stop_recording(
    app_handle: AppHandle,
    state: State<'_, VoiceState>,
) -> Result<String, String> {
    let mut processor = state.processor.lock().unwrap();
    processor
        .stop_processing(app_handle)
        .map_err(|e| format!("Failed to stop recording: {}", e))
}

/// Check if currently recording
#[tauri::command]
pub async fn is_recording(state: State<'_, VoiceState>) -> Result<bool, String> {
    let processor = state.processor.lock().unwrap();
    Ok(processor.is_processing())
}

/// Check if transcription is ready
#[tauri::command]
pub async fn is_ready(state: State<'_, VoiceState>) -> Result<bool, String> {
    let processor = state.processor.lock().unwrap();
    Ok(processor.is_transcription_initialized())
}

/// Get comprehensive transcription status
#[tauri::command]
pub async fn get_status(state: State<'_, VoiceState>) -> Result<TranscriptionStatus, String> {
    let processor = state.processor.lock().unwrap();
    
    let initialized = processor.is_transcription_initialized();
    let recording = processor.is_processing();
    let session_info = processor.get_session_info();
    
    let status = TranscriptionStatus {
        initialized,
        recording,
        ready: initialized && !recording,
        session_info,
    };
    
    Ok(status)
}

/// Update transcription configuration
#[tauri::command]
pub async fn update_transcription_config(
    state: State<'_, VoiceState>,
    config: TranscriptionConfig,
) -> Result<String, String> {
    let processor = state.processor.lock().unwrap();
    processor
        .update_config(config)
        .map_err(|e| format!("Failed to update config: {}", e))?;
    
    Ok("Configuration updated".to_string())
}

/// Get current session information
#[tauri::command]
pub async fn get_session_info(state: State<'_, VoiceState>) -> Result<Option<SessionInfo>, String> {
    let processor = state.processor.lock().unwrap();
    Ok(processor.get_session_info())
}

/// Get model information and cache status
#[tauri::command]
pub async fn get_model_status(state: State<'_, VoiceState>) -> Result<Option<ModelStatus>, String> {
    let processor = state.processor.lock().unwrap();
    
    if let Some(session_info) = processor.get_session_info() {
        // In a real implementation, you'd get this from the model manager
        // For now, return mock data
        Ok(Some(ModelStatus {
            variant: "distil-small.en".to_string(),
            device: "Metal".to_string(),
            cache_size_mb: 166.0,
        }))
    } else {
        Ok(None)
    }
}

/// Clear model cache (useful for troubleshooting)
#[tauri::command]
pub async fn clear_model_cache(
    _state: State<'_, VoiceState>,
) -> Result<String, String> {
    // In a real implementation, you'd access the model manager through the processor
    // For now, just return a message
    Ok("Model cache clearing not implemented yet".to_string())
}

/// Test transcription with a short audio sample (for debugging)
#[tauri::command]
pub async fn test_transcription(
    state: State<'_, VoiceState>,
) -> Result<String, String> {
    let processor = state.processor.lock().unwrap();
    
    if !processor.is_transcription_initialized() {
        return Err("Transcription service not initialized".to_string());
    }
    
    // Generate a simple test tone (1 second of 440Hz sine wave)
    let sample_rate = 16000;
    let duration = 1.0;
    let frequency = 440.0;
    let mut test_audio = Vec::with_capacity(sample_rate);
    
    for i in 0..sample_rate {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.1;
        test_audio.push(sample);
    }
    
    // Note: This would need to be implemented in the processor
    // For now, just return a test message
    Ok("Test transcription completed (mock)".to_string())
}

/// Get transcription statistics
#[tauri::command]
pub async fn get_transcription_stats(
    state: State<'_, VoiceState>,
) -> Result<Option<serde_json::Value>, String> {
    let processor = state.processor.lock().unwrap();
    
    if let Some(session_info) = processor.get_session_info() {
        let stats = serde_json::json!({
            "session_id": session_info.session_id,
            "segment_count": session_info.segment_count,
            "total_chunks_processed": session_info.stats.total_chunks_processed,
            "total_processing_time_ms": session_info.stats.total_processing_time.as_millis(),
            "average_confidence": session_info.stats.average_confidence,
            "corrections_made": session_info.stats.corrections_made,
        });
        Ok(Some(stats))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Note: These tests would need a mock AppHandle to run properly
    // For now, they're just structural tests
    
    #[test]
    fn test_transcription_status_serialization() {
        let status = TranscriptionStatus {
            initialized: true,
            recording: false,
            ready: true,
            session_info: None,
        };
        
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("initialized"));
        assert!(json.contains("recording"));
        assert!(json.contains("ready"));
    }

    #[test]
    fn test_model_status_serialization() {
        let status = ModelStatus {
            variant: "distil-small.en".to_string(),
            device: "Metal".to_string(),
            cache_size_mb: 166.0,
        };
        
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("distil-small.en"));
        assert!(json.contains("Metal"));
    }
}
