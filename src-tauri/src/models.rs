use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Folder,
    Snippet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String, // UUID v4
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub label: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: i64,

    // Fields for Folder
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Node>>,

    // Fields for Snippet
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "encryptedValue", skip_serializing_if = "Option::is_none")]
    pub encrypted_value: Option<String>,
    #[serde(rename = "isSecret", skip_serializing_if = "Option::is_none")]
    pub is_secret: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettingsSecurity {
    #[serde(rename = "lockTimeout")]
    pub lock_timeout: u32,
    #[serde(rename = "clearClipboard")]
    pub clear_clipboard: bool,
    #[serde(rename = "masterPasswordEnabled")]
    pub master_password_enabled: bool,
    #[serde(rename = "passwordHash")]
    pub password_hash: Option<String>,
    #[serde(rename = "derivationSalt")]
    pub derivation_salt: Option<String>,
}

impl Default for AppSettingsSecurity {
    fn default() -> Self {
        Self {
            lock_timeout: 0,
            clear_clipboard: false,
            master_password_enabled: true,
            password_hash: None,
            derivation_salt: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String, // 'dark' | 'light' | 'system'
    pub security: AppSettingsSecurity,
    #[serde(rename = "notificationsEnabled")]
    pub notifications_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            security: AppSettingsSecurity::default(),
            notifications_enabled: true,
        }
    }
}
