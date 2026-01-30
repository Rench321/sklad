use crate::models::{Node, NodeType};
use tauri::{
    menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    AppHandle, Runtime,
};

pub struct TrayGenerator;

impl TrayGenerator {
    pub fn generate_menu<R: Runtime>(app: &AppHandle<R>, nodes: &[Node]) -> tauri::Result<Menu<R>> {
        let mut menu_builder = MenuBuilder::new(app);

        // Add a Quit item at the top
        let quit_item = MenuItemBuilder::new("Quit Sklad").id("quit").build(app)?;
        menu_builder = menu_builder.item(&quit_item);
        
        let open_item = MenuItemBuilder::new("Open Sklad").id("open").build(app)?;
        menu_builder = menu_builder.item(&open_item);
        
        menu_builder = menu_builder.separator();

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
                    let item = MenuItemBuilder::new(text)
                        .id(&node.id)
                        .build(app)?;
                    menu_builder = menu_builder.item(&item);
                }
            }
        }

        menu_builder.build()
    }

    fn generate_submenu<R: Runtime>(app: &AppHandle<R>, node: &Node) -> tauri::Result<tauri::menu::Submenu<R>> {
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
                        let item = MenuItemBuilder::new(text)
                            .id(&child.id)
                            .build(app)?;
                        submenu_builder = submenu_builder.item(&item);
                    }
                }
            }
        }

        submenu_builder.build()
    }
}
