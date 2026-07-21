use crate::models::{AppSettings, BackupInfo, Node};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};
use tempfile::NamedTempFile;

const DATA_FILE_VERSION: u32 = 1;
const DERIVATION_SALT_BYTES: usize = 16;

fn derivation_salt_is_valid(salt: &str) -> bool {
    hex::decode(salt).is_ok_and(|bytes| bytes.len() == DERIVATION_SALT_BYTES)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VaultMetadata {
    password_hash: String,
    derivation_salt: String,
}

impl VaultMetadata {
    fn from_settings(settings: &AppSettings) -> Option<Self> {
        if !settings.security.master_password_enabled {
            return None;
        }

        Some(Self {
            password_hash: settings.security.password_hash.clone()?,
            derivation_salt: settings.security.derivation_salt.clone()?,
        })
    }

    fn apply_to_settings(&self, settings: &mut AppSettings) {
        settings.security.master_password_enabled = true;
        settings.security.password_hash = Some(self.password_hash.clone());
        settings.security.derivation_salt = Some(self.derivation_salt.clone());
    }

    fn is_complete(&self) -> bool {
        !self.password_hash.is_empty() && derivation_salt_is_valid(&self.derivation_salt)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionedData {
    version: u32,
    nodes: Vec<Node>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vault_metadata: Option<VaultMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PersistedData {
    Legacy(Vec<Node>),
    Versioned(VersionedData),
}

#[derive(Debug, Clone)]
struct DataContents {
    nodes: Vec<Node>,
    vault_metadata: Option<VaultMetadata>,
    is_versioned: bool,
}

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
    VaultMetadata,
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

    fn vault_metadata(reason: impl Into<String>) -> Self {
        Self {
            file: StorageFile::Settings,
            kind: StorageIssueKind::VaultMetadata,
            file_name: StorageFile::Settings.file_name().to_string(),
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for StorageIssue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let description = match self.kind {
            StorageIssueKind::InvalidFormat => "has an invalid format",
            StorageIssueKind::Unreadable => "could not be read",
            StorageIssueKind::VaultMetadata => "has missing or inconsistent vault metadata",
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
    pub newest_vault_backup: Option<BackupInfo>,
    pub has_encrypted_secrets: bool,
    pub vault_metadata_recoverable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultRecoveryResult {
    pub removed_secret_count: usize,
    pub data_recovery_copy: Option<String>,
    pub settings_recovery_copy: Option<String>,
    pub restored_from_backup: Option<String>,
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

        Self::read_data_file(&self.file_path, StorageFile::Data).map(|contents| contents.nodes)
    }

    pub fn save_data(&self, nodes: &[Node]) -> Result<(), std::io::Error> {
        let vault_metadata = if Self::has_encrypted_secrets(nodes) {
            let settings = self.load_settings().map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Cannot save encrypted snippets: {}", error),
                )
            })?;
            Some(VaultMetadata::from_settings(&settings).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Cannot save encrypted snippets without complete vault metadata",
                )
            })?)
        } else {
            None
        };

        self.save_data_with_metadata(nodes, vault_metadata.as_ref())
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
        let validated = Self::validate_settings(settings.clone()).map_err(|issue| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Cannot save invalid settings: {}", issue.reason),
            )
        })?;
        let content = serde_json::to_string_pretty(&validated)?;
        Self::atomic_write(&settings_path, content.as_bytes())
    }

    pub fn storage_status(&self) -> StorageStatus {
        let active_data = if self.file_path.exists() {
            Some(Self::read_data_file(&self.file_path, StorageFile::Data))
        } else {
            None
        };
        let data_issue = active_data
            .as_ref()
            .and_then(|result| result.as_ref().err().cloned());
        let mut settings_issue = self.settings_issue();
        let newest_valid_backup = self.newest_valid_backup();
        let newest_vault_backup = self.newest_vault_backup();

        let active_contents = active_data.as_ref().and_then(|result| result.as_ref().ok());
        let fallback_contents = if active_contents.is_none() {
            newest_vault_backup
                .as_ref()
                .or(newest_valid_backup.as_ref())
                .and_then(|backup| self.read_backup(&backup.filename).ok())
        } else {
            None
        };
        let relevant_contents = active_contents.or(fallback_contents.as_ref());

        let has_encrypted_secrets = relevant_contents
            .is_some_and(|contents| Self::has_encrypted_secrets(&contents.nodes))
            || (data_issue.is_some() && settings_issue.is_some());

        let active_vault_metadata = active_contents
            .filter(|contents| Self::has_encrypted_secrets(&contents.nodes))
            .and_then(|contents| contents.vault_metadata.as_ref());

        if settings_issue.is_none() && has_encrypted_secrets {
            if !self.settings_path().exists() {
                settings_issue = Some(StorageIssue::vault_metadata(
                    "Encrypted snippets exist, but settings.json is missing",
                ));
            } else if let Ok(settings) = self.load_settings() {
                if !settings.security.master_password_enabled {
                    settings_issue = Some(StorageIssue::vault_metadata(
                        "Encrypted snippets exist, but the vault is disabled in settings.json",
                    ));
                } else if active_vault_metadata.is_some_and(|metadata| {
                    VaultMetadata::from_settings(&settings).as_ref() != Some(metadata)
                }) {
                    settings_issue = Some(StorageIssue::vault_metadata(
                        "Vault metadata in settings.json does not match the encrypted snippet data",
                    ));
                }
            }
        }

        let vault_metadata_recoverable = data_issue.is_none()
            && settings_issue.is_some()
            && active_vault_metadata.is_some_and(VaultMetadata::is_complete);

        StorageStatus {
            data_issue,
            settings_issue,
            newest_valid_backup,
            newest_vault_backup,
            has_encrypted_secrets,
            vault_metadata_recoverable,
        }
    }

    pub fn ensure_vault_metadata_redundancy(&self) -> Result<bool, String> {
        if !self.file_path.exists() {
            return Ok(false);
        }

        let contents = Self::read_data_file(&self.file_path, StorageFile::Data)
            .map_err(|issue| issue.to_string())?;
        if !Self::has_encrypted_secrets(&contents.nodes) {
            return Ok(false);
        }

        let settings = self.load_settings().map_err(|issue| issue.to_string())?;
        let metadata = VaultMetadata::from_settings(&settings)
            .ok_or_else(|| "Encrypted snippets require complete vault metadata".to_string())?;
        if contents.is_versioned && contents.vault_metadata.as_ref() == Some(&metadata) {
            return Ok(false);
        }

        if !contents.is_versioned {
            self.preserve_file(&self.file_path, "sklad", "pre_metadata_migration")?;
        }

        self.save_data_with_metadata(&contents.nodes, Some(&metadata))
            .map_err(|error| format!("Failed to make vault metadata recoverable: {}", error))?;
        Ok(true)
    }

    pub fn has_storage_issues(&self) -> bool {
        let status = self.storage_status();
        status.data_issue.is_some() || status.settings_issue.is_some()
    }

    pub fn ensure_storage_healthy(&self) -> Result<(), String> {
        let status = self.storage_status();
        status
            .data_issue
            .or(status.settings_issue)
            .map_or(Ok(()), |issue| Err(issue.to_string()))
    }

    pub fn reset_invalid_data(&self) -> Result<String, String> {
        Self::require_invalid_issue(self.data_issue(), StorageFile::Data)?;
        let quarantined = self.quarantine_file(&self.file_path, "sklad")?;
        self.save_data(&Self::default_nodes())
            .map_err(|error| format!("Failed to create fresh data: {}", error))?;
        Ok(quarantined)
    }

    pub fn reset_invalid_settings(&self) -> Result<String, String> {
        let status = self.storage_status();
        if status.has_encrypted_secrets {
            return Err(
                "Encrypted snippets may depend on these settings; use vault recovery instead"
                    .to_string(),
            );
        }
        Self::require_invalid_issue(status.settings_issue, StorageFile::Settings)?;
        let settings_path = self.settings_path();
        let quarantined = self.quarantine_file(&settings_path, "settings")?;
        self.save_settings(&crate::models::AppSettings::default())
            .map_err(|error| format!("Failed to create fresh settings: {}", error))?;
        Ok(quarantined)
    }

    pub fn recover_vault_metadata(&self) -> Result<Option<String>, String> {
        let status = self.storage_status();
        if status.data_issue.is_some() {
            return Err("Recover snippet data before restoring vault metadata".to_string());
        }
        if !status.vault_metadata_recoverable {
            return Err("No recoverable vault metadata is available in sklad.json".to_string());
        }

        let contents = Self::read_data_file(&self.file_path, StorageFile::Data)
            .map_err(|issue| issue.to_string())?;
        let metadata = contents
            .vault_metadata
            .ok_or_else(|| "sklad.json does not contain vault recovery metadata".to_string())?;

        let settings_path = self.settings_path();
        let (mut settings, recovery_copy) = if settings_path.exists() {
            let copy = self.preserve_file(&settings_path, "settings", "vault_recovery")?;
            let settings = match self.load_settings() {
                Ok(settings) => settings,
                Err(issue) if issue.kind == StorageIssueKind::InvalidFormat => {
                    AppSettings::default()
                }
                Err(issue) => {
                    return Err(format!(
                        "{} cannot be recovered automatically: {}",
                        issue.file_name, issue.reason
                    ));
                }
            };
            (settings, Some(copy))
        } else {
            (AppSettings::default(), None)
        };

        metadata.apply_to_settings(&mut settings);
        self.save_settings(&settings)
            .map_err(|error| format!("Failed to restore vault settings: {}", error))?;

        Ok(recovery_copy)
    }

    pub fn discard_unrecoverable_vault_data(&self) -> Result<VaultRecoveryResult, String> {
        let status = self.storage_status();
        if status.data_issue.is_some() {
            return Err("Recover snippet data before resetting the unavailable vault".to_string());
        }
        if !status.has_encrypted_secrets {
            return Err("No encrypted snippets require vault recovery".to_string());
        }

        let settings_issue = status
            .settings_issue
            .ok_or_else(|| "Vault metadata is healthy; recovery is not required".to_string())?;
        if !matches!(
            settings_issue.kind,
            StorageIssueKind::InvalidFormat | StorageIssueKind::VaultMetadata
        ) {
            return Err(format!(
                "{} cannot be recovered automatically: {}",
                settings_issue.file_name, settings_issue.reason
            ));
        }

        let (mut nodes, restored_from_backup) = if self.file_path.exists() {
            (
                Self::read_data_file(&self.file_path, StorageFile::Data)
                    .map_err(|issue| issue.to_string())?
                    .nodes,
                None,
            )
        } else {
            let backup = status
                .newest_valid_backup
                .ok_or_else(|| "No valid snippet data is available for recovery".to_string())?;
            let contents = self.read_backup(&backup.filename)?;
            (contents.nodes, Some(backup.filename))
        };

        let removed_secret_count = Self::remove_unrecoverable_secrets(&mut nodes);
        let data_recovery_copy = if self.file_path.exists() {
            Some(self.preserve_file(&self.file_path, "sklad", "vault_recovery")?)
        } else {
            None
        };
        let settings_path = self.settings_path();
        let settings_recovery_copy = if settings_path.exists() {
            Some(self.preserve_file(&settings_path, "settings", "vault_recovery")?)
        } else {
            None
        };

        self.save_settings(&crate::models::AppSettings::default())
            .map_err(|error| format!("Failed to reset vault settings: {}", error))?;
        self.save_data(&nodes)
            .map_err(|error| format!("Failed to save recovered snippet data: {}", error))?;

        Ok(VaultRecoveryResult {
            removed_secret_count,
            data_recovery_copy,
            settings_recovery_copy,
            restored_from_backup,
        })
    }

    pub fn create_backup(&self) -> Result<(), std::io::Error> {
        if !self.file_path.exists() {
            return Ok(());
        }

        let contents =
            Self::read_data_file(&self.file_path, StorageFile::Data).map_err(|issue| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Cannot back up invalid sklad.json: {}", issue.reason),
                )
            })?;
        let vault_metadata = if Self::has_encrypted_secrets(&contents.nodes) {
            match contents.vault_metadata {
                Some(metadata) => Some(metadata),
                None => {
                    let settings = self.load_settings().map_err(|issue| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Cannot back up encrypted snippets: {}", issue),
                        )
                    })?;
                    Some(VaultMetadata::from_settings(&settings).ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Cannot back up encrypted snippets without complete vault metadata",
                        )
                    })?)
                }
            }
        } else {
            None
        };
        let content = Self::serialize_data(&contents.nodes, vault_metadata.as_ref())?;

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
                            has_vault_metadata: self
                                .read_backup(filename)
                                .is_ok_and(|contents| contents.vault_metadata.is_some()),
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

    pub fn newest_vault_backup(&self) -> Option<BackupInfo> {
        self.list_backups().into_iter().find(|backup| {
            self.read_backup(&backup.filename).is_ok_and(|contents| {
                contents.vault_metadata.is_some() && Self::has_encrypted_secrets(&contents.nodes)
            })
        })
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

        let mut backup = self.read_backup(filename)?;
        if Self::has_encrypted_secrets(&backup.nodes) && backup.vault_metadata.is_none() {
            let settings = self.load_settings().map_err(|issue| {
                format!("This legacy backup needs the original settings: {}", issue)
            })?;
            backup.vault_metadata = Some(VaultMetadata::from_settings(&settings).ok_or_else(|| {
                "This legacy backup contains encrypted snippets but no recoverable vault metadata"
                    .to_string()
            })?);
        }

        let target_content = Self::serialize_data(&backup.nodes, backup.vault_metadata.as_ref())
            .map_err(|error| format!("Failed to prepare backup restore: {}", error))?;

        let target_settings = if let Some(metadata) = &backup.vault_metadata {
            let mut settings = match self.load_settings() {
                Ok(settings) => settings,
                Err(issue) if issue.kind == StorageIssueKind::InvalidFormat => {
                    AppSettings::default()
                }
                Err(issue) => {
                    return Err(format!(
                        "{} cannot be recovered automatically: {}",
                        issue.file_name, issue.reason
                    ));
                }
            };
            metadata.apply_to_settings(&mut settings);
            Some(settings)
        } else {
            None
        };

        if self.file_path.exists() {
            match Self::read_data_file(&self.file_path, StorageFile::Data) {
                Ok(_) => {
                    if let Err(error) = self.create_backup() {
                        self.preserve_file(&self.file_path, "sklad", "before_restore")
                            .map_err(|preserve_error| {
                                format!(
                                    "Failed to preserve current data: {}; {}",
                                    error, preserve_error
                                )
                            })?;
                    }
                }
                Err(issue) if issue.kind == StorageIssueKind::InvalidFormat => {
                    self.quarantine_file(&self.file_path, "sklad")?;
                }
                Err(issue) => {
                    return Err(format!("Failed to preserve current data: {}", issue));
                }
            }
        }

        if target_settings.is_some() {
            let settings_path = self.settings_path();
            if settings_path.exists() {
                self.preserve_file(&settings_path, "settings", "before_restore")?;
            }
        }

        Self::atomic_write(&self.file_path, &target_content)
            .map_err(|e| format!("Failed to restore backup: {}", e))?;

        if let Some(settings) = target_settings {
            self.save_settings(&settings).map_err(|error| {
                format!(
                    "Data was restored, but vault settings need recovery: {}",
                    error
                )
            })?;
        }

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
        Self::read_data_file(&self.file_path, StorageFile::Data).err()
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

    fn read_data_file(
        path: &Path,
        storage_file: StorageFile,
    ) -> Result<DataContents, StorageIssue> {
        let content =
            fs::read(path).map_err(|error| StorageIssue::unreadable(storage_file, error))?;
        let persisted: PersistedData = serde_json::from_slice(&content)
            .map_err(|error| StorageIssue::invalid(storage_file, error))?;

        match persisted {
            PersistedData::Legacy(nodes) => Ok(DataContents {
                nodes,
                vault_metadata: None,
                is_versioned: false,
            }),
            PersistedData::Versioned(versioned) => {
                if versioned.version != DATA_FILE_VERSION {
                    return Err(StorageIssue::invalid_reason(
                        storage_file,
                        format!("Unsupported data format version {}", versioned.version),
                    ));
                }
                if versioned
                    .vault_metadata
                    .as_ref()
                    .is_some_and(|metadata| !metadata.is_complete())
                {
                    return Err(StorageIssue::invalid_reason(
                        storage_file,
                        "Embedded vault metadata is incomplete",
                    ));
                }
                if Self::has_encrypted_secrets(&versioned.nodes)
                    && versioned.vault_metadata.is_none()
                {
                    return Err(StorageIssue::invalid_reason(
                        storage_file,
                        "Versioned encrypted data is missing vault metadata",
                    ));
                }

                Ok(DataContents {
                    nodes: versioned.nodes,
                    vault_metadata: versioned.vault_metadata,
                    is_versioned: true,
                })
            }
        }
    }

    fn serialize_data(
        nodes: &[Node],
        vault_metadata: Option<&VaultMetadata>,
    ) -> Result<Vec<u8>, io::Error> {
        if Self::has_encrypted_secrets(nodes) {
            let metadata = vault_metadata.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Encrypted snippets require embedded vault metadata",
                )
            })?;
            serde_json::to_vec_pretty(&VersionedData {
                version: DATA_FILE_VERSION,
                nodes: nodes.to_vec(),
                vault_metadata: Some(metadata.clone()),
            })
            .map_err(Into::into)
        } else {
            serde_json::to_vec_pretty(nodes).map_err(Into::into)
        }
    }

    fn save_data_with_metadata(
        &self,
        nodes: &[Node],
        vault_metadata: Option<&VaultMetadata>,
    ) -> Result<(), io::Error> {
        let content = Self::serialize_data(nodes, vault_metadata)?;
        Self::atomic_write(&self.file_path, &content)
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
        if settings.security.master_password_enabled
            && settings
                .security
                .password_hash
                .as_deref()
                .is_some_and(str::is_empty)
        {
            return Err(StorageIssue::invalid_reason(
                StorageFile::Settings,
                "Vault password verifier is empty",
            ));
        }
        if settings.security.master_password_enabled
            && settings
                .security
                .derivation_salt
                .as_deref()
                .is_some_and(|salt| !derivation_salt_is_valid(salt))
        {
            return Err(StorageIssue::invalid_reason(
                StorageFile::Settings,
                "Vault derivation salt is invalid",
            ));
        }

        Ok(settings)
    }

    fn read_backup(&self, filename: &str) -> Result<DataContents, String> {
        if Self::backup_timestamp(filename).is_none()
            || filename.contains('/')
            || filename.contains('\\')
        {
            return Err("Invalid backup filename".to_string());
        }

        Self::read_data_file(&self.backups_dir.join(filename), StorageFile::Data)
            .map_err(|issue| format!("Invalid backup: {}", issue.reason))
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
        self.preserve_file(source, base_name, "corrupt")
    }

    fn preserve_file(
        &self,
        source: &Path,
        base_name: &str,
        reason: &str,
    ) -> Result<String, String> {
        let content = fs::read(source)
            .map_err(|error| format!("Failed to read source file for preservation: {}", error))?;
        let mut timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let quarantine_path = loop {
            let candidate =
                source.with_file_name(format!("{}.{}_{}.json", base_name, reason, timestamp));
            if !candidate.exists() {
                break candidate;
            }
            timestamp += 1;
        };

        Self::atomic_write(&quarantine_path, &content)
            .map_err(|error| format!("Failed to create recovery copy: {}", error))?;

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

    fn remove_unrecoverable_secrets(nodes: &mut Vec<Node>) -> usize {
        let original_len = nodes.len();
        nodes.retain(|node| !node.is_secret.unwrap_or(false) && node.encrypted_value.is_none());
        let mut removed = original_len - nodes.len();

        for node in nodes {
            if let Some(children) = &mut node.children {
                removed += Self::remove_unrecoverable_secrets(children);
            }
        }

        removed
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

    const FIRST_VALID_SALT: &str = "00112233445566778899aabbccddeeff";
    const SECOND_VALID_SALT: &str = "ffeeddccbbaa99887766554433221100";

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

    fn encrypted_snippet(label: &str) -> Node {
        Node {
            id: format!("{}-secret-id", label),
            node_type: NodeType::Snippet,
            label: label.to_string(),
            parent_id: None,
            created_at: 0,
            children: None,
            value: None,
            encrypted_value: Some("nonce:ciphertext".to_string()),
            is_secret: Some(true),
        }
    }

    fn enabled_vault_settings(password_hash: &str, derivation_salt: &str) -> AppSettings {
        let mut settings = AppSettings::default();
        settings.security.master_password_enabled = true;
        settings.security.password_hash = Some(password_hash.to_string());
        settings.security.derivation_salt = Some(derivation_salt.to_string());
        settings
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

        assert!(error.starts_with("Invalid backup:"));
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

        manager.save_settings(&settings).unwrap();
        let rewritten = std::fs::read_to_string(manager.settings_path()).unwrap();
        assert!(!rewritten.contains("clearClipboard"));
    }

    #[test]
    fn older_nodes_without_optional_secret_fields_remain_compatible() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        std::fs::write(
            &manager.file_path,
            br#"[{
                "id":"legacy-id",
                "type":"snippet",
                "label":"Legacy snippet",
                "parentId":null,
                "createdAt":123,
                "value":"legacy value"
            }]"#,
        )
        .unwrap();

        let nodes = manager.load_data().unwrap();

        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].label, "Legacy snippet");
        assert_eq!(nodes[0].value.as_deref(), Some("legacy value"));
        assert!(nodes[0].encrypted_value.is_none());
        assert!(nodes[0].is_secret.is_none());
    }

    #[test]
    fn incomplete_vault_settings_require_recovery() {
        for (password_hash, derivation_salt) in [(None, Some("salt")), (Some("hash"), None)] {
            let directory = tempfile::tempdir().unwrap();
            let manager = DataManager::from_app_data_dir(directory.path().join("app"));
            let mut settings = AppSettings::default();
            settings.security.master_password_enabled = true;
            settings.security.password_hash = password_hash.map(str::to_string);
            settings.security.derivation_salt = derivation_salt.map(str::to_string);
            std::fs::write(
                manager.settings_path(),
                serde_json::to_vec_pretty(&settings).unwrap(),
            )
            .unwrap();

            let issue = manager.load_settings().unwrap_err();

            assert_eq!(issue.kind, StorageIssueKind::InvalidFormat);
            assert_eq!(issue.reason, "Vault settings are incomplete");
        }
    }

    #[test]
    fn invalid_derivation_salt_requires_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let settings = enabled_vault_settings("hash", "not-a-valid-salt");
        assert!(manager.save_settings(&settings).is_err());
        std::fs::write(
            manager.settings_path(),
            serde_json::to_vec_pretty(&settings).unwrap(),
        )
        .unwrap();

        let issue = manager.load_settings().unwrap_err();
        assert_eq!(issue.kind, StorageIssueKind::InvalidFormat);
        assert_eq!(issue.reason, "Vault derivation salt is invalid");
    }

    #[test]
    fn missing_settings_with_encrypted_data_requires_recovery_without_writing_files() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let original_data = serde_json::to_vec(&vec![encrypted_snippet("secret")]).unwrap();
        std::fs::write(&manager.file_path, &original_data).unwrap();

        let status = manager.storage_status();

        let issue = status.settings_issue.unwrap();
        assert_eq!(issue.kind, StorageIssueKind::VaultMetadata);
        assert_eq!(std::fs::read(&manager.file_path).unwrap(), original_data);
        assert!(!manager.settings_path().exists());
        assert!(manager.has_storage_issues());
    }

    #[test]
    fn missing_settings_with_an_encrypted_backup_requires_recovery_without_creating_data() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        std::fs::write(
            manager.backups_dir.join("sklad_backup_123.json"),
            serde_json::to_vec(&vec![encrypted_snippet("backup-secret")]).unwrap(),
        )
        .unwrap();

        let status = manager.storage_status();

        assert_eq!(
            status.settings_issue.unwrap().kind,
            StorageIssueKind::VaultMetadata
        );
        assert!(!manager.file_path.exists());
        assert!(!manager.settings_path().exists());
    }

    #[test]
    fn disabled_vault_with_encrypted_data_requires_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        std::fs::write(
            &manager.file_path,
            serde_json::to_vec(&vec![encrypted_snippet("disabled-secret")]).unwrap(),
        )
        .unwrap();
        manager.save_settings(&AppSettings::default()).unwrap();

        let status = manager.storage_status();

        let issue = status.settings_issue.unwrap();
        assert_eq!(issue.kind, StorageIssueKind::VaultMetadata);
        assert!(issue.reason.contains("vault is disabled"));
    }

    #[test]
    fn complete_enabled_vault_metadata_keeps_encrypted_data_healthy() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let settings = enabled_vault_settings("hash", FIRST_VALID_SALT);
        manager.save_settings(&settings).unwrap();
        manager
            .save_data(&[encrypted_snippet("healthy-secret")])
            .unwrap();

        let status = manager.storage_status();

        assert!(status.data_issue.is_none());
        assert!(status.settings_issue.is_none());
        assert!(status.has_encrypted_secrets);
        assert!(!manager.has_storage_issues());
    }

    #[test]
    fn embedded_vault_metadata_recovers_missing_settings() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let settings = enabled_vault_settings("original-hash", FIRST_VALID_SALT);
        manager.save_settings(&settings).unwrap();
        manager
            .save_data(&[encrypted_snippet("recoverable-secret")])
            .unwrap();

        let persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manager.file_path).unwrap()).unwrap();
        assert_eq!(persisted["version"], 1);
        assert_eq!(
            persisted["vaultMetadata"]["derivationSalt"],
            FIRST_VALID_SALT
        );

        std::fs::remove_file(manager.settings_path()).unwrap();
        let status = manager.storage_status();
        assert!(status.settings_issue.is_some());
        assert!(status.vault_metadata_recoverable);

        assert_eq!(manager.recover_vault_metadata().unwrap(), None);
        let recovered = manager.load_settings().unwrap();
        assert_eq!(
            recovered.security.password_hash.as_deref(),
            Some("original-hash")
        );
        assert_eq!(
            recovered.security.derivation_salt.as_deref(),
            Some(FIRST_VALID_SALT)
        );
        assert!(!manager.has_storage_issues());
    }

    #[test]
    fn legacy_encrypted_data_is_upgraded_with_recoverable_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let settings = enabled_vault_settings("legacy-hash", FIRST_VALID_SALT);
        manager.save_settings(&settings).unwrap();
        let legacy_data = serde_json::to_vec(&vec![encrypted_snippet("legacy-secret")]).unwrap();
        std::fs::write(&manager.file_path, &legacy_data).unwrap();

        assert!(manager.ensure_vault_metadata_redundancy().unwrap());
        assert!(!manager.ensure_vault_metadata_redundancy().unwrap());
        let migration_copy = std::fs::read_dir(manager.file_path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("sklad.pre_metadata_migration_")
            })
            .unwrap();
        assert_eq!(std::fs::read(migration_copy.path()).unwrap(), legacy_data);
        std::fs::remove_file(manager.settings_path()).unwrap();

        let status = manager.storage_status();
        assert!(status.vault_metadata_recoverable);
        manager.recover_vault_metadata().unwrap();
        assert_eq!(
            manager
                .load_settings()
                .unwrap()
                .security
                .derivation_salt
                .as_deref(),
            Some(FIRST_VALID_SALT)
        );
    }

    #[test]
    fn encrypted_backup_restores_its_vault_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let settings = enabled_vault_settings("backup-hash", FIRST_VALID_SALT);
        manager.save_settings(&settings).unwrap();
        std::fs::write(
            &manager.file_path,
            serde_json::to_vec(&vec![encrypted_snippet("backup-secret")]).unwrap(),
        )
        .unwrap();
        manager.create_backup().unwrap();
        let backup = manager.list_backups().remove(0);
        assert!(backup.has_vault_metadata);

        std::fs::remove_file(&manager.file_path).unwrap();
        std::fs::remove_file(manager.settings_path()).unwrap();
        let status = manager.storage_status();
        assert_eq!(
            status
                .newest_vault_backup
                .as_ref()
                .map(|item| &item.filename),
            Some(&backup.filename)
        );

        manager.restore_backup(&backup.filename).unwrap();
        assert_eq!(manager.load_data().unwrap()[0].label, "backup-secret");
        assert_eq!(
            manager
                .load_settings()
                .unwrap()
                .security
                .derivation_salt
                .as_deref(),
            Some(FIRST_VALID_SALT)
        );
        assert!(!manager.has_storage_issues());
    }

    #[test]
    fn embedded_metadata_detects_and_repairs_mismatched_settings() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        manager
            .save_settings(&enabled_vault_settings("first-hash", FIRST_VALID_SALT))
            .unwrap();
        manager
            .save_data(&[encrypted_snippet("protected")])
            .unwrap();
        manager
            .save_settings(&enabled_vault_settings("second-hash", SECOND_VALID_SALT))
            .unwrap();

        let status = manager.storage_status();
        assert_eq!(
            status.settings_issue.unwrap().kind,
            StorageIssueKind::VaultMetadata
        );
        assert!(status.vault_metadata_recoverable);

        let recovery_copy = manager.recover_vault_metadata().unwrap().unwrap();
        assert!(recovery_copy.starts_with("settings.vault_recovery_"));
        let repaired = manager.load_settings().unwrap();
        assert_eq!(
            repaired.security.password_hash.as_deref(),
            Some("first-hash")
        );
        assert_eq!(
            repaired.security.derivation_salt.as_deref(),
            Some(FIRST_VALID_SALT)
        );
    }

    #[test]
    fn vault_recovery_preserves_sources_and_removes_only_unavailable_secrets() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let mut inconsistent_secret = encrypted_snippet("inconsistent-secret");
        inconsistent_secret.is_secret = Some(false);
        let original_nodes = vec![
            snippet("public"),
            encrypted_snippet("secret"),
            inconsistent_secret,
        ];
        let original_data = serde_json::to_vec_pretty(&original_nodes).unwrap();
        std::fs::write(&manager.file_path, &original_data).unwrap();
        let original_settings = serde_json::to_vec_pretty(&AppSettings::default()).unwrap();
        std::fs::write(manager.settings_path(), &original_settings).unwrap();

        let result = manager.discard_unrecoverable_vault_data().unwrap();

        assert_eq!(result.removed_secret_count, 2);
        assert!(result.restored_from_backup.is_none());
        let data_copy = manager
            .file_path
            .with_file_name(result.data_recovery_copy.unwrap());
        let settings_copy = manager
            .file_path
            .with_file_name(result.settings_recovery_copy.unwrap());
        assert_eq!(std::fs::read(data_copy).unwrap(), original_data);
        assert_eq!(std::fs::read(settings_copy).unwrap(), original_settings);
        let recovered = manager.load_data().unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].label, "public");
        assert!(!manager.has_storage_issues());
    }

    #[test]
    fn vault_recovery_can_keep_public_data_from_an_encrypted_backup() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        let backup_filename = "sklad_backup_123.json";
        let backup_content = serde_json::to_vec(&vec![
            snippet("backup-public"),
            encrypted_snippet("backup-secret"),
        ])
        .unwrap();
        std::fs::write(manager.backups_dir.join(backup_filename), &backup_content).unwrap();

        let result = manager.discard_unrecoverable_vault_data().unwrap();

        assert_eq!(
            result.restored_from_backup.as_deref(),
            Some(backup_filename)
        );
        assert_eq!(result.removed_secret_count, 1);
        assert_eq!(manager.load_data().unwrap()[0].label, "backup-public");
        assert_eq!(
            std::fs::read(manager.backups_dir.join(backup_filename)).unwrap(),
            backup_content
        );
        assert!(!manager.has_storage_issues());
    }

    #[test]
    fn invalid_settings_with_encrypted_data_cannot_be_reset_without_vault_recovery() {
        let directory = tempfile::tempdir().unwrap();
        let manager = DataManager::from_app_data_dir(directory.path().join("app"));
        std::fs::write(
            &manager.file_path,
            serde_json::to_vec(&vec![encrypted_snippet("protected")]).unwrap(),
        )
        .unwrap();
        let invalid_settings = b"{invalid settings";
        std::fs::write(manager.settings_path(), invalid_settings).unwrap();

        let error = manager.reset_invalid_settings().unwrap_err();

        assert!(error.contains("use vault recovery"));
        assert_eq!(
            std::fs::read(manager.settings_path()).unwrap(),
            invalid_settings
        );
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
