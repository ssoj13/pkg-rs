//! Tree editor for package details.
//!
//! Displays package structure as collapsible tree:
//! - envs -> Env -> Evars
//! - apps -> App (with Launch button)
//! - reqs (editable for toolsets)
//! - tags

use eframe::egui::{self, Color32, RichText, Ui};
use log::{debug, info, warn};
use std::collections::HashMap;
use crate::{Solver, Storage, toolset};
use super::state::AppState;

/// Edit state for toolset requirements and tags.
#[derive(Debug, Clone, Default)]
pub struct TreeEditState {
    /// Is editing mode active?
    pub editing: bool,
    /// Package being edited (base name).
    pub pkg_base: String,
    /// Source file path.
    pub source_path: Option<String>,
    /// Editable requirements list.
    pub reqs: Vec<String>,
    /// New requirement being added.
    pub new_req: String,
    /// Editable tags (comma-separated string for simplicity).
    pub tags: String,
}

impl TreeEditState {
    /// Start editing a toolset.
    pub fn start_edit(&mut self, pkg: &crate::Package) {
        self.editing = true;
        self.pkg_base = pkg.base.clone();
        self.source_path = pkg.package_source.clone();
        self.reqs = pkg.reqs.clone();
        self.new_req.clear();
        // Tags without "toolset" (it's auto-added)
        self.tags = pkg.tags.iter()
            .filter(|t| *t != "toolset")
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        info!("[GUI] Started editing toolset: {}", pkg.base);
    }

    /// Cancel editing.
    pub fn cancel(&mut self) {
        self.editing = false;
        self.pkg_base.clear();
        self.reqs.clear();
        self.new_req.clear();
        self.tags.clear();
        debug!("[GUI] Edit cancelled");
    }

    /// Check if we're editing this package.
    pub fn is_editing(&self, pkg_base: &str) -> bool {
        self.editing && self.pkg_base == pkg_base
    }

