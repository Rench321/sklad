pub mod commands;
pub mod data_manager;
pub mod models;
pub mod security;
pub mod tray_generator;

use crate::data_manager::DataManager;
use crate::tray_generator::TrayGenerator;
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_notification::NotificationExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.emit("single-instance", ());
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        if let Some(window) = app.get_webview_window("search") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        } else {
                            eprintln!("Could not find search window!");
                        }
                    }
                })
                .build(),
        )
        // on_window_event removed to allow frontend to handle CloseRequested

        .setup(|app| {
            let handle = app.handle();
            let data_manager = DataManager::new(handle);
            let nodes = data_manager.load_data();

            let menu = TrayGenerator::generate_menu(handle, &nodes)?;

            tauri::tray::TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    let id = event.id.as_ref();
                    match id {
                        "quit" => app.exit(0),
                        "open" => show_main_window(app),
                        snippet_id => handle_snippet_click(app, snippet_id.to_string()),
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        let vault_manager = app.state::<crate::security::VaultManager>();
                        let last_id = vault_manager.last_used_id.lock().unwrap().clone();

                        if let Some(id) = last_id {
                            handle_snippet_click(app, id);
                        }
                    }
                })
                .build(app)?;

            use std::env;

            // Check for --minimized flag
            let args: Vec<String> = env::args().collect();
            let start_minimized = args.contains(&"--minimized".to_string());

            if !start_minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            // Register global shortcut
            let settings = data_manager.load_settings();
            let shortcut_str = &settings.global_search_shortcut;
            if !shortcut_str.is_empty() {
                use tauri_plugin_global_shortcut::GlobalShortcutExt;
                match shortcut_str.parse::<tauri_plugin_global_shortcut::Shortcut>() {
                    Ok(shortcut) => {
                        if let Err(e) = app.global_shortcut().register(shortcut) {
                            eprintln!("Failed to register shortcut on startup: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse shortcut string '{}' on startup: {}", shortcut_str, e);
                    }
                }
            }

            Ok(())
        })
        .manage(crate::security::VaultManager::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_data,
            commands::save_data,
            commands::copy_snippet,
            commands::init_vault,
            commands::unlock_vault,
            commands::lock_vault,
            commands::get_settings,
            commands::save_settings,
            commands::get_snippets_path,
            commands::open_snippets_path,
            commands::reset_vault,
            commands::is_vault_unlocked
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn handle_snippet_click(app: &tauri::AppHandle, id: String) {
    let vault_manager = app.state::<crate::security::VaultManager>();

    if let Err(e) = crate::commands::copy_snippet(app.clone(), vault_manager, id.clone()) {
        if e == "Vault is Locked" {
            show_main_window(app);
            let _ = app.emit("request-unlock", id);
        } else {
            let _ = app
                .notification()
                .builder()
                .title("Sklad: Copy Error")
                .body(format!("Failed to copy: {}", e))
                .show();
        }
    }
}

