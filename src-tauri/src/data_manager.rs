use crate::models::{BackupInfo, Node};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageFile {
    Data,
    Settings,
}

impl StorageFile {
    fn file_name(self) -> &'static str {
        match self {
            Self::Data => "sklad.json",
            Self::Settings => "settings.json",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageIssueKind {
    InvalidFormat,
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageIssue {
    pub file: StorageFile,
    pub kind: StorageIssueKind,
    pub file_name: String,
    pub reason: String,
}

impl StorageIssue {
    fn invalid(file: StorageFile, error: serde_json::Error) -> Self {
        Self {
            file,
            kind: StorageIssueKind::InvalidFormat,
            file_name: file.file_name().to_string(),
            reason: format!(
                "JSON does not match the expected Sklad format (line {}, column {})",
                error.line(),
                error.column()
            ),
        }
    }

    fn unreadable(file: StorageFile, error: io::Error) -> Self {
        Self {
            file,
            kind: StorageIssueKind::Unreadable,
            file_name: file.file_name().to_string(),
            reason: error.to_string(),
        }
    }

    fn invalid_reason(file: StorageFile, reason: impl Into<String>) -> Self {
        Self {
            file,
            kind: StorageIssueKind::InvalidFormat,
            file_name: file.file_name().to_string(),
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for StorageIssue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let description = match self.kind {
            StorageIssueKind::InvalidFormat => "has an invalid format",
            StorageIssueKind::Unreadable => "could not be read",
        };
        write!(
            formatter,
            "{} {}: {}",
            self.file_name, description, self.reason
        )
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatus {
    pub data_issue: Option<StorageIssue>,
    pub settings_issue: Option<StorageIssue>,
    pub newest_valid_backup: Option<BackupInfo>,
    pub has_encrypted_secrets: bool,
}

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

    pub fn load_data(&self) -> Result<Vec<Node>, StorageIssue> {
        if !self.file_path.exists() {
            let defaults = Self::default_nodes();
            // Save defaults to disk so the file exists for "Open File"
            self.save_data(&defaults)
                .map_err(|error| StorageIssue::unreadable(StorageFile::Data, error))?;
            return Ok(defaults);
        }

        Self::read_json(&self.file_path, StorageFile::Data)
    }

    pub fn save_data(&self, nodes: &[Node]) -> Result<(), std::io::Error> {
        let content = serde_json::to_string_pretty(nodes)?;
        Self::atomic_write(&self.file_path, content.as_bytes())
    }

    pub fn load_settings(&self) -> Result<crate::models::AppSettings, StorageIssue> {
        let settings_path = self.settings_path();
        if !settings_path.exists() {
            return Ok(crate::models::AppSettings::default());
        }

        let settings = Self::read_json(&settings_path, StorageFile::Settings)?;
        Self::validate_settings(settings)
    }

    pub fn save_settings(
        &self,
        settings: &crate::models::AppSettings,
    ) -> Result<(), std::io::Error> {
        let settings_path = self.settings_path();
        let content = serde_json::to_string_pretty(settings)?;
        Self::atomic_write(&settings_path, content.as_bytes())
    }

    pub fn storage_status(&self) -> StorageStatus {
        let data_issue = self.data_issue();
        let settings_issue = self.settings_issue();
        let newest_valid_backup = self.newest_valid_backup();

        let has_encrypted_secrets = if data_issue.is_none() && self.file_path.exists() {
            Self::read_json::<Vec<Node>>(&self.file_path, StorageFile::Data)
                .map(|nodes| Self::has_encrypted_secrets(&nodes))
                .unwrap_or(false)
        } else {
            newest_valid_backup
                .as_ref()
                .and_then(|backup| self.read_backup(&backup.filename).ok())
                .map(|nodes| Self::has_encrypted_secrets(&nodes))
                .unwrap_or(false)
        } || (data_issue.is_some() && settings_issue.is_some());

        StorageStatus {
            data_issue,
            settings_issue,
            newest_valid_backup,
            has_encrypted_secrets,
        }
    }

    pub fn has_storage_issues(&self) -> bool {
        self.data_issue().is_some() || self.settings_issue().is_some()
    }

    pub fn reset_invalid_data(&self) -> Result<String, String> {
        Self::require_invalid_issue(self.data_issue(), StorageFile::Data)?;
        let quarantined = self.quarantine_file(&self.file_path, "sklad")?;
        self.save_data(&Self::default_nodes())
            .map_err(|error| format!("Failed to create fresh data: {}", error))?;
        Ok(quarantined)
    }

    pub fn reset_invalid_settings(&self) -> Result<String, String> {
        Self::require_invalid_issue(self.settings_issue(), StorageFile::Settings)?;
        let settings_path = self.settings_path();
        let quarantined = self.quarantine_file(&settings_path, "settings")?;
        self.save_settings(&crate::models::AppSettings::default())
            .map_err(|error| format!("Failed to create fresh settings: {}", error))?;
        Ok(quarantined)
    }

    pub fn create_backup(&self) -> Result<(), std::io::Error> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let content = fs::read(&self.file_path)?;
        serde_json::from_slice::<Vec<Node>>(&content).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Cannot back up invalid sklad.json (line {}, column {})",
                    error.line(),
                    error.column()
                ),
            )
        })?;

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

    pub fn newest_valid_backup(&self) -> Option<BackupInfo> {
        self.list_backups()
            .into_iter()
            .find(|backup| self.read_backup(&backup.filename).is_ok())
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
            fs::read(&backup_path).map_err(|error| format!("Failed to read backup: {}", error))?;

        serde_json::from_slice::<Vec<Node>>(&content).map_err(|error| {
            format!(
                "Invalid backup format at line {}, column {}",
                error.line(),
                error.column()
            )
        })?;

        if self.file_path.exists() {
            match Self::read_json::<Vec<Node>>(&self.file_path, StorageFile::Data) {
                Ok(_) => self
                    .create_backup()
                    .map_err(|e| format!("Failed to preserve current data: {}", e))?,
                Err(issue) if issue.kind == StorageIssueKind::InvalidFormat => {
                    self.quarantine_file(&self.file_path, "sklad")?;
                }
                Err(issue) => {
                    return Err(format!("Failed to preserve current data: {}", issue));
                }
            }
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

    fn settings_path(&self) -> PathBuf {
        self.file_path.with_file_name("settings.json")
    }

    fn data_issue(&self) -> Option<StorageIssue> {
        if !self.file_path.exists() {
            return None;
        }
        Self::read_json::<Vec<Node>>(&self.file_path, StorageFile::Data).err()
    }

    fn settings_issue(&self) -> Option<StorageIssue> {
        let settings_path = self.settings_path();
        if !settings_path.exists() {
            return None;
        }
        self.load_settings().err()
    }

    fn read_json<T: DeserializeOwned>(
        path: &Path,
        storage_file: StorageFile,
    ) -> Result<T, StorageIssue> {
        let content =
            fs::read(path).map_err(|error| StorageIssue::unreadable(storage_file, error))?;
        serde_json::from_slice(&content).map_err(|error| StorageIssue::invalid(storage_file, error))
    }

    fn validate_settings(
        settings: crate::models::AppSettings,
    ) -> Result<crate::models::AppSettings, StorageIssue> {
        if settings.security.master_password_enabled
            && (settings.security.password_hash.is_none()
                || settings.security.derivation_salt.is_none())
        {
            return Err(StorageIssue::invalid_reason(
                StorageFile::Settings,
                "Vault settings are incomplete",
            ));
        }

        Ok(settings)
    }

    fn read_backup(&self, filename: &str) -> Result<Vec<Node>, String> {
        if Self::backup_timestamp(filename).is_none()
            || filename.contains('/')
            || filename.contains('\\')
        {
            return Err("Invalid backup filename".to_string());
        }

        let content = fs::read(self.backups_dir.join(filename))
            .map_err(|error| format!("Failed to read backup: {}", error))?;
        serde_json::from_slice(&content).map_err(|error| {
            format!(
                "Invalid backup format at line {}, column {}",
                error.line(),
                error.column()
            )
        })
    }

    fn require_invalid_issue(
        issue: Option<StorageIssue>,
        storage_file: StorageFile,
    ) -> Result<(), String> {
        match issue {
            Some(issue) if issue.kind == StorageIssueKind::InvalidFormat => Ok(()),
            Some(issue) => Err(format!(
                "{} cannot be quarantined automatically: {}",
                issue.file_name, issue.reason
            )),
            None => Err(format!(
                "{} is valid; recovery is not required",
                storage_file.file_name()
            )),
        }
    }

    fn quarantine_file(&self, source: &Path, base_name: &str) -> Result<String, String> {
        let content = fs::read(source)
            .map_err(|error| format!("Failed to read file for quarantine: {}", error))?;
        let mut timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let quarantine_path = loop {
            let candidate =
                source.with_file_name(format!("{}.corrupt_{}.json", base_name, timestamp));
            if !candidate.exists() {
                break candidate;
            }
            timestamp += 1;
        };

        Self::atomic_write(&quarantine_path, &content)
            .map_err(|error| format!("Failed to create quarantine copy: {}", error))?;

        quarantine_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .ok_or_else(|| "Failed to resolve quarantine filename".to_string())
    }

    fn has_encrypted_secrets(nodes: &[Node]) -> bool {
        nodes.iter().any(|node| {
            node.encrypted_value.is_some()
                || node
                    .children
                    .as_deref()
                    .is_some_and(Self::has_encrypted_secrets)
        })
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
    use super::{DataManager, StorageIssueKind};
    use crate::models::{AppSettings, Node, NodeType};

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

        assert_eq!(manager.load_data().unwrap()[0].label, "saved");
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

        assert_eq!(manager.load_data().unwrap()[0].label, "old");
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

        assert!(error.starts_with("Invalid backup format"));
        assert_eq!(manager.load_data().unwrap()[0].label, "current");
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

    #[test]
    fn missing_data_is_initialized_as_a_first_run() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));

        let nodes = manager.load_data().unwrap();

        assert_eq!(nodes[0].label, "Welcome to Sklad");
        assert!(manager.file_path.exists());
    }

    #[test]
    fn invalid_data_is_reported_without_being_replaced() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let invalid_content = b"{not valid json";
        std::fs::write(&manager.file_path, invalid_content).unwrap();

        let issue = manager.load_data().unwrap_err();

        assert_eq!(issue.kind, StorageIssueKind::InvalidFormat);
        assert_eq!(std::fs::read(&manager.file_path).unwrap(), invalid_content);
        assert!(manager.list_backups().is_empty());
    }

