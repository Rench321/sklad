// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

pub mod models;
pub mod data_manager;
pub mod tray_generator;
pub mod commands;
pub mod security;

use crate::data_manager::DataManager;
use crate::tray_generator::TrayGenerator;
use tauri::{Manager, Emitter};
use tauri_plugin_notification::NotificationExt;


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(|app| {
            let handle = app.handle();
            let data_manager = DataManager::new(handle);
            let nodes = data_manager.load_data();
            
            // Generate the tray menu
            let menu = TrayGenerator::generate_menu(handle, &nodes)?;
            
            // Set up the tray icon
            tauri::tray::TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    let id = event.id.as_ref();
                    if id == "quit" {
                        app.exit(0);
                    } else if id == "open" {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    } else {
                        // It's a snippet ID
                        let vault_manager = app.state::<crate::security::VaultManager>();
                        let id_str = id.to_string();
                        if let Err(e) = crate::commands::copy_snippet(app.clone(), vault_manager, id_str.clone()) {
                            eprintln!("Failed to copy: {}", e);
                            if e == "Vault is Locked" {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                                let _ = app.emit("request-unlock", id_str);
                            } else {
                                let _ = app.notification()
                                    .builder()
                                    .title("Sklad: Copy Error")
                                    .body(format!("Failed to copy: {}", e))
                                    .show();
                            }
                        }
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event {
                        let app = tray.app_handle();
                        let vault_manager = app.state::<crate::security::VaultManager>();
                        let last_id = {
                            let lock = vault_manager.last_used_id.lock().unwrap();
                            lock.clone()
                        };

                        if let Some(id) = last_id {
                            if let Err(e) = crate::commands::copy_snippet(app.clone(), vault_manager, id.clone()) {
                                eprintln!("Failed to copy last used: {}", e);
                                if e == "Vault is Locked" {
                                    if let Some(window) = app.get_webview_window("main") {
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                    }
                                    let _ = app.emit("request-unlock", id);
                                } else {
                                    let _ = app.notification()
                                        .builder()
                                        .title("Sklad: Copy Error")
                                        .body(format!("Failed to copy: {}", e))
                                        .show();
                                }
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .manage(crate::security::VaultManager::new())
        .invoke_handler(tauri::generate_handler![
            greet, 
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
