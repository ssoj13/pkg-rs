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
}

/// Render package list panel. Returns action if user requested one.
pub fn render(ui: &mut Ui, state: &mut AppState, storage: &Storage) -> Option<ListAction> {
    use std::cell::RefCell;
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

        // Group by base name
        let mut bases: Vec<&str> = packages.iter()
            .map(|p| p.base.as_str())
            .collect();
        bases.sort();
        bases.dedup();

        for base in bases {
            let versions: Vec<_> = packages.iter()
                .filter(|p| p.base == base)
                .collect();

            if versions.len() == 1 {
                // Single version - show directly
                let pkg = versions[0];
                let selected = state.selection.package.as_ref() == Some(&pkg.name);
                let is_toolset = pkg.has_tag("toolset");
                let label = if is_toolset {
                    RichText::new(&pkg.name).color(Color32::from_rgb(100, 149, 237))
                } else {
                    RichText::new(&pkg.name)
                };
                
                ui.horizontal(|ui| {
                    if ui.selectable_label(selected, label).clicked() {
                        info!("[GUI] Selected package: {}", pkg.name);
                        state.selection.package = Some(pkg.name.clone());
                        if let Some(gs) = &mut state.graph_state {
                            gs.set_package(&pkg.name);
                        }
                    }
                    // Edit button for toolsets
                    if is_toolset && ui.small_button("✏").on_hover_text("Edit").clicked() {
                        *action.borrow_mut() = Some(ListAction::EditToolset(pkg.base.clone()));
                    }
                });
            } else {
                // Multiple versions - collapsible
                let header_text = if versions.iter().any(|p| p.has_tag("toolset")) {
                    RichText::new(base).color(Color32::from_rgb(100, 149, 237))
                } else {
                    RichText::new(base)
                };
                
                egui::CollapsingHeader::new(header_text)
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
    });

    action.into_inner()
}
