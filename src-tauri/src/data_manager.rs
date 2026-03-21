use crate::models::{BackupInfo, Node};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

pub struct DataManager {
    pub file_path: PathBuf,
    backups_dir: PathBuf,
}

impl DataManager {
    pub fn new<R: Runtime>(app: &AppHandle<R>) -> Self {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .expect("failed to resolve app data dir");

        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
        }

        let backups_dir = app_data_dir.join("backups");
        if !backups_dir.exists() {
            let _ = fs::create_dir_all(&backups_dir);
        }

        Self {
            file_path: app_data_dir.join("sklad.json"),
            backups_dir,
        }
    }

    pub fn load_data(&self) -> Vec<Node> {
        if !self.file_path.exists() {
            let defaults = Self::default_nodes();
            // Save defaults to disk so the file exists for "Open File"
            let _ = self.save_data(&defaults);
            return defaults;
        }

        let content = fs::read_to_string(&self.file_path).unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save_data(&self, nodes: &[Node]) -> Result<(), std::io::Error> {
        let content = serde_json::to_string_pretty(nodes)?;
        fs::write(&self.file_path, content)
    }

    pub fn load_settings(&self) -> crate::models::AppSettings {
        let settings_path = self.file_path.with_file_name("settings.json");
        if !settings_path.exists() {
            return crate::models::AppSettings::default();
        }

        fs::read_to_string(&settings_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(
        &self,
        settings: &crate::models::AppSettings,
    ) -> Result<(), std::io::Error> {
        let settings_path = self.file_path.with_file_name("settings.json");
        let content = serde_json::to_string_pretty(settings)?;
        fs::write(settings_path, content)
    }

    pub fn create_backup(&self) -> Result<(), std::io::Error> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let backup_name = format!("sklad_backup_{}.json", timestamp);
        let backup_path = self.backups_dir.join(&backup_name);

        fs::copy(&self.file_path, &backup_path)?;

        log::info!("Created backup: {:?}", backup_path);
        Ok(())
    }

    pub fn rotate_backups(&self, keep_count: u32) -> Result<(), std::io::Error> {
        let mut backups: Vec<(i64, PathBuf)> = Vec::new();

        for entry in fs::read_dir(&self.backups_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem.starts_with("sklad_backup_") {
                        if let Some(timestamp_str) = stem.strip_prefix("sklad_backup_") {
                            if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                                backups.push((timestamp, path));
                            }
                        }
                    }
                }
            }
        }

        backups.sort_by(|a, b| b.0.cmp(&a.0));

        let to_delete = backups.iter().skip(keep_count as usize);

        for (_, path) in to_delete {
            if let Err(e) = fs::remove_file(path) {
                log::error!("Failed to delete old backup {:?}: {}", path, e);
            } else {
                log::info!("Deleted old backup: {:?}", path);
            }
        }

        Ok(())
    }

    pub fn list_backups(&self) -> Vec<BackupInfo> {
        let mut backups: Vec<BackupInfo> = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.backups_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if stem.starts_with("sklad_backup_") {
                            if let Some(timestamp_str) = stem.strip_prefix("sklad_backup_") {
                                if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                                    if let Some(filename) =
                                        path.file_name().and_then(|n| n.to_str())
                                    {
                                        backups.push(BackupInfo {
                                            filename: filename.to_string(),
                                            timestamp,
                                            size,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        backups
    }

    pub fn restore_backup(&self, filename: &str) -> Result<(), String> {
        let backup_path = self.backups_dir.join(filename);

        if !backup_path.exists() {
            return Err("Backup file not found".to_string());
        }

        if !self.file_path.exists() {
            return Err("No current data file to restore over".to_string());
        }

        let content = fs::read_to_string(&backup_path)
            .map_err(|e| format!("Failed to read backup: {}", e))?;

        serde_json::from_str::<Vec<Node>>(&content)
            .map_err(|e| format!("Invalid backup format: {}", e))?;

        fs::copy(&backup_path, &self.file_path)
            .map_err(|e| format!("Failed to restore backup: {}", e))?;

        log::info!("Restored backup: {:?}", backup_path);
        Ok(())
    }

    pub fn find_node_by_id(nodes: &[Node], id: &str) -> Option<Node> {
        for node in nodes {
            if node.id == id {
                return Some(node.clone());
            }
            if let Some(children) = &node.children {
                if let Some(found) = Self::find_node_by_id(children, id) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn default_nodes() -> Vec<Node> {
        vec![Node {
            id: "welcome-1".to_string(),
            node_type: crate::models::NodeType::Snippet,
            label: "Welcome to Sklad".to_string(),
            parent_id: None,
            created_at: 0,
            children: None,
            value: Some("This is your first snippet.".to_string()),
            encrypted_value: None,
            is_secret: Some(false),
        }]
    }
}
