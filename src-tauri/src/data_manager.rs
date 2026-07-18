use crate::models::{BackupInfo, Node};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};
use tempfile::NamedTempFile;

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

        Self::from_app_data_dir(app_data_dir)
    }

    fn from_app_data_dir(app_data_dir: PathBuf) -> Self {
        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
        }

        let backups_dir = app_data_dir.join("backups");
        if !backups_dir.exists() {
            fs::create_dir_all(&backups_dir).expect("failed to create backups dir");
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
        Self::atomic_write(&self.file_path, content.as_bytes())
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
        Self::atomic_write(&settings_path, content.as_bytes())
    }

    pub fn create_backup(&self) -> Result<(), std::io::Error> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let mut timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let backup_path = loop {
            let candidate = self
                .backups_dir
                .join(format!("sklad_backup_{}.json", timestamp));
            if !candidate.exists() {
                break candidate;
            }
            timestamp += 1;
        };

        let content = fs::read(&self.file_path)?;
        Self::atomic_write(&backup_path, &content)?;

        log::info!("Created backup: {:?}", backup_path);
        Ok(())
    }

    pub fn rotate_backups(&self, keep_count: u32) -> Result<(), std::io::Error> {
        let mut backups: Vec<(i64, PathBuf)> = Vec::new();

        for entry in fs::read_dir(&self.backups_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(timestamp) = path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(Self::backup_timestamp)
            {
                backups.push((timestamp, path));
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
                if let Some(filename) = path.file_name().and_then(|name| name.to_str()) {
                    if let Some(timestamp) = Self::backup_timestamp(filename) {
                        let size = entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
                        backups.push(BackupInfo {
                            filename: filename.to_string(),
                            timestamp,
                            size,
                        });
                    }
                }
            }
        }

        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        backups
    }

    pub fn restore_backup(&self, filename: &str) -> Result<(), String> {
        if Self::backup_timestamp(filename).is_none()
            || filename.contains('/')
            || filename.contains('\\')
        {
            return Err("Invalid backup filename".to_string());
        }

        let backup_path = self.backups_dir.join(filename);

        if !backup_path.exists() {
            return Err("Backup file not found".to_string());
        }

        let content =
            fs::read(&backup_path).map_err(|e| format!("Failed to read backup: {}", e))?;

        serde_json::from_slice::<Vec<Node>>(&content)
            .map_err(|e| format!("Invalid backup format: {}", e))?;

        if self.file_path.exists() {
            self.create_backup()
                .map_err(|e| format!("Failed to preserve current data: {}", e))?;
        }

        Self::atomic_write(&self.file_path, &content)
            .map_err(|e| format!("Failed to restore backup: {}", e))?;

        log::info!("Restored backup: {:?}", backup_path);
        Ok(())
    }

    fn backup_timestamp(filename: &str) -> Option<i64> {
        filename
            .strip_prefix("sklad_backup_")?
            .strip_suffix(".json")?
            .parse::<i64>()
            .ok()
    }

    fn atomic_write(path: &Path, content: &[u8]) -> io::Result<()> {
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "file path has no parent directory",
            )
        })?;
        fs::create_dir_all(parent)?;

        let mut temporary = NamedTempFile::new_in(parent)?;
        temporary.write_all(content)?;
        temporary.as_file().sync_all()?;
        temporary.persist(path).map_err(|error| error.error)?;

        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }

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

#[cfg(test)]
mod tests {
    use super::DataManager;
    use crate::models::{Node, NodeType};

    fn snippet(label: &str) -> Node {
        Node {
            id: format!("{}-id", label),
            node_type: NodeType::Snippet,
            label: label.to_string(),
            parent_id: None,
            created_at: 0,
            children: None,
            value: Some(label.to_string()),
            encrypted_value: None,
            is_secret: Some(false),
        }
    }

    #[test]
    fn save_data_round_trips_without_leaving_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));

        manager.save_data(&[snippet("saved")]).unwrap();

        assert_eq!(manager.load_data()[0].label, "saved");
        let app_entries = std::fs::read_dir(manager.file_path.parent().unwrap()).unwrap();
        assert!(app_entries
            .filter_map(Result::ok)
            .all(|entry| !entry.file_name().to_string_lossy().starts_with(".tmp")));
    }

    #[test]
    fn backups_use_unique_names() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        manager.save_data(&[snippet("current")]).unwrap();

        manager.create_backup().unwrap();
        manager.create_backup().unwrap();

        let backups = manager.list_backups();
        assert_eq!(backups.len(), 2);
        assert_ne!(backups[0].filename, backups[1].filename);
    }

    #[test]
    fn restore_preserves_current_data_as_a_safety_backup() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        manager.save_data(&[snippet("old")]).unwrap();
        manager.create_backup().unwrap();
        let old_backup = manager.list_backups()[0].filename.clone();
        manager.save_data(&[snippet("current")]).unwrap();

        manager.restore_backup(&old_backup).unwrap();

        assert_eq!(manager.load_data()[0].label, "old");
        let safety_backup = &manager.list_backups()[0];
        let safety_content =
            std::fs::read(manager.backups_dir.join(&safety_backup.filename)).unwrap();
        let safety_nodes: Vec<Node> = serde_json::from_slice(&safety_content).unwrap();
        assert_eq!(safety_nodes[0].label, "current");
    }

    #[test]
    fn restore_rejects_invalid_content_without_replacing_current_data() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        manager.save_data(&[snippet("current")]).unwrap();
        let invalid_backup = manager.backups_dir.join("sklad_backup_123.json");
        std::fs::write(invalid_backup, b"not json").unwrap();

        let error = manager.restore_backup("sklad_backup_123.json").unwrap_err();

        assert!(error.starts_with("Invalid backup format:"));
        assert_eq!(manager.load_data()[0].label, "current");
    }

    #[test]
    fn restore_rejects_paths_outside_the_backups_directory() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));

        assert_eq!(
            manager.restore_backup("../sklad_backup_123.json"),
            Err("Invalid backup filename".to_string())
        );
        assert_eq!(
            manager.restore_backup("..\\sklad_backup_123.json"),
            Err("Invalid backup filename".to_string())
        );
    }
}
