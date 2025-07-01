use global_hotkey::{hotkey::{Code, HotKey, Modifiers}, GlobalHotKeyManager, GlobalHotKeyEvent};
use tauri::Manager;
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

mod audio;
mod commands;

use commands::{
    VoiceState, ensure_initialized, start_recording, stop_recording,
    is_recording, is_ready, get_status, update_transcription_config,
    get_session_info, get_model_status, clear_model_cache, test_transcription, get_transcription_stats
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::SUPER), Code::Space);
    manager.register(hotkey).unwrap();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // Voice transcription commands (now using Candle + Distil-Whisper)
            ensure_initialized,
            start_recording,
            stop_recording,
            is_recording,
            is_ready,
            get_status,
            update_transcription_config,
            get_session_info,
            get_model_status,
            clear_model_cache,
            test_transcription,
            get_transcription_stats
        ])
        .setup(move |app| {
            // Initialize voice state with Candle backend
            println!("🚀 [lib.rs] Setting up Tauri app...");
            println!("🚀 [lib.rs] Initializing VoiceState...");
            
            match VoiceState::new(app.handle().clone()) {
                Ok(voice_state) => {
                    println!("🚀 [lib.rs] VoiceState initialized successfully");
                    app.manage(voice_state);
                }
                Err(e) => {
                    println!("🚀 [lib.rs] Failed to initialize VoiceState: {}", e);
                    panic!("Failed to initialize voice state: {}", e);
                }
            }
            
            println!("🚀 [lib.rs] Getting main window...");
            let main_window = app.get_webview_window("main").unwrap();

            if let Some(monitor) = main_window.current_monitor().unwrap() {
                let size = monitor.size();
                let width = size.width / 3;
                let height = size.height;
                let x = size.width - width;

                main_window
                    .set_size(tauri::PhysicalSize::new(width, height))
                    .unwrap();
                main_window
                    .set_position(tauri::PhysicalPosition::new(x, 0))
                    .unwrap();
            }

            // Remove vibrancy for true transparency on macOS Sequoia
            // #[cfg(target_os = "macos")]
            // apply_vibrancy(&main_window, NSVisualEffectMaterial::UnderWindowBackground, None, None)
            //     .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let receiver = GlobalHotKeyEvent::receiver();
            std::thread::spawn(move || loop {
                if let Ok(event) = receiver.recv() {
                    if event.id == hotkey.id() {
                        if main_window.is_visible().unwrap() {
                            main_window.hide().unwrap();
                        } else {
                            main_window.show().unwrap();
                            main_window.set_focus().unwrap();
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
