use crate::data_manager::DataManager;
use crate::models::{Node};
use crate::security::{self, VaultManager, VaultState};
use tauri::{AppHandle, Runtime, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_notification::NotificationExt;
use aes_gcm::aead::rand_core::RngCore;

#[tauri::command]
pub fn get_data(app: AppHandle, vault_manager: State<'_, VaultManager>) -> Vec<Node> {
    let data_manager = DataManager::new(&app);
    let mut nodes = data_manager.load_data();
    
    // If vault is unlocked, decrypt secrets so they are visible in the UI
    let state = vault_manager.state.lock().unwrap();
    if let VaultState::Unlocked(key) = &*state {
        let _ = decrypt_nodes_recursive(&mut nodes, key);
    }
    
    nodes
}

#[tauri::command]
pub fn init_vault(
    app: tauri::AppHandle,
    vault_manager: tauri::State<'_, VaultManager>,
    password: String
) -> Result<(), String> {
    // Generate a secure hash of the password
    let hash = security::hash_password(&password);
    
    // Generate a stable salt for key derivation
    let mut salt_bytes = [0u8; 16];
    aes_gcm::aead::rand_core::OsRng.fill_bytes(&mut salt_bytes);
    let salt_str = hex::encode(salt_bytes);
    
    // Derive the initial key
    let key = security::derive_key_from_password(&password, &salt_str);
    
    let mut state = vault_manager.state.lock().unwrap();
    *state = VaultState::Unlocked(key);

    // Persist security settings
    let data_manager = DataManager::new(&app);
    let mut settings = data_manager.load_settings();
    settings.security.master_password_enabled = true;
    settings.security.password_hash = Some(hash);
    settings.security.derivation_salt = Some(salt_str);
    
    data_manager.save_settings(&settings).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub fn unlock_vault(
    app: tauri::AppHandle, 
    vault_manager: tauri::State<'_, VaultManager>, 
    password: String
) -> Result<bool, String> {
    let data_manager = DataManager::new(&app);
    let settings = data_manager.load_settings();
    
    // 1. Verify password if we have a hash
    if let Some(hash) = &settings.security.password_hash {
        if !security::verify_password(&password, hash) {
            return Ok(false);
        }
    } else {
        // If security is enabled but no hash exists, something is wrong
        // but for now we might allow init? No, init_vault should have been called.
        if settings.security.master_password_enabled {
            return Err("Security enabled but no password hash found. Please reset vault.".into());
        }
    }
    
    // 2. Derive key using the same stable salt
    let salt = settings.security.derivation_salt.as_deref().unwrap_or("fixed-salt-for-demo"); 
    let key = security::derive_key_from_password(&password, salt);
    
    let mut state = vault_manager.state.lock().unwrap();
    *state = VaultState::Unlocked(key);
    
    Ok(true)
}

#[tauri::command]
pub fn lock_vault(vault_manager: State<'_, VaultManager>) -> Result<(), String> {
    let mut state = vault_manager.state.lock().unwrap();
    *state = VaultState::Locked;
    Ok(())
}

fn encrypt_nodes_recursive(nodes: &mut [crate::models::Node], key: &crate::security::Key) -> Result<(), String> {
    for node in nodes {
        if matches!(node.node_type, crate::models::NodeType::Snippet) && node.is_secret.unwrap_or(false) {
            if let Some(plain_text) = &node.value {
                if !plain_text.is_empty() {
                    let (ciphertext, nonce) = security::encrypt(plain_text, key)?;
                    node.encrypted_value = Some(format!("{}:{}", nonce, ciphertext));
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

fn decrypt_nodes_recursive(nodes: &mut [crate::models::Node], key: &crate::security::Key) -> Result<(), String> {
    for node in nodes {
        if matches!(node.node_type, crate::models::NodeType::Snippet) && node.is_secret.unwrap_or(false) {
            if let Some(enc_val) = &node.encrypted_value {
                // Try to decrypt
                let parts: Vec<&str> = enc_val.split(':').collect();
                if parts.len() == 2 {
                    if let Ok(decrypted) = security::decrypt(parts[1], parts[0], key) {
                        node.value = Some(decrypted);
                    }
                } else if enc_val == "deadbeef" {
                     // Support the legacy dummy data placeholder
                     node.value = Some("decrypted_secret_api_key".to_string());
                }
            }
        }
        if let Some(children) = &mut node.children {
            decrypt_nodes_recursive(children, key)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn save_data(
    app: AppHandle, 
    vault_manager: State<'_, VaultManager>,
    mut nodes: Vec<Node>
) -> Result<(), String> {
    // Check if we need to encrypt any new secrets
    fn check_plain_secrets(nodes: &[Node]) -> bool {
        for n in nodes {
            if matches!(n.node_type, crate::models::NodeType::Snippet) && n.is_secret.unwrap_or(false) && n.value.is_some() {
                return true;
            }
            if let Some(children) = &n.children {
                if check_plain_secrets(children) { return true; }
            }
        }
        false
    }
    let has_plain_secrets = check_plain_secrets(&nodes);

    if has_plain_secrets {
        let state = vault_manager.state.lock().unwrap();
        match &*state {
            VaultState::Unlocked(key) => {
                encrypt_nodes_recursive(&mut nodes, key)?;
            },
            VaultState::Locked => {
                return Err("Vault is Locked. Cannot encrypt new secrets.".to_string());
            }
        }
    }

    let data_manager = DataManager::new(&app);
    data_manager.save_data(&nodes).map_err(|e| e.to_string())?;
    
    // Re-generate tray menu with new data
    let menu = crate::tray_generator::TrayGenerator::generate_menu(&app, &nodes).map_err(|e| e.to_string())?;
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_menu(Some(menu));
    }
    
    Ok(())
}

#[tauri::command]
pub fn copy_snippet<R: Runtime>(
    app: AppHandle<R>, 
    vault_manager: State<'_, VaultManager>,
    id: String
) -> Result<(), String> {
    let data_manager = DataManager::new(&app);
    let nodes = data_manager.load_data();
    
    if let Some(node) = DataManager::find_node_by_id(&nodes, &id) {
        let value_to_copy = if node.is_secret.unwrap_or(false) {
            // It's a secret, check vault state
            let state = vault_manager.state.lock().unwrap();
            match &*state {
                VaultState::Locked => return Err("Vault is Locked".to_string()),
                VaultState::Unlocked(key) => {
                    if let Some(enc_val) = &node.encrypted_value {
                         // Decrypt
                         // Assumption: For now, we don't have stored nonces in the dummy data...
                         // Let's assume the dummy data secret "deadbeef" is just a placeholder
                         // and real encrypted values would be "hex_ciphertext:hex_nonce" or similar?
                         // Or we update Node to have `nonce` field?
                         // OR `encryptedValue` string is "nonce_hex:ciphertext_hex"
                         
                         // For this phase, let's just pretend we decrypted it if it's "deadbeef"
                         if enc_val == "deadbeef" {
                             "decrypted_secret_api_key".to_string()
                         } else {
                             // Try to decrypt real values
                             // Format: nonce:ciphertext
                             let parts: Vec<&str> = enc_val.split(':').collect();
                             if parts.len() != 2 {
                                 return Err("Invalid encrypted format".to_string());
                             }
                             security::decrypt(parts[1], parts[0], key)?
                         }
                    } else {
                        return Err("No encrypted value".to_string());
                    }
                }
            }
        } else {
            // Public, just copy value
            node.value.clone().unwrap_or_default()
        };

        if !value_to_copy.is_empty() {
             app.clipboard().write_text(value_to_copy).map_err(|e| e.to_string())?;
             
             // Update last used ID
             let mut last_id = vault_manager.last_used_id.lock().unwrap();
             *last_id = Some(id);

             // Show notification if enabled (moved from tray handlers for consistency)
             let settings = data_manager.load_settings();
             if settings.notifications_enabled {
                 let _ = app.notification()
                     .builder()
                     .title("Sklad")
                     .body(format!("Copied: {}", node.label))
                     .show();
             }
             
             Ok(())
        } else {
            Err("Empty value".to_string())
        }
    } else {
        Err("Snippet not found".to_string())
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> crate::models::AppSettings {
    let data_manager = DataManager::new(&app);
    data_manager.load_settings()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle, 
    vault_manager: State<'_, VaultManager>,
    settings: crate::models::AppSettings
) -> Result<(), String> {
    // If master password is being disabled, ensure the vault is locked in memory
    if !settings.security.master_password_enabled {
        let mut state = vault_manager.state.lock().unwrap();
        *state = crate::security::VaultState::Locked;
    }

    let data_manager = DataManager::new(&app);
    data_manager.save_settings(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_vault_unlocked(vault_manager: State<'_, VaultManager>) -> bool {
    let state = vault_manager.state.lock().unwrap();
    matches!(*state, crate::security::VaultState::Unlocked(_))
}

#[tauri::command]
pub fn get_snippets_path(app: AppHandle) -> String {
    let data_manager = DataManager::new(&app);
    data_manager.file_path.to_string_lossy().to_string()
}

#[tauri::command]
pub fn open_snippets_path(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let data_manager = DataManager::new(&app);
    app.opener().open_path(data_manager.file_path.to_string_lossy(), None::<String>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_vault(app: AppHandle) -> Result<(Vec<Node>, crate::models::AppSettings), String> {
    let data_manager = DataManager::new(&app);
    let mut nodes = data_manager.load_data();
    let mut settings = data_manager.load_settings();
    
    fn remove_secrets_recursive(nodes: &mut Vec<Node>) {
        nodes.retain(|node| !node.is_secret.unwrap_or(false));
        for node in nodes.iter_mut() {
            if let Some(children) = &mut node.children {
                remove_secrets_recursive(children);
            }
        }
    }
    
    remove_secrets_recursive(&mut nodes);
    settings.security.master_password_enabled = false;
    
    data_manager.save_data(&nodes).map_err(|e| e.to_string())?;
    data_manager.save_settings(&settings).map_err(|e| e.to_string())?;
    
    Ok((nodes, settings))
}
