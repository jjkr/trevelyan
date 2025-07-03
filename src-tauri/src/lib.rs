use global_hotkey::{hotkey::{Code, HotKey, Modifiers}, GlobalHotKeyManager, GlobalHotKeyEvent};
use tauri::Manager;
use std::sync::{Arc, Mutex};
use log::info;

mod audio_capture;
mod whisper_engine;
mod transcription_service;
mod voice_commands;

use voice_commands::{
    TranscriptionServiceState,
    initialize_voice_service,
    start_voice_recording,
    stop_voice_recording,
    is_voice_recording,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::SUPER), Code::Space);
    manager.register(hotkey).unwrap();

    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(None)) as TranscriptionServiceState)
        .invoke_handler(tauri::generate_handler![
            initialize_voice_service,
            start_voice_recording,
            stop_voice_recording,
            is_voice_recording,
        ])
        .setup(move |app| {
            info!("Getting main window...");
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

            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(if cfg!(debug_assertions) {
                        log::LevelFilter::Debug
                    } else {
                        log::LevelFilter::Info
                    })
                    .build(),
            )?;

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
