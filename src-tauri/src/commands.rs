use crate::data_manager::DataManager;
use crate::models::{Node, NodeType};
use crate::security::{self, Key, VaultManager, VaultState};
use aes_gcm::aead::rand_core::RngCore;
use tauri::{AppHandle, Emitter, Runtime, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_notification::NotificationExt;

const SALT_SIZE: usize = 16;
const ENCRYPTED_VALUE_SEPARATOR: char = ':';

#[tauri::command]
pub fn get_data(
    app: AppHandle,
    vault_manager: State<'_, VaultManager>,
) -> Result<Vec<Node>, String> {
    let data_manager = DataManager::new(&app);
    data_manager.ensure_storage_healthy()?;
    let mut nodes = data_manager
        .load_data()
        .map_err(|error| error.to_string())?;

    if let VaultState::Unlocked(key) = &*vault_manager.state.lock().unwrap() {
        decrypt_nodes_recursive(&mut nodes, key);
    }

    Ok(nodes)
}

#[tauri::command]
pub fn init_vault(
    app: AppHandle,
    vault_manager: State<'_, VaultManager>,
    password: String,
) -> Result<(), String> {
    let data_manager = DataManager::new(&app);
    data_manager.ensure_storage_healthy()?;
    let hash = security::hash_password(&password);

    let mut salt_bytes = [0u8; SALT_SIZE];
    aes_gcm::aead::rand_core::OsRng.fill_bytes(&mut salt_bytes);
    let salt = hex::encode(salt_bytes);

    let key = security::derive_key_from_password(&password, &salt);

    let mut settings = data_manager
        .load_settings()
        .map_err(|error| error.to_string())?;
    settings.security.master_password_enabled = true;
    settings.security.password_hash = Some(hash);
    settings.security.derivation_salt = Some(salt);

    data_manager
        .save_settings(&settings)
        .map_err(|e| e.to_string())?;

    *vault_manager.state.lock().unwrap() = VaultState::Unlocked(key);
    Ok(())
}

#[tauri::command]
pub fn unlock_vault(
    app: AppHandle,
    vault_manager: State<'_, VaultManager>,
    password: String,
) -> Result<bool, String> {
    let data_manager = DataManager::new(&app);
    data_manager.ensure_storage_healthy()?;
    let settings = data_manager
        .load_settings()
        .map_err(|error| error.to_string())?;

    if !settings.security.master_password_enabled {
        return Err("Vault is not enabled".to_string());
    }

    let hash = settings
        .security
        .password_hash
        .as_deref()
        .ok_or_else(|| "Vault password verifier is missing; open recovery mode".to_string())?;
    if !security::verify_password(&password, hash) {
        return Ok(false);
    }
    let salt = settings
        .security
        .derivation_salt
        .as_deref()
        .ok_or_else(|| "Vault derivation salt is missing; open recovery mode".to_string())?;
    let key = security::derive_key_from_password(&password, salt);

    *vault_manager.state.lock().unwrap() = VaultState::Unlocked(key);

    Ok(true)
}

#[tauri::command]
pub fn lock_vault(vault_manager: State<'_, VaultManager>) -> Result<(), String> {
    *vault_manager.state.lock().unwrap() = VaultState::Locked;
    Ok(())
}

fn encrypt_nodes_recursive(nodes: &mut [Node], key: &Key) -> Result<(), String> {
    for node in nodes {
        if matches!(node.node_type, NodeType::Snippet) && node.is_secret.unwrap_or(false) {
            if let Some(plain_text) = &node.value {
                if !plain_text.is_empty() {
                    let (ciphertext, nonce) = security::encrypt(plain_text, key)?;
                    node.encrypted_value = Some(format!(
                        "{}{}{}",
                        nonce, ENCRYPTED_VALUE_SEPARATOR, ciphertext
                    ));
                    node.value = None;
                }
            }
        }
        if let Some(children) = &mut node.children {
            encrypt_nodes_recursive(children, key)?;
        }
    }
    Ok(())
}

fn decrypt_nodes_recursive(nodes: &mut [Node], key: &Key) {
    for node in nodes {
        if matches!(node.node_type, NodeType::Snippet) && node.is_secret.unwrap_or(false) {
            if let Some(encrypted) = &node.encrypted_value {
                if let Some(decrypted) = try_decrypt_value(encrypted, key) {
                    node.value = Some(decrypted);
                }
            }
        }
        if let Some(children) = &mut node.children {
            decrypt_nodes_recursive(children, key);
        }
    }
}

fn try_decrypt_value(encrypted: &str, key: &Key) -> Option<String> {
    let parts: Vec<&str> = encrypted.split(ENCRYPTED_VALUE_SEPARATOR).collect();
    if parts.len() != 2 {
        return None;
    }
    security::decrypt(parts[1], parts[0], key).ok()
}

fn has_plain_secrets(nodes: &[Node]) -> bool {
    nodes.iter().any(|n| {
        let is_plain_secret = matches!(n.node_type, NodeType::Snippet)
            && n.is_secret.unwrap_or(false)
            && n.value.is_some();
        is_plain_secret || n.children.as_deref().is_some_and(has_plain_secrets)
    })
}

#[tauri::command]
pub fn save_data(
    app: AppHandle,
    vault_manager: State<'_, VaultManager>,
    mut nodes: Vec<Node>,
) -> Result<(), String> {
    let data_manager = DataManager::new(&app);
    data_manager.ensure_storage_healthy()?;
    data_manager
        .load_data()
        .map_err(|error| error.to_string())?;
    let settings = data_manager
        .load_settings()
        .map_err(|error| error.to_string())?;

    if !settings.security.master_password_enabled && has_encrypted_values(&nodes) {
        return Err("Encrypted snippets require an enabled vault".to_string());
    }

    if has_plain_secrets(&nodes) {
        let state = vault_manager.state.lock().unwrap();
        match &*state {
            VaultState::Unlocked(key) => encrypt_nodes_recursive(&mut nodes, key)?,
            VaultState::Locked => return Err("Vault is locked. Cannot encrypt new secrets.".into()),
        }
    }

    data_manager.save_data(&nodes).map_err(|e| e.to_string())?;

    if settings.auto_backup_enabled {
        if let Err(e) = data_manager.create_backup() {
            log::error!("Failed to create backup after save: {}", e);
        } else if let Err(e) = data_manager.rotate_backups(settings.auto_backup_count) {
            log::error!("Failed to rotate backups: {}", e);
        }
    }

    let menu = crate::tray_generator::TrayGenerator::generate_menu(&app, &nodes, &settings)
        .map_err(|e| e.to_string())?;
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_menu(Some(menu));
    }

    Ok(())
}