    /// Parse tags from comma-separated string.
    pub fn parsed_tags(&self) -> Vec<String> {
        self.tags.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// Action returned from tree editor.
#[derive(Debug, Clone)]
pub enum TreeAction {
    /// Refresh storage after save.
    Refresh,
}

/// Render tree editor panel.
pub fn render(ui: &mut Ui, state: &mut AppState, storage: &Storage) -> Option<TreeAction> {
    let mut action: Option<TreeAction> = None;

    let Some(pkg_name) = &state.selection.package else {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("Select a package from the list").color(Color32::GRAY));
        });
        return None;
    };

    let Some(pkg) = storage.get(pkg_name) else {
        ui.label(RichText::new(format!("Package not found: {}", pkg_name)).color(Color32::RED));
        return None;
    };

    let is_toolset = pkg.has_tag("toolset");
    let pkg_base = pkg.base.clone();
    let pkg_version = pkg.version.clone();
    let source_path = pkg.package_source.clone();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Package header
        ui.horizontal(|ui| {
            ui.heading(&pkg.name);
            if pkg.has_tag("toolset") {
                ui.label(RichText::new("[toolset]").color(Color32::from_rgb(100, 149, 237)));
            }
        });
        
        // Version info
        ui.label(RichText::new(format!("v{}", pkg.version)).color(Color32::GRAY));
        
        ui.add_space(8.0);

        // Envs section
        let env_header = format!("envs ({})", pkg.envs.len());
        egui::CollapsingHeader::new(RichText::new(env_header).strong())
            .default_open(true)
            .show(ui, |ui| {
                if pkg.envs.is_empty() {
                    ui.label(RichText::new("(no environments)").color(Color32::GRAY));
                } else {
                    for env in &pkg.envs {
                        egui::CollapsingHeader::new(&env.name)
                            .default_open(true)
                            .show(ui, |ui| {
                                if env.evars.is_empty() {
                                    ui.label(RichText::new("(no variables)").color(Color32::GRAY));
                                } else {
                                    egui::Grid::new(format!("env_grid_{}", env.name))
                                        .striped(true)
                                        .show(ui, |ui| {
                                            for evar in &env.evars {
                                                ui.label(RichText::new(&evar.name).color(Color32::LIGHT_BLUE));
                                                ui.label("=");
                                                // Truncate long values
                                                let val = if evar.value.len() > 50 {
                                                    format!("{}...", &evar.value[..47])
                                                } else {
                                                    evar.value.clone()
                                                };
                                                ui.label(&val);
                                                ui.end_row();
                                            }
                                        });
                                }
                            });
                    }
                }
            });

        // Apps section
        let apps_header = format!("apps ({})", pkg.apps.len());
        egui::CollapsingHeader::new(RichText::new(apps_header).strong())
            .default_open(true)
            .show(ui, |ui| {
                if pkg.apps.is_empty() {
                    ui.label(RichText::new("(no applications)").color(Color32::GRAY));
                } else {
                    for app in &pkg.apps {
                        ui.horizontal(|ui| {
                            egui::CollapsingHeader::new(RichText::new(&app.name).color(Color32::GREEN))
                                .default_open(false)
                                .show(ui, |ui| {
                                    if let Some(path) = &app.path {
                                        ui.horizontal(|ui| {
                                            ui.label("path:");
                                            ui.label(RichText::new(path).color(Color32::GRAY));
                                        });
                                    }
                                    if let Some(env_name) = &app.env_name {
                                        ui.horizontal(|ui| {
                                            ui.label("env:");
                                            ui.label(env_name);
                                        });
                                    }
                                    if !app.args.is_empty() {
                                        ui.horizontal(|ui| {
                                            ui.label("args:");
                                            ui.label(app.args.join(" "));
                                        });
                                    }
                                });
                            
                            // Launch button
                            if ui.small_button("▶ Launch").clicked() {
                                info!("[GUI] Launch clicked: {} / {}", pkg.name, app.name);
                                launch_app(pkg_name, &app.name, storage);
                            }
                        });
                    }
                }
            });

        // Reqs section - editable for toolsets
        let editing = state.tree_edit.is_editing(&pkg_base);
        let reqs_count = if editing { state.tree_edit.reqs.len() } else { pkg.reqs.len() };
        let reqs_header = format!("reqs ({})", reqs_count);

        egui::CollapsingHeader::new(RichText::new(reqs_header).strong())
            .default_open(true)
            .show(ui, |ui| {
                if is_toolset && editing {
                    // Edit mode - show editable list
                    let mut to_remove: Option<usize> = None;

                    for (i, req) in state.tree_edit.reqs.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            // Delete button
                            if ui.small_button("−").clicked() {
                                to_remove = Some(i);
                            }
                            // Editable text
                            ui.text_edit_singleline(req);
                        });
                    }

                    // Remove if requested
                    if let Some(idx) = to_remove {
                        state.tree_edit.reqs.remove(idx);
                    }

                    // Add new requirement
                    ui.horizontal(|ui| {
                        if ui.small_button("+").clicked() && !state.tree_edit.new_req.is_empty() {
                            state.tree_edit.reqs.push(state.tree_edit.new_req.clone());
                            state.tree_edit.new_req.clear();
                        }
                        ui.text_edit_singleline(&mut state.tree_edit.new_req)
                            .on_hover_text("New requirement (e.g. maya@2026)");
                    });

                    ui.add_space(8.0);

                    // Cancel / Apply buttons
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            state.tree_edit.cancel();
                        }
                        if ui.button(RichText::new("Apply").color(Color32::GREEN)).clicked() {
                            // Add pending new_req if not empty
                            let new_req = state.tree_edit.new_req.trim();
                            if !new_req.is_empty() {
                                state.tree_edit.reqs.push(new_req.to_string());
                                state.tree_edit.new_req.clear();
                            }

                            // Save changes
                            if let Some(ref path) = source_path {
                                let def = toolset::ToolsetDef {
                                    version: pkg_version.clone(),
                                    description: None,
                                    requires: state.tree_edit.reqs.clone(),
                                    tags: state.tree_edit.parsed_tags(),
                                };
                                match toolset::save_toolset(std::path::Path::new(path), &pkg_base, &def) {
                                    Ok(_) => {
                                        info!("[GUI] Saved toolset: {}", pkg_base);
                                        state.tree_edit.cancel();
                                        action = Some(TreeAction::Refresh);
                                    }
                                    Err(e) => {
                                        warn!("[GUI] Failed to save: {}", e);
                                    }
                                }
                            }
                        }
                    });
                } else {
                    // View mode
                    if pkg.reqs.is_empty() {
                        ui.label(RichText::new("(no requirements)").color(Color32::GRAY));
                    } else {
                        for req in &pkg.reqs {
                            ui.horizontal(|ui| {
                                ui.label("•");
                                ui.label(RichText::new(req).color(Color32::LIGHT_BLUE));
                            });
                        }
                    }

                    // Edit button for toolsets
                    if is_toolset {
                        ui.add_space(4.0);
                        if ui.small_button("Edit").clicked() {
                            state.tree_edit.start_edit(&pkg);
                        }
                    }
                }
            });

        // Tags section - editable for toolsets in edit mode
        let tags_count = if editing { state.tree_edit.parsed_tags().len() + 1 } else { pkg.tags.len() }; // +1 for "toolset"
        let tags_header = format!("tags ({})", tags_count);
        egui::CollapsingHeader::new(RichText::new(tags_header).strong())
            .default_open(editing) // Open when editing
            .show(ui, |ui| {
                if is_toolset && editing {
                    // Edit mode - show text input
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("[toolset]").color(tag_color("toolset")));
                        ui.label(RichText::new("(auto)").color(Color32::GRAY));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Tags:");
                        ui.text_edit_singleline(&mut state.tree_edit.tags)
                            .on_hover_text("Comma-separated: dcc, render, plugin");
                    });
                } else {
                    // View mode
                    if pkg.tags.is_empty() {
                        ui.label(RichText::new("(no tags)").color(Color32::GRAY));
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for tag in &pkg.tags {
                                let color = tag_color(tag);
                                ui.label(RichText::new(format!("[{}]", tag)).color(color));
                            }
                        });
                    }
                }
            });
    });

    action
}

