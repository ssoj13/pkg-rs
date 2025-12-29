//! Toolset editor for creating, editing, and deleting toolsets.
//!
//! Provides a modal dialog for toolset management.

use eframe::egui::{self, Color32, RichText, Window};
use log::{debug, info, warn};
use crate::toolset::{ToolsetDef, save_toolset, delete_toolset, user_toolsets_dir};

/// Editor state for toolsets.
#[derive(Debug, Clone, Default)]
pub struct ToolsetEditorState {
    /// Is the editor window visible?
    pub visible: bool,
    /// Are we editing (true) or creating (false)?
    pub is_edit: bool,
    /// Original name (for edit mode).
    pub original_name: String,
    /// Source file path (for edit/delete).
    pub source_path: Option<String>,
    /// Toolset name.
    pub name: String,
    /// Version.
    pub version: String,
    /// Description.
    pub description: String,
    /// Requirements (one per line).
    pub requires: String,
    /// Tags (comma-separated).
    pub tags: String,
    /// Error message if any.
    pub error: Option<String>,
    /// Success message.
    pub success: Option<String>,
    /// Pending refresh (set when save/delete completes).
    pub needs_refresh: bool,
}

impl ToolsetEditorState {
    /// Open editor for new toolset.
    pub fn new_toolset(&mut self) {
        self.new_toolset_in_file(None);
    }
    
    /// Open editor for new toolset in specific file.
    pub fn new_toolset_in_file(&mut self, target_file: Option<&str>) {
        self.visible = true;
        self.is_edit = false;
        self.original_name.clear();
        self.source_path = target_file.map(|s| s.to_string());
        self.name = "my-toolset".to_string();
        self.version = "1.0.0".to_string();
        self.description.clear();
        self.requires.clear();
        self.tags.clear();
        self.error = None;
        self.success = None;
        info!("[GUI] Opening new toolset editor, target: {:?}", target_file);
    }

    /// Open editor to edit existing toolset.
    /// 
    /// # Arguments
    /// * `name` - Toolset name
    /// * `def` - Toolset definition
    /// * `source_path` - Path to the source .toml file
    pub fn edit_toolset(&mut self, name: &str, def: &ToolsetDef, source_path: Option<&str>) {
        self.visible = true;
        self.is_edit = true;
        self.original_name = name.to_string();
        self.source_path = source_path.map(|s| s.to_string());
        self.name = name.to_string();
        self.version = def.version.clone();
        self.description = def.description.clone().unwrap_or_default();
        self.requires = def.requires.join("\n");
        self.tags = def.tags.join(", ");
        self.error = None;
        self.success = None;
        info!("[GUI] Opening toolset editor for: {} from {:?}", name, source_path);
    }

    /// Build ToolsetDef from current state.
    fn to_def(&self) -> ToolsetDef {
        ToolsetDef {
            version: self.version.clone(),
            description: if self.description.is_empty() { None } else { Some(self.description.clone()) },
            requires: self.requires.lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            tags: self.tags.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        }
    }
}

/// Render the toolset editor window.
pub fn render(ctx: &egui::Context, state: &mut ToolsetEditorState) -> bool {
    // Check if refresh was requested
    if state.needs_refresh {
        state.needs_refresh = false;
        return true;
    }
    
    if !state.visible {
        return false;
    }

    let title = if state.is_edit { "Edit Toolset" } else { "New Toolset" };
    
    Window::new(title)
        .collapsible(false)
        .resizable(true)
        .default_width(400.0)
        .show(ctx, |ui| {
            // Error/success messages
            if let Some(err) = &state.error {
                ui.colored_label(Color32::RED, err);
                ui.add_space(4.0);
            }
            if let Some(msg) = &state.success {
                ui.colored_label(Color32::GREEN, msg);
                ui.add_space(4.0);
            }

            egui::Grid::new("toolset_editor_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    // Name
                    ui.label("Name:");
                    ui.add_enabled(!state.is_edit, egui::TextEdit::singleline(&mut state.name)
                        .hint_text("my-toolset"));
                    ui.end_row();

                    // Version
                    ui.label("Version:");
                    ui.add(egui::TextEdit::singleline(&mut state.version)
                        .hint_text("1.0.0"));
                    ui.end_row();

                    // Description
                    ui.label("Description:");
                    ui.add(egui::TextEdit::singleline(&mut state.description)
                        .hint_text("Optional description"));
                    ui.end_row();

                    // Tags
                    ui.label("Tags:");
                    ui.add(egui::TextEdit::singleline(&mut state.tags)
                        .hint_text("dcc, vfx"));
                    ui.end_row();
                });

            ui.add_space(8.0);
            
            // Requirements (multiline)
            ui.label(RichText::new("Requirements (one per line):").strong());
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut state.requires)
                        .hint_text("maya@2026\nredshift@>=3.5")
                        .desired_width(f32::INFINITY)
                        .desired_rows(6));
                });

            ui.add_space(12.0);
            ui.separator();

            // Buttons: [Cancel] [Save/Create] ... [Delete]
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    debug!("[GUI] Toolset editor cancelled");
                    state.visible = false;
                }

                let btn_text = if state.is_edit { "Save" } else { "Create" };
                if ui.button(btn_text).clicked() {
                    // Validate
                    if state.name.is_empty() {
                        state.error = Some("Name is required".to_string());
                    } else if state.version.is_empty() {
                        state.error = Some("Version is required".to_string());
                    } else {
                        // Determine save path
                        let save_path = if let Some(ref src) = state.source_path {
                            // Use specified source path (edit mode or new in existing file)
                            Some(std::path::PathBuf::from(src))
                        } else {
                            // New toolset without target: create {name}.toml in user dir
                            user_toolsets_dir().map(|dir| dir.join(format!("{}.toml", state.name)))
                        };
                        
                        if let Some(path) = save_path {
                            // Ensure parent dir exists
                            if let Some(parent) = path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let def = state.to_def();
                            match save_toolset(&path, &state.name, &def) {
                                Ok(_) => {
                                    info!("[GUI] Saved toolset: {} to {:?}", state.name, path);
                                    state.needs_refresh = true;
                                    state.visible = false;
                                }
                                Err(e) => {
                                    warn!("[GUI] Failed to save toolset: {}", e);
                                    state.error = Some(e);
                                }
                            }
                        } else {
                            state.error = Some("Cannot determine save path".to_string());
                        }
                    }
                }

                // Delete button on the right (edit mode only)
                if state.is_edit {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Delete").color(Color32::RED)).clicked() {
                            if let Some(ref source) = state.source_path {
                                let path = std::path::Path::new(source);
                                match delete_toolset(path, &state.original_name) {
                                    Ok(true) => {
                                        info!("[GUI] Deleted toolset: {} from {:?}", state.original_name, path);
                                        state.needs_refresh = true;
                                        state.visible = false;
                                    }
                                    Ok(false) => state.error = Some("Toolset not found".to_string()),
                                    Err(e) => {
                                        warn!("[GUI] Failed to delete: {}", e);
                                        state.error = Some(e);
                                    }
                                }
                            } else {
                                state.error = Some("No source path for this toolset".to_string());
                            }
                        }
                    });
                }
            });
        });

    false
}