#[tauri::command]
pub fn copy_snippet<R: Runtime>(
    app: AppHandle<R>,
    vault_manager: State<'_, VaultManager>,
    id: String,
) -> Result<(), String> {
    let data_manager = DataManager::new(&app);
    data_manager.ensure_storage_healthy()?;
    let nodes = data_manager
        .load_data()
        .map_err(|error| error.to_string())?;
    let settings = data_manager
        .load_settings()
        .map_err(|error| error.to_string())?;

    let node = DataManager::find_node_by_id(&nodes, &id).ok_or("Snippet not found")?;

    let value = if node.is_secret.unwrap_or(false) {
        let state = vault_manager.state.lock().unwrap();
        match &*state {
            VaultState::Locked => return Err("Vault is Locked".into()),
            VaultState::Unlocked(key) => {
                let encrypted = node.encrypted_value.as_ref().ok_or("No encrypted value")?;
                try_decrypt_value(encrypted, key).ok_or("Failed to decrypt")?
            }
        }
    } else {
        node.value.clone().unwrap_or_default()
    };

    if value.is_empty() {
        return Err("Empty value".into());
    }

    app.clipboard()
        .write_text(value)
        .map_err(|e| e.to_string())?;

    *vault_manager.last_used_id.lock().unwrap() = Some(id);

    if settings.notifications_enabled {
        let _ = app
            .notification()
            .builder()
            .title("Sklad")
            .body(format!("Copied: {}", node.label))
            .show();
    }

    Ok(())
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<crate::models::AppSettings, String> {
    let data_manager = DataManager::new(&app);
    data_manager.ensure_storage_healthy()?;
    data_manager
        .load_settings()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    vault_manager: State<'_, VaultManager>,
    settings: crate::models::AppSettings,
) -> Result<(), String> {
    let data_manager = DataManager::new(&app);
    data_manager.ensure_storage_healthy()?;
    data_manager
        .load_settings()
        .map_err(|error| error.to_string())?;
    let nodes = data_manager
        .load_data()
        .map_err(|error| error.to_string())?;
    if settings.security.master_password_enabled
        && (settings.security.password_hash.is_none()
            || settings.security.derivation_salt.is_none())
    {
        return Err(
            "Enabled vault settings require a password verifier and derivation salt".to_string(),
        );
    }
    if !settings.security.master_password_enabled && has_encrypted_values(&nodes) {
        return Err(
            "Disable the vault through Reset Vault so encrypted snippets are handled safely"
                .to_string(),
        );
    }
    data_manager
        .save_settings(&settings)
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    if !settings.security.master_password_enabled {
        *vault_manager.state.lock().unwrap() = VaultState::Locked;
    }

    crate::LOGGING_ENABLED.store(
        settings.logging_enabled,
        std::sync::atomic::Ordering::Relaxed,
    );

    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let _ = app.global_shortcut().unregister_all();

    // Register Search Shortcut
    if !settings.global_search_shortcut.is_empty() {
        match settings
            .global_search_shortcut
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
        {
            Ok(shortcut) => {
                if let Err(e) = app.global_shortcut().register(shortcut) {
                    log::error!("Failed to register search shortcut: {}", e);
                }
            }
            Err(e) => {
                log::error!(
                    "Failed to parse search shortcut string '{}': {}",
                    settings.global_search_shortcut,
                    e
                );
            }
        }
    }

    // Register Create Shortcut
    if !settings.global_create_shortcut.is_empty() {
        match settings
            .global_create_shortcut
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
        {
            Ok(shortcut) => {
                if let Err(e) = app.global_shortcut().register(shortcut) {
                    log::error!("Failed to register create shortcut: {}", e);
                }
            }
            Err(e) => {
                log::error!(
                    "Failed to parse create shortcut string '{}': {}",
                    settings.global_create_shortcut,
                    e
                );
            }
        }
    }

    if let Ok(menu) = crate::tray_generator::TrayGenerator::generate_menu(&app, &nodes, &settings) {
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_menu(Some(menu));
        }
    }

    Ok(())
}