/// Get color for tag.
fn tag_color(tag: &str) -> Color32 {
    match tag {
        "toolset" => Color32::from_rgb(100, 149, 237),
        "dcc" => Color32::from_rgb(50, 205, 50),
        "render" | "renderer" => Color32::from_rgb(255, 140, 0),
        "plugin" | "ext" => Color32::from_rgb(186, 85, 211),
        _ => Color32::GRAY,
    }
}

/// Launch an application with resolved environment.
fn launch_app(pkg_name: &str, app_name: &str, storage: &Storage) {
    use std::process::Command;
    
    let Some(pkg) = storage.get(pkg_name) else {
        warn!("[GUI] Package not found for launch: {}", pkg_name);
        return;
    };

    let Some(app) = pkg.apps.iter().find(|a| a.name == app_name) else {
        warn!("[GUI] App not found: {} in {}", app_name, pkg_name);
        return;
    };

    let Some(path) = &app.path else {
        warn!("[GUI] App has no path: {}", app_name);
        return;
    };

    // Solve dependencies to get full environment
    let merged_env = match solve_env(pkg_name, storage) {
        Ok(env) => env,
        Err(e) => {
            warn!("[GUI] Failed to solve for launch: {}", e);
            HashMap::new()
        }
    };

    info!("[GUI] Launching {} with {} env vars", path, merged_env.len());
    debug!("[GUI] Args: {:?}", app.args);
    
    // Build command
    let mut cmd = Command::new(path);
    cmd.args(&app.args);
    
    // Apply merged environment
    for (key, value) in &merged_env {
        cmd.env(key, value);
    }
    
    // Set working directory if specified
    if let Some(cwd) = &app.cwd {
        cmd.current_dir(cwd);
    }
    
    // Spawn process
    match cmd.spawn() {
        Ok(child) => {
            info!("[GUI] Launched {} (pid: {})", app_name, child.id());
        }
        Err(e) => {
            warn!("[GUI] Failed to launch {}: {}", app_name, e);
        }
    }
}

/// Solve package and merge all environments.
fn solve_env(pkg_name: &str, storage: &Storage) -> Result<HashMap<String, String>, String> {
    // Create solver
    let solver = Solver::from_packages(&storage.packages())
        .map_err(|e| format!("Solver error: {:?}", e))?;
    
    // Solve
    let resolved = solver.solve_impl(pkg_name)
        .map_err(|e| format!("Solve failed: {:?}", e))?;
    
    debug!("[GUI] Resolved {} packages for env", resolved.len());
    
    // Merge environments from all packages (reverse order for priority)
    let mut merged: HashMap<String, String> = HashMap::new();
    
    for resolved_name in resolved.iter().rev() {
        if let Some(pkg) = storage.get(resolved_name) {
            if let Some(env) = pkg.envs.first() {
                for evar in &env.evars {
                    merged.insert(evar.name.clone(), evar.value.clone());
                }
            }
        }
    }
    
    Ok(merged)
}
