//! Package list panel (left side).
//!
//! Shows filterable list of packages/toolsets with version grouping.

use eframe::egui::{self, Color32, RichText, Ui};
use log::{info, trace};
use crate::Storage;
use super::state::{AppState, ViewMode};

/// Action returned from package list.
#[derive(Debug, Clone)]
pub enum ListAction {
    /// Edit selected toolset.
    EditToolset(String),
    /// Create new toolset in specified file.
    NewToolset(Option<String>),
    /// Delete selected toolset.
    DeleteToolset(String),
    /// Create new .toml file.
    NewFile,
    /// Delete .toml file (with all toolsets).
    DeleteFile(String),
}

use std::cell::RefCell;
use std::path::Path;
use crate::Package;

/// Render packages grouped by base name.
fn render_packages(
    ui: &mut Ui,
    state: &mut AppState,
    packages: &[&Package],
    _action: &RefCell<Option<ListAction>>,
) {
    // Group by base name
    let mut bases: Vec<&str> = packages.iter()
        .map(|p| p.base.as_str())
        .collect();
    bases.sort();
    bases.dedup();

    for base in bases {
        let versions: Vec<_> = packages.iter()
            .filter(|p| p.base == base)
            .copied()
            .collect();

        if versions.len() == 1 {
            let pkg = versions[0];
            let selected = state.selection.package.as_ref() == Some(&pkg.name);
            if ui.selectable_label(selected, &pkg.name).clicked() {
                info!("[GUI] Selected package: {}", pkg.name);
                state.selection.package = Some(pkg.name.clone());
                if let Some(gs) = &mut state.graph_state {
                    gs.set_package(&pkg.name);
                }
            }
        } else {
            egui::CollapsingHeader::new(base)
                .default_open(false)
                .show(ui, |ui| {
                    for pkg in versions {
                        let selected = state.selection.package.as_ref() == Some(&pkg.name);
                        if ui.selectable_label(selected, &pkg.version).clicked() {
                            info!("[GUI] Selected package: {}", pkg.name);
                            state.selection.package = Some(pkg.name.clone());
                            if let Some(gs) = &mut state.graph_state {
                                gs.set_package(&pkg.name);
                            }
                        }
                    }
                });
        }
    }
}

/// Render toolsets grouped by source file.
fn render_toolsets(
    ui: &mut Ui,
    state: &mut AppState,
    packages: &[&Package],
    action: &RefCell<Option<ListAction>>,
) {
    use std::collections::BTreeMap;
    
    // Group by source file
    let mut by_source: BTreeMap<String, Vec<&Package>> = BTreeMap::new();
    
    for pkg in packages {
        let source = pkg.package_source.clone().unwrap_or_else(|| "(unknown)".to_string());
        by_source.entry(source).or_default().push(pkg);
    }
    
    for (source, toolsets) in &by_source {
        // Extract filename from path
        let filename = Path::new(source)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| source.clone());
        
        let file_selected = state.selection.source_file.as_ref() == Some(source);
        
        // File header with buttons
        ui.horizontal(|ui| {
            // Clickable file name to select
            let header_color = if file_selected {
                Color32::from_rgb(255, 200, 100)
            } else {
                Color32::from_rgb(180, 180, 180)
            };
            if ui.selectable_label(file_selected, RichText::new(&filename).color(header_color)).clicked() {
                state.selection.source_file = Some(source.clone());
            }
            
            // Add toolset to this file
            if ui.small_button("+").on_hover_text("Add toolset to this file").clicked() {
                *action.borrow_mut() = Some(ListAction::NewToolset(Some(source.clone())));
            }
            
            // Delete file (only if not "(unknown)")
            if source != "(unknown)" {
                if ui.small_button("×").on_hover_text("Delete this file").clicked() {
                    *action.borrow_mut() = Some(ListAction::DeleteFile(source.clone()));
                }
            }
        });
        
        // Toolsets in this file (indented)
        ui.indent(source, |ui| {
            for pkg in toolsets {
                let selected = state.selection.package.as_ref() == Some(&pkg.name);
                let label = RichText::new(&pkg.base).color(Color32::from_rgb(100, 149, 237));
                
                ui.horizontal(|ui| {
                    if ui.selectable_label(selected, label).clicked() {
                        info!("[GUI] Selected toolset: {}", pkg.name);
                        state.selection.package = Some(pkg.name.clone());
                        state.selection.source_file = Some(source.clone());
                        if let Some(gs) = &mut state.graph_state {
                            gs.set_package(&pkg.name);
                        }
                    }
                    // Edit button
                    if ui.small_button("✏").on_hover_text("Edit").clicked() {
                        *action.borrow_mut() = Some(ListAction::EditToolset(pkg.base.clone()));
                    }
                    // Delete toolset
                    if ui.small_button("−").on_hover_text("Delete toolset").clicked() {
                        *action.borrow_mut() = Some(ListAction::DeleteToolset(pkg.name.clone()));
                    }
                });
            }
        });
        
        ui.add_space(4.0);
    }
}

/// Render package list panel. Returns action if user requested one.
pub fn render(ui: &mut Ui, state: &mut AppState, storage: &Storage) -> Option<ListAction> {
    let action: RefCell<Option<ListAction>> = RefCell::new(None);
    // Header with count
    let (title, count) = match state.view_mode {
        ViewMode::Packages => {
            let c = storage.packages_iter().filter(|p| !p.has_tag("toolset")).count();
            ("Packages", c)
        }
        ViewMode::Toolsets => {
            let c = storage.packages_iter().filter(|p| p.has_tag("toolset")).count();
            ("Toolsets", c)
        }
    };
    
    ui.horizontal(|ui| {
        ui.heading(title);
        ui.label(RichText::new(format!("({})", count)).color(Color32::GRAY));
    });

    // Filter input
    ui.horizontal(|ui| {
        ui.label("Filter:");
        let filter_changed = ui.text_edit_singleline(&mut state.filter).changed();
        if filter_changed {
            trace!("[GUI] Filter changed: '{}'", state.filter);
        }
    });

    ui.separator();

    // Package list
    egui::ScrollArea::vertical().show(ui, |ui| {
        let filter_lower = state.filter.to_lowercase();

        // Get packages, optionally filtered
        let packages: Vec<_> = storage.packages_iter()
            .filter(|pkg| {
                // Filter by view mode
                let is_toolset = pkg.has_tag("toolset");
                match state.view_mode {
                    ViewMode::Packages => !is_toolset,
                    ViewMode::Toolsets => is_toolset,
                }
            })
            .filter(|pkg| {
                // Filter by search text
                filter_lower.is_empty() || pkg.name.to_lowercase().contains(&filter_lower)
            })
            .collect();

        if packages.is_empty() {
            ui.label(RichText::new("(no matches)").color(Color32::GRAY));
            return;
        }

        match state.view_mode {
            ViewMode::Packages => render_packages(ui, state, &packages, &action),
            ViewMode::Toolsets => render_toolsets(ui, state, &packages, &action),
        }
    });

    // Bottom buttons for Toolsets mode
    if state.view_mode == ViewMode::Toolsets {
        ui.separator();
        ui.horizontal(|ui| {
            // New file button
            if ui.button("+ File").on_hover_text("Create new .toml file").clicked() {
                *action.borrow_mut() = Some(ListAction::NewFile);
            }
            // New toolset (uses selected file or creates default)
            if ui.button("+ Toolset").on_hover_text("Add toolset to selected file").clicked() {
                *action.borrow_mut() = Some(ListAction::NewToolset(state.selection.source_file.clone()));
            }
        });
    }

    action.into_inner()
}
