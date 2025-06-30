use crate::audio::AudioProcessor;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State, Manager};

pub struct VoiceState {
    pub processor: Arc<Mutex<AudioProcessor>>,
}

impl VoiceState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let processor = Arc::new(Mutex::new(AudioProcessor::new()?));
        Ok(Self { processor })
    }
}

#[tauri::command]
pub async fn initialize_whisper(
    app_handle: AppHandle,
    state: State<'_, VoiceState>,
) -> Result<String, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // Clone the processor Arc to avoid holding the lock across await
    let processor = Arc::clone(&state.processor);
    
    // Use spawn_blocking to handle the async initialization
    tokio::task::spawn_blocking(move || {
        let mut proc = processor.lock().unwrap();
        // We need to use a runtime for the async call
        let rt = tokio::runtime::Handle::current();
        rt.block_on(proc.initialize(app_data_dir))
    }).await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| format!("Failed to initialize Whisper: {}", e))?;

    Ok("Whisper initialized successfully".to_string())
}

#[tauri::command]
pub async fn start_realtime_recording(
    app_handle: AppHandle,
    state: State<'_, VoiceState>,
) -> Result<String, String> {
    let mut processor = state.processor.lock().unwrap();
    
    if !processor.is_whisper_initialized() {
        return Err("Whisper not initialized. Please call initialize_whisper first.".to_string());
    }

    processor
        .start_realtime_processing(app_handle)
        .map_err(|e| format!("Failed to start recording: {}", e))?;

    Ok("Recording started".to_string())
}

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

#[tauri::command]
pub async fn is_recording(state: State<'_, VoiceState>) -> Result<bool, String> {
    let processor = state.processor.lock().unwrap();
    Ok(processor.is_processing())
}

#[tauri::command]
pub async fn is_whisper_ready(state: State<'_, VoiceState>) -> Result<bool, String> {
    let processor = state.processor.lock().unwrap();
    Ok(processor.is_whisper_initialized())
}

#[tauri::command]
pub async fn get_whisper_status(state: State<'_, VoiceState>) -> Result<serde_json::Value, String> {
    let processor = state.processor.lock().unwrap();
    
    let status = serde_json::json!({
        "initialized": processor.is_whisper_initialized(),
        "recording": processor.is_processing(),
        "ready": processor.is_whisper_initialized() && !processor.is_processing()
    });
    
    Ok(status)
}
