use crate::models::{Node, NodeType};
use tauri::{
    menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    AppHandle, Runtime,
};

use crate::data_manager::DataManager;

pub struct TrayGenerator;

impl TrayGenerator {
    pub fn generate_menu<R: Runtime>(app: &AppHandle<R>, nodes: &[Node]) -> tauri::Result<Menu<R>> {
        log::info!(
            "Generating tray menu. Received {} top-level nodes.",
            nodes.len()
        );
        let mut menu_builder = MenuBuilder::new(app);

        let data_manager = DataManager::new(app);
        let settings = data_manager.load_settings();

        let is_top = settings.tray_menu_root_position == "top";

        let quit_item = MenuItemBuilder::new("Quit Sklad").id("quit").build(app)?;
        let open_item = MenuItemBuilder::new("Open Sklad").id("open").build(app)?;

        if is_top {
            menu_builder = menu_builder.item(&quit_item);
            menu_builder = menu_builder.item(&open_item);
            menu_builder = menu_builder.separator();
        }

        for node in nodes {
            match node.node_type {
                NodeType::Folder => {
                    let submenu = Self::generate_submenu(app, node)?;
                    menu_builder = menu_builder.item(&submenu);
                }
                NodeType::Snippet => {
                    let text = if node.is_secret.unwrap_or(false) {
                        format!("🔒 {}", node.label)
                    } else {
                        node.label.clone()
                    };
                    let item = MenuItemBuilder::new(text).id(&node.id).build(app)?;
                    menu_builder = menu_builder.item(&item);
                }
            }
        }

        if !is_top {
            menu_builder = menu_builder.separator();
            menu_builder = menu_builder.item(&open_item);
            menu_builder = menu_builder.item(&quit_item);
        }

        log::info!("Tray menu generated successfully. Compiling into a native menu object.");
        menu_builder.build()
    }

    fn generate_submenu<R: Runtime>(
        app: &AppHandle<R>,
        node: &Node,
    ) -> tauri::Result<tauri::menu::Submenu<R>> {
        log::info!("Generating submenu for folder ID: {}", node.id);
        let mut submenu_builder = SubmenuBuilder::new(app, &node.label);

        if let Some(children) = &node.children {
            for child in children {
                match child.node_type {
                    NodeType::Folder => {
                        let sub = Self::generate_submenu(app, child)?;
                        submenu_builder = submenu_builder.item(&sub);
                    }
                    NodeType::Snippet => {
                        let text = if child.is_secret.unwrap_or(false) {
                            format!("🔒 {}", child.label)
                        } else {
                            child.label.clone()
                        };
                        let item = MenuItemBuilder::new(text).id(&child.id).build(app)?;
                        submenu_builder = submenu_builder.item(&item);
                    }
                }
            }
        }

        submenu_builder.build()
    }
}
