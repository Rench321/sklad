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
    let app = tauri::Builder::default()
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
                .with_handler(|app, shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let data_manager = DataManager::new(&app);
                        let settings = data_manager.load_settings();

                        // Parse the shortcuts from settings to compare IDs
                        let search_shortcut = settings
                            .global_search_shortcut
                            .parse::<tauri_plugin_global_shortcut::Shortcut>()
                            .ok();
                        let create_shortcut = settings
                            .global_create_shortcut
                            .parse::<tauri_plugin_global_shortcut::Shortcut>()
                            .ok();

                        println!("Shortcut pressed: {:?}", shortcut);
                        println!(
                            "Settings search: {:?} (parsed: {:?})",
                            settings.global_search_shortcut, search_shortcut
                        );
                        println!(
                            "Settings create: {:?} (parsed: {:?})",
                            settings.global_create_shortcut, create_shortcut
                        );

                        if Some(*shortcut) == search_shortcut {
                            if let Some(window) = app.get_webview_window("search") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            } else {
                                eprintln!("Could not find search window!");
                            }
                        } else if Some(*shortcut) == create_shortcut {
                            if let Some(window) = app.get_webview_window("create") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            } else {
                                eprintln!("Could not find create window!");
                            }
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

            // Setup customized macOS app menu with metadata
            #[cfg(target_os = "macos")]
            {
                use tauri::menu::{AboutMetadata, Menu, PredefinedMenuItem};
                if let Ok(mut default_menu) = Menu::default(app.handle()) {
                    let mut about_metadata = AboutMetadata::default();
                    about_metadata.version = Some(app.package_info().version.to_string());
                    about_metadata.website = Some("https://github.com/Rench321/sklad".to_string());
                    about_metadata.website_label = Some("GitHub Repository".to_string());
                    about_metadata.comments =
                        Some("Industrial-grade secure snippet warehouse".to_string());

                    if let Ok(about_item) =
                        PredefinedMenuItem::about(app.handle(), None, Some(about_metadata))
                    {
                        // The default menu on macOS has the app menu as the first item
                        if let Ok(items) = default_menu.items() {
                            if let Some(app_menu) = items.first() {
                                if let Some(_submenu) = app_menu.as_submenu() {
                                    // The about item is usually the first one in the app submenu
                                    // We can just append it or insert it
                                    // Alternatively, since setting the default menu is easier, let's just let Tauri handle it if we build the menu.
                                    // Actually tauri::menu::Menu::default() returns a new menu.
                                }
                            }
                        }
                    }
                }
            }

            // We only actually need to inject the about metadata if macOS
            #[cfg(target_os = "macos")]
            if let Ok(menu) = tauri::menu::Menu::default(app.handle()) {
                let _ = app.set_menu(menu); // though we still need to modify the AboutMetadata inside it, which might be tricky in v2 this way.
            }

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
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();

                        let data_manager = DataManager::new(app);
                        let settings = data_manager.load_settings();

                        if settings.tray_click_action == "open_app" {
                            show_main_window(app);
                        } else {
                            // "copy_last" is the default fallback
                            let vault_manager = app.state::<crate::security::VaultManager>();
                            let last_id = vault_manager.last_used_id.lock().unwrap().clone();

                            if let Some(id) = last_id {
                                handle_snippet_click(app, id);
                            }
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

            // Register global shortcuts
            let settings = data_manager.load_settings();

            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            let shortcut_str = &settings.global_search_shortcut;
            if !shortcut_str.is_empty() {
                match shortcut_str.parse::<tauri_plugin_global_shortcut::Shortcut>() {
                    Ok(shortcut) => {
                        if let Err(e) = app.global_shortcut().register(shortcut) {
                            eprintln!("Failed to register search shortcut on startup: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to parse search shortcut string '{}' on startup: {}",
                            shortcut_str, e
                        );
                    }
                }
            }

            let create_shortcut_str = &settings.global_create_shortcut;
            if !create_shortcut_str.is_empty() {
                match create_shortcut_str.parse::<tauri_plugin_global_shortcut::Shortcut>() {
                    Ok(shortcut) => {
                        if let Err(e) = app.global_shortcut().register(shortcut) {
                            eprintln!("Failed to register create shortcut on startup: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to parse create shortcut string '{}' on startup: {}",
                            create_shortcut_str, e
                        );
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
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|_app_handle, _event| {
        #[cfg(target_os = "macos")]
        let app_handle = _app_handle;
        #[cfg(target_os = "macos")]
        let event = _event;

        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            api.prevent_exit();
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.hide();
            }
        }
    });
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
