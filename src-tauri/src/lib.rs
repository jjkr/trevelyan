use global_hotkey::{hotkey::{Code, HotKey, Modifiers}, GlobalHotKeyManager, GlobalHotKeyEvent};
use tauri::Manager;
#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};

mod audio;
mod commands;

use commands::{VoiceState, ensure_whisper_initialized, start_realtime_recording, stop_recording, is_recording, is_whisper_ready, get_whisper_status};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::SUPER), Code::Space);
    manager.register(hotkey).unwrap();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ensure_whisper_initialized,
            start_realtime_recording,
            stop_recording,
            is_recording,
            is_whisper_ready,
            get_whisper_status
        ])
        .setup(move |app| {
            app.manage(VoiceState::new(app.handle().clone()).expect("Failed to initialize voice state"));
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
