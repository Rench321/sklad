use crate::models::Node;
use std::fs;
use std::path::{PathBuf};
use tauri::{AppHandle, Manager, Runtime};

pub struct DataManager {
    pub file_path: PathBuf,
}

impl DataManager {
    pub fn new<R: Runtime>(app: &AppHandle<R>) -> Self {
        // Resolve the data directory for the app
        let app_data_dir = app.path().app_data_dir().expect("failed to resolve app data dir");
        
        // Ensure the directory exists
        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
        }

        let file_path = app_data_dir.join("sklad.json");

        DataManager { file_path }
    }

    pub fn load_data(&self) -> Vec<Node> {
        if !self.file_path.exists() {
            // If file doesn't exist, try to load dummy data from where we created it in Phase 1 (project root?)
            // Or just return empty/default
            
            // For now, let's return a default "Welcome" node if no file exists
            return vec![
                Node {
                    id: "welcome-1".to_string(),
                    node_type: crate::models::NodeType::Snippet,
                    label: "Welcome to Sklad".to_string(),
                    parent_id: None,
                    created_at: 0,
                    children: None,
                    value: Some("This is your first snippet.".to_string()),
                    encrypted_value: None,
                    is_secret: Some(false),
                }
            ];
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

        let content = fs::read_to_string(&settings_path).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&content).unwrap_or_else(|_| crate::models::AppSettings::default())
    }

    pub fn save_settings(&self, settings: &crate::models::AppSettings) -> Result<(), std::io::Error> {
        let settings_path = self.file_path.with_file_name("settings.json");
        let content = serde_json::to_string_pretty(settings)?;
        fs::write(settings_path, content)
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
}