    #[test]
    fn invalid_data_diagnostics_do_not_include_file_values() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let sensitive_marker = "do-not-log-this-value";
        let content = format!(
            r#"[{{"id":"1","type":"{}","label":"label","parentId":null,"createdAt":0}}]"#,
            sensitive_marker
        );
        std::fs::write(&manager.file_path, content).unwrap();

        let issue = manager.load_data().unwrap_err();

        assert!(!issue.reason.contains(sensitive_marker));
        assert!(!issue.to_string().contains(sensitive_marker));
    }

    #[test]
    fn resetting_invalid_data_preserves_an_exact_quarantine_copy() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let invalid_content = b"[invalid data]";
        std::fs::write(&manager.file_path, invalid_content).unwrap();

        let quarantine_filename = manager.reset_invalid_data().unwrap();

        let quarantine_path = manager.file_path.with_file_name(quarantine_filename);
        assert_eq!(std::fs::read(quarantine_path).unwrap(), invalid_content);
        assert_eq!(manager.load_data().unwrap()[0].label, "Welcome to Sklad");
    }

    #[test]
    fn restore_quarantines_invalid_current_data_before_replacement() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let invalid_content = b"not json";
        std::fs::write(&manager.file_path, invalid_content).unwrap();
        let backup_filename = "sklad_backup_123.json";
        let backup_content = serde_json::to_vec(&vec![snippet("recovered")]).unwrap();
        std::fs::write(manager.backups_dir.join(backup_filename), backup_content).unwrap();

        manager.restore_backup(backup_filename).unwrap();

        assert_eq!(manager.load_data().unwrap()[0].label, "recovered");
        let quarantine = std::fs::read_dir(manager.file_path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("sklad.corrupt_")
            })
            .unwrap();
        assert_eq!(std::fs::read(quarantine.path()).unwrap(), invalid_content);
    }

    #[test]
    fn newest_valid_backup_skips_a_newer_invalid_file() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        std::fs::write(
            manager.backups_dir.join("sklad_backup_123.json"),
            serde_json::to_vec(&vec![snippet("valid")]).unwrap(),
        )
        .unwrap();
        std::fs::write(
            manager.backups_dir.join("sklad_backup_124.json"),
            b"invalid",
        )
        .unwrap();

        let backup = manager.newest_valid_backup().unwrap();

        assert_eq!(backup.filename, "sklad_backup_123.json");
    }

    #[test]
    fn invalid_data_is_not_copied_into_normal_backups() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        std::fs::write(&manager.file_path, b"invalid").unwrap();

        let error = manager.create_backup().unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(manager.list_backups().is_empty());
    }

    #[test]
    fn older_settings_with_missing_fields_remain_compatible() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        std::fs::write(
            manager.settings_path(),
            br#"{
                "theme":"dark",
                "security":{
                    "lockTimeout":300000,
                    "clearClipboard":false,
                    "masterPasswordEnabled":false,
                    "passwordHash":null,
                    "derivationSalt":null
                },
                "notificationsEnabled":true
            }"#,
        )
        .unwrap();

        let settings = manager.load_settings().unwrap();

        assert_eq!(settings.theme, "dark");
        assert!(!settings.security.master_password_enabled);
        assert_eq!(settings.auto_backup_count, 5);
    }

    #[test]
    fn incomplete_vault_settings_require_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        std::fs::write(
            manager.settings_path(),
            br#"{
                "theme":"dark",
                "security":{
                    "lockTimeout":300000,
                    "clearClipboard":false,
                    "masterPasswordEnabled":true,
                    "passwordHash":"hash",
                    "derivationSalt":null
                },
                "notificationsEnabled":true
            }"#,
        )
        .unwrap();

        let issue = manager.load_settings().unwrap_err();

        assert_eq!(issue.kind, StorageIssueKind::InvalidFormat);
        assert_eq!(issue.reason, "Vault settings are incomplete");
    }

    #[test]
    fn resetting_invalid_settings_preserves_an_exact_quarantine_copy() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let invalid_content = b"{invalid settings";
        std::fs::write(manager.settings_path(), invalid_content).unwrap();

        let issue = manager.load_settings().unwrap_err();
        assert_eq!(issue.kind, StorageIssueKind::InvalidFormat);
        assert_eq!(
            std::fs::read(manager.settings_path()).unwrap(),
            invalid_content
        );

        let quarantine_filename = manager.reset_invalid_settings().unwrap();
        let quarantine_path = manager.file_path.with_file_name(quarantine_filename);
        assert_eq!(std::fs::read(quarantine_path).unwrap(), invalid_content);
        assert_eq!(
            manager.load_settings().unwrap().theme,
            AppSettings::default().theme
        );
    }

    #[test]
    fn unreadable_data_is_not_replaced_by_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        std::fs::create_dir(&manager.file_path).unwrap();

        let status = manager.storage_status();

        assert_eq!(
            status.data_issue.unwrap().kind,
            StorageIssueKind::Unreadable
        );
        assert!(manager.reset_invalid_data().is_err());
        assert!(manager.file_path.is_dir());
    }
}