#[tauri::command]
pub fn is_vault_unlocked(vault_manager: State<'_, VaultManager>) -> bool {
    matches!(
        *vault_manager.state.lock().unwrap(),
        VaultState::Unlocked(_)
    )
}

#[tauri::command]
pub fn get_snippets_path(app: AppHandle) -> String {
    DataManager::new(&app)
        .file_path
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
pub fn open_snippets_path(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = DataManager::new(&app).file_path;
    app.opener()
        .open_path(path.to_string_lossy(), None::<String>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_data_directory(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let data_manager = DataManager::new(&app);
    let directory = data_manager
        .file_path
        .parent()
        .ok_or_else(|| "Failed to resolve application data directory".to_string())?;
    app.opener()
        .open_path(directory.to_string_lossy(), None::<String>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_app_logs_dir(app: AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use tauri_plugin_opener::OpenerExt;

    // Fallback to essentially the app path / logs if path resolving fails
    let app_log_dir = app.path().app_log_dir().map_err(|e| e.to_string())?;

    // Ensure the dir exists before trying to open it natively, else Windows Explorer might error
    if !app_log_dir.exists() {
        std::fs::create_dir_all(&app_log_dir)
            .map_err(|e| format!("Failed to create logs dir: {}", e))?;
    }

    app.opener()
        .open_path(app_log_dir.to_string_lossy(), None::<String>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_vault(
    app: AppHandle,
    vault_manager: State<'_, VaultManager>,
) -> Result<(Vec<Node>, crate::models::AppSettings), String> {
    let data_manager = DataManager::new(&app);
    data_manager.ensure_storage_healthy()?;
    let mut nodes = data_manager
        .load_data()
        .map_err(|error| error.to_string())?;
    let mut settings = data_manager
        .load_settings()
        .map_err(|error| error.to_string())?;

    remove_secrets_recursive(&mut nodes);
    settings.security.master_password_enabled = false;
    settings.security.password_hash = None;
    settings.security.derivation_salt = None;

    data_manager.save_data(&nodes).map_err(|e| e.to_string())?;
    data_manager
        .save_settings(&settings)
        .map_err(|e| e.to_string())?;

    *vault_manager.state.lock().unwrap() = VaultState::Locked;

    Ok((nodes, settings))
}

fn remove_secrets_recursive(nodes: &mut Vec<Node>) {
    nodes.retain(|node| !node.is_secret.unwrap_or(false) && node.encrypted_value.is_none());
    for node in nodes.iter_mut() {
        if let Some(children) = &mut node.children {
            remove_secrets_recursive(children);
        }
    }
}

fn has_encrypted_values(nodes: &[Node]) -> bool {
    nodes.iter().any(|node| {
        node.encrypted_value.is_some() || node.children.as_deref().is_some_and(has_encrypted_values)
    })
}

#[tauri::command]
pub fn create_backup(app: AppHandle) -> Result<(), String> {
    let data_manager = DataManager::new(&app);
    data_manager.ensure_storage_healthy()?;
    let settings = data_manager
        .load_settings()
        .map_err(|error| error.to_string())?;

    data_manager.create_backup().map_err(|e| e.to_string())?;
    data_manager
        .rotate_backups(settings.auto_backup_count)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_backups(app: AppHandle) -> Vec<crate::models::BackupInfo> {
    DataManager::new(&app).list_backups()
}

#[tauri::command]
pub fn restore_backup(app: AppHandle, filename: String) -> Result<(), String> {
    let data_manager = DataManager::new(&app);
    data_manager.restore_backup(&filename)?;
    refresh_tray_from_storage(&app, &data_manager)?;

    app.emit("data-updated", ()).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_storage_status(app: AppHandle) -> crate::data_manager::StorageStatus {
    DataManager::new(&app).storage_status()
}

#[tauri::command]
pub fn reset_corrupt_data(app: AppHandle) -> Result<String, String> {
    let data_manager = DataManager::new(&app);
    let quarantined = data_manager.reset_invalid_data()?;
    refresh_tray_from_storage(&app, &data_manager)?;

    app.emit("data-updated", ())
        .map_err(|error| error.to_string())?;
    Ok(quarantined)
}

#[tauri::command]
pub fn reset_corrupt_settings(
    app: AppHandle,
    vault_manager: State<'_, VaultManager>,
) -> Result<String, String> {
    let data_manager = DataManager::new(&app);
    let quarantined = data_manager.reset_invalid_settings()?;
    *vault_manager.state.lock().unwrap() = VaultState::Locked;
    crate::LOGGING_ENABLED.store(false, std::sync::atomic::Ordering::Relaxed);
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let _ = app.global_shortcut().unregister_all();
    refresh_tray_from_storage(&app, &data_manager)?;
    Ok(quarantined)
}

#[tauri::command]
pub fn discard_unrecoverable_vault_data(
    app: AppHandle,
    vault_manager: State<'_, VaultManager>,
) -> Result<crate::data_manager::VaultRecoveryResult, String> {
    let data_manager = DataManager::new(&app);
    let result = data_manager.discard_unrecoverable_vault_data()?;
    *vault_manager.state.lock().unwrap() = VaultState::Locked;
    crate::LOGGING_ENABLED.store(false, std::sync::atomic::Ordering::Relaxed);
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let _ = app.global_shortcut().unregister_all();
    refresh_tray_from_storage(&app, &data_manager)?;
    app.emit("data-updated", ())
        .map_err(|error| error.to_string())?;
    Ok(result)
}

fn refresh_tray_from_storage(app: &AppHandle, data_manager: &DataManager) -> Result<(), String> {
    let storage_is_healthy = !data_manager.has_storage_issues();
    let (mut settings, nodes) = if storage_is_healthy {
        (
            data_manager
                .load_settings()
                .map_err(|error| error.to_string())?,
            data_manager
                .load_data()
                .map_err(|error| error.to_string())?,
        )
    } else {
        (crate::models::AppSettings::default(), Vec::new())
    };
    if !storage_is_healthy {
        settings.auto_backup_enabled = false;
    }
    let menu_nodes = if storage_is_healthy {
        nodes.as_slice()
    } else {
        &[]
    };
    let menu = crate::tray_generator::TrayGenerator::generate_menu(app, menu_nodes, &settings)
        .map_err(|error| error.to_string())?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}
