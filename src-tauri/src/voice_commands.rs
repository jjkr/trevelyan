use crate::transcription_service::TranscriptionService;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};
use log::{info, debug, error};

pub type TranscriptionServiceState = Arc<Mutex<Option<TranscriptionService>>>;

#[tauri::command]
pub async fn initialize_voice_service(
    app_handle: AppHandle,
    service_state: State<'_, TranscriptionServiceState>,
) -> Result<(), String> {
    info!("Starting voice service initialization...");
    
    let mut service = TranscriptionService::new(app_handle);
    debug!("TranscriptionService created");
    
    match service.initialize().await {
        Ok(()) => {
            debug!("Service initialization successful, storing in state...");
            let mut state = service_state.lock().unwrap();
            *state = Some(service);
            info!("Voice service initialized and stored successfully");
            Ok(())
        }
        Err(e) => {
            error!("Failed to initialize voice service: {}", e);
            error!("Error details: {:?}", e);
            Err(format!("Failed to initialize voice service: {}", e))
        }
    }
}

#[tauri::command]
pub async fn start_voice_recording(
    service_state: State<'_, TranscriptionServiceState>,
) -> Result<(), String> {
    info!("Starting voice recording...");
    
    // Clone the service to avoid holding the lock across await
    let service = {
        debug!("Acquiring service state lock...");
        let service_guard = service_state.lock().unwrap();
        debug!("Lock acquired, cloning service...");
        service_guard.clone()
    };
    
    if let Some(service) = service {
        debug!("Service found, calling start_recording...");
        match service.start_recording().await {
            Ok(()) => {
                info!("Voice recording started successfully");
                Ok(())
            }
            Err(e) => {
                error!("Failed to start recording: {}", e);
                error!("Error details: {:?}", e);
                Err(format!("Failed to start recording: {}", e))
            }
        }
    } else {
        error!("Voice service not initialized");
        Err("Voice service not initialized".to_string())
    }
}

#[tauri::command]
pub async fn stop_voice_recording(
    service_state: State<'_, TranscriptionServiceState>,
) -> Result<String, String> {
    info!("Stopping voice recording...");
    
    // Clone the service to avoid holding the lock across await
    let service = {
        let service_guard = service_state.lock().unwrap();
        service_guard.clone()
    };
    
    if let Some(service) = service {
        match service.stop_recording().await {
            Ok(()) => {
                // Get final transcription
                match service.get_final_transcription() {
                    Ok(text) => {
                        info!("Voice recording stopped, final text: {}", text);
                        Ok(text)
                    }
                    Err(e) => {
                        error!("Failed to get final transcription: {}", e);
                        Err(format!("Failed to get final transcription: {}", e))
                    }
                }
            }
            Err(e) => {
                error!("Failed to stop recording: {}", e);
                Err(format!("Failed to stop recording: {}", e))
            }
        }
    } else {
        Err("Voice service not initialized".to_string())
    }
}

#[tauri::command]
pub fn is_voice_recording(
    service_state: State<'_, TranscriptionServiceState>,
) -> Result<bool, String> {
    let service_guard = service_state.lock().unwrap();
    if let Some(ref service) = *service_guard {
        Ok(service.is_recording())
    } else {
        Ok(false)
    }
}
