use crate::transcription_service::TranscriptionService;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};

pub type TranscriptionServiceState = Arc<Mutex<Option<TranscriptionService>>>;

#[tauri::command]
pub async fn initialize_voice_service(
    app_handle: AppHandle,
    service_state: State<'_, TranscriptionServiceState>,
) -> Result<(), String> {
    println!("🎤 [voice_commands] Starting voice service initialization...");
    
    let mut service = TranscriptionService::new(app_handle);
    println!("🎤 [voice_commands] TranscriptionService created");
    
    match service.initialize().await {
        Ok(()) => {
            println!("🎤 [voice_commands] Service initialization successful, storing in state...");
            let mut state = service_state.lock().unwrap();
            *state = Some(service);
            println!("✅ [voice_commands] Voice service initialized and stored successfully");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ [voice_commands] Failed to initialize voice service: {}", e);
            eprintln!("❌ [voice_commands] Error details: {:?}", e);
            Err(format!("Failed to initialize voice service: {}", e))
        }
    }
}

#[tauri::command]
pub async fn start_voice_recording(
    service_state: State<'_, TranscriptionServiceState>,
) -> Result<(), String> {
    println!("🎤 [voice_commands] Starting voice recording...");
    
    // Clone the service to avoid holding the lock across await
    let service = {
        println!("🎤 [voice_commands] Acquiring service state lock...");
        let service_guard = service_state.lock().unwrap();
        println!("🎤 [voice_commands] Lock acquired, cloning service...");
        service_guard.clone()
    };
    
    if let Some(service) = service {
        println!("🎤 [voice_commands] Service found, calling start_recording...");
        match service.start_recording().await {
            Ok(()) => {
                println!("✅ [voice_commands] Voice recording started successfully");
                Ok(())
            }
            Err(e) => {
                eprintln!("❌ [voice_commands] Failed to start recording: {}", e);
                eprintln!("❌ [voice_commands] Error details: {:?}", e);
                Err(format!("Failed to start recording: {}", e))
            }
        }
    } else {
        eprintln!("❌ [voice_commands] Voice service not initialized");
        Err("Voice service not initialized".to_string())
    }
}

#[tauri::command]
pub async fn stop_voice_recording(
    service_state: State<'_, TranscriptionServiceState>,
) -> Result<String, String> {
    println!("Stopping voice recording...");
    
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
                        println!("Voice recording stopped, final text: {}", text);
                        Ok(text)
                    }
                    Err(e) => {
                        eprintln!("Failed to get final transcription: {}", e);
                        Err(format!("Failed to get final transcription: {}", e))
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to stop recording: {}", e);
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
