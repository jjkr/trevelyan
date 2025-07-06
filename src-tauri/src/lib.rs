use global_hotkey::{hotkey::{Code, HotKey, Modifiers}, GlobalHotKeyManager, GlobalHotKeyEvent, HotKeyState};
use tauri::{Manager, menu::{MenuBuilder, MenuItemBuilder}, tray::TrayIconBuilder};
use log::info;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Space);
    manager.register(hotkey).unwrap();

    tauri::Builder::default()
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

            // Create system tray menu
            let show_hide = MenuItemBuilder::with_id("show_hide", "Show/Hide").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .items(&[&show_hide, &quit])
                .build()?;

            // Create system tray with white icon
            let tray_icon = {
                let icon_bytes = include_bytes!("../icons/OriginalLogoWhite.png");
                match image::load_from_memory(icon_bytes) {
                    Ok(img) => {
                        let rgba = img.to_rgba8();
                        let (width, height) = rgba.dimensions();
                        tauri::image::Image::new_owned(rgba.into_raw(), width, height)
                    }
                    Err(_) => app.default_window_icon().unwrap().clone()
                }
            };
            
            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&tray_menu)
                .tooltip("Trevelyan")
                .on_menu_event(move |app, event| {
                    let window = app.get_webview_window("main").unwrap();
                    match event.id.as_ref() {
                        "show_hide" => {
                            if window.is_visible().unwrap() {
                                window.hide().unwrap();
                            } else {
                                window.show().unwrap();
                                window.set_focus().unwrap();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Handle window close event to hide instead of quit
            let window_clone = main_window.clone();
            main_window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    window_clone.hide().unwrap();
                }
            });

            let receiver = GlobalHotKeyEvent::receiver();
            std::thread::spawn(move || loop {
                if let Ok(event) = receiver.recv() {
                    info!("Received global hotkey event: {:?}", event);
                    if event.id == hotkey.id() && event.state == HotKeyState::Pressed {
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
