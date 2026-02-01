use crate::data_manager::DataManager;
use crate::models::{Node, NodeType};
use crate::security::{self, Key, VaultManager, VaultState};
use aes_gcm::aead::rand_core::RngCore;
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_notification::NotificationExt;

const SALT_SIZE: usize = 16;
const ENCRYPTED_VALUE_SEPARATOR: char = ':';

#[tauri::command]
pub fn get_data(app: AppHandle, vault_manager: State<'_, VaultManager>) -> Vec<Node> {
    let data_manager = DataManager::new(&app);
    let mut nodes = data_manager.load_data();

    if let VaultState::Unlocked(key) = &*vault_manager.state.lock().unwrap() {
        decrypt_nodes_recursive(&mut nodes, key);
    }

    nodes
}

#[tauri::command]
pub fn init_vault(
    app: AppHandle,
    vault_manager: State<'_, VaultManager>,
    password: String,
) -> Result<(), String> {
    let hash = security::hash_password(&password);

    let mut salt_bytes = [0u8; SALT_SIZE];
    aes_gcm::aead::rand_core::OsRng.fill_bytes(&mut salt_bytes);
    let salt = hex::encode(salt_bytes);

    let key = security::derive_key_from_password(&password, &salt);

    *vault_manager.state.lock().unwrap() = VaultState::Unlocked(key);

    let data_manager = DataManager::new(&app);
    let mut settings = data_manager.load_settings();
    settings.security.master_password_enabled = true;
    settings.security.password_hash = Some(hash);
    settings.security.derivation_salt = Some(salt);

    data_manager
        .save_settings(&settings)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn unlock_vault(
    app: AppHandle,
    vault_manager: State<'_, VaultManager>,
    password: String,
) -> Result<bool, String> {
    let data_manager = DataManager::new(&app);
    let settings = data_manager.load_settings();

    if let Some(hash) = &settings.security.password_hash {
        if !security::verify_password(&password, hash) {
            return Ok(false);
        }
    } else if settings.security.master_password_enabled {
        return Err("Security enabled but no password hash found. Please reset vault.".into());
    }

    let salt = settings
        .security
        .derivation_salt
        .as_deref()
        .unwrap_or("default-salt");
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
                    node.encrypted_value =
                        Some(format!("{}{}{}", nonce, ENCRYPTED_VALUE_SEPARATOR, ciphertext));
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
        is_plain_secret || n.children.as_ref().map_or(false, |c| has_plain_secrets(c))
    })
}

#[tauri::command]
pub fn save_data(
    app: AppHandle,
    vault_manager: State<'_, VaultManager>,
    mut nodes: Vec<Node>,
) -> Result<(), String> {
    if has_plain_secrets(&nodes) {
        let state = vault_manager.state.lock().unwrap();
        match &*state {
            VaultState::Unlocked(key) => encrypt_nodes_recursive(&mut nodes, key)?,
            VaultState::Locked => return Err("Vault is locked. Cannot encrypt new secrets.".into()),
        }
    }

    let data_manager = DataManager::new(&app);
    data_manager.save_data(&nodes).map_err(|e| e.to_string())?;

    let menu =
        crate::tray_generator::TrayGenerator::generate_menu(&app, &nodes).map_err(|e| e.to_string())?;
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
    let nodes = data_manager.load_data();

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

    let settings = data_manager.load_settings();
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
pub fn get_settings(app: AppHandle) -> crate::models::AppSettings {
    DataManager::new(&app).load_settings()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    vault_manager: State<'_, VaultManager>,
    settings: crate::models::AppSettings,
) -> Result<(), String> {
    if !settings.security.master_password_enabled {
        *vault_manager.state.lock().unwrap() = VaultState::Locked;
    }

    DataManager::new(&app)
        .save_settings(&settings)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_vault_unlocked(vault_manager: State<'_, VaultManager>) -> bool {
    matches!(*vault_manager.state.lock().unwrap(), VaultState::Unlocked(_))
}

#[tauri::command]
pub fn get_snippets_path(app: AppHandle) -> String {
    DataManager::new(&app).file_path.to_string_lossy().to_string()
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
pub fn reset_vault(app: AppHandle) -> Result<(Vec<Node>, crate::models::AppSettings), String> {
    let data_manager = DataManager::new(&app);
    let mut nodes = data_manager.load_data();
    let mut settings = data_manager.load_settings();

    remove_secrets_recursive(&mut nodes);
    settings.security.master_password_enabled = false;

    data_manager.save_data(&nodes).map_err(|e| e.to_string())?;
    data_manager
        .save_settings(&settings)
        .map_err(|e| e.to_string())?;

    Ok((nodes, settings))
}

fn remove_secrets_recursive(nodes: &mut Vec<Node>) {
    nodes.retain(|node| !node.is_secret.unwrap_or(false));
    for node in nodes.iter_mut() {
        if let Some(children) = &mut node.children {
            remove_secrets_recursive(children);
        }
    }
}

