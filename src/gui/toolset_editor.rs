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
}

impl ToolsetEditorState {
    /// Open editor for new toolset.
    pub fn new_toolset(&mut self) {
        self.visible = true;
        self.is_edit = false;
        self.original_name.clear();
        self.name = "my-toolset".to_string();
        self.version = "1.0.0".to_string();
        self.description.clear();
        self.requires.clear();
        self.tags.clear();
        self.error = None;
        self.success = None;
        info!("[GUI] Opening new toolset editor");
    }

    /// Open editor to edit existing toolset.
    pub fn edit_toolset(&mut self, name: &str, def: &ToolsetDef) {
        self.visible = true;
        self.is_edit = true;
        self.original_name = name.to_string();
        self.name = name.to_string();
        self.version = def.version.clone();
        self.description = def.description.clone().unwrap_or_default();
        self.requires = def.requires.join("\n");
        self.tags = def.tags.join(", ");
        self.error = None;
        self.success = None;
        info!("[GUI] Opening toolset editor for: {}", name);
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
pub fn render(ctx: &egui::Context, state: &mut ToolsetEditorState) -> Option<String> {
    if !state.visible {
        return None;
    }

    let mut created_toolset: Option<String> = None;

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

            // Buttons
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    debug!("[GUI] Toolset editor cancelled");
                    state.visible = false;
                }

                if state.is_edit {
                    // Delete button (only in edit mode)
                    if ui.button(RichText::new("Delete").color(Color32::RED)).clicked() {
                        if let Some(dir) = user_toolsets_dir() {
                            let path = dir.join("user.toml");
                            match delete_toolset(&path, &state.original_name) {
                                Ok(deleted) => {
                                    if deleted {
                                        info!("[GUI] Deleted toolset: {}", state.original_name);
                                        state.success = Some("Toolset deleted!".to_string());
                                        state.visible = false;
                                    } else {
                                        state.error = Some("Toolset not found".to_string());
                                    }
                                }
                                Err(e) => {
                                    warn!("[GUI] Failed to delete toolset: {}", e);
                                    state.error = Some(e);
                                }
                            }
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let btn_text = if state.is_edit { "Save" } else { "Create" };
                    if ui.button(btn_text).clicked() {
                        // Validate
                        if state.name.is_empty() {
                            state.error = Some("Name is required".to_string());
                            return;
                        }
                        if state.version.is_empty() {
                            state.error = Some("Version is required".to_string());
                            return;
                        }

                        // Save
                        if let Some(dir) = user_toolsets_dir() {
                            let path = dir.join("user.toml");
                            let def = state.to_def();
                            
                            match save_toolset(&path, &state.name, &def) {
                                Ok(_) => {
                                    info!("[GUI] Saved toolset: {}", state.name);
                                    created_toolset = Some(state.name.clone());
                                    state.visible = false;
                                }
                                Err(e) => {
                                    warn!("[GUI] Failed to save toolset: {}", e);
                                    state.error = Some(e);
                                }
                            }
                        } else {
                            state.error = Some("Cannot determine user toolsets directory".to_string());
                        }
                    }
                });
            });
        });

    created_toolset
}
