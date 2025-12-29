//! Action buttons: Solve, Launch, Export.
//!
//! Bottom panel with actions for selected package.

use eframe::egui::{self, Color32, RichText, Ui};
use log::{debug, info, warn};
use crate::{Solver, Storage};
use super::state::AppState;

/// Resolved app info.
#[derive(Debug, Clone, Default)]
pub struct ResolvedApp {
    pub name: String,
    pub path: Option<String>,
    #[allow(dead_code)]
    pub from_pkg: String,
}

/// Solve result for display.
#[derive(Debug, Clone, Default)]
pub struct SolveResult {
    pub show: bool,
    pub pkg_name: String,
    pub packages: Vec<String>,
    pub apps: Vec<ResolvedApp>,
    pub env_lines: Vec<(String, String)>,
    pub error: Option<String>,
}

/// Render action buttons.
pub fn render(ui: &mut Ui, state: &mut AppState, storage: &Storage, solve_result: &mut SolveResult) {
    let has_selection = state.selection.package.is_some();
    
    ui.horizontal(|ui| {
        // Solve button
        ui.add_enabled_ui(has_selection, |ui| {
            if ui.button("Solve").clicked() {
                if let Some(pkg_name) = &state.selection.package {
                    info!("[GUI] Solve clicked: {}", pkg_name);
                    run_solve(pkg_name, storage, solve_result);
                }
            }
        });

        ui.separator();

        // Export buttons - enabled only if solve was run
        let has_env = solve_result.show && !solve_result.env_lines.is_empty();
        ui.add_enabled_ui(has_env, |ui| {
            if ui.button("Export .cmd").clicked() {
                debug!("[GUI] Export CMD clicked");
                export_env(solve_result, ExportFormat::Cmd);
            }
            if ui.button("Export .ps1").clicked() {
                debug!("[GUI] Export PS1 clicked");
                export_env(solve_result, ExportFormat::Ps1);
            }
            if ui.button("Export .sh").clicked() {
                debug!("[GUI] Export SH clicked");
                export_env(solve_result, ExportFormat::Sh);
            }
        });

        // Show selection info
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(pkg_name) = &state.selection.package {
                ui.label(RichText::new(pkg_name).color(Color32::LIGHT_BLUE));
            } else {
                ui.label(RichText::new("No selection").color(Color32::GRAY));
            }
        });
    });
}

/// Render solve result inline (no popup window).
pub fn render_solve_inline(ui: &mut Ui, state: &mut super::AppState, result: &mut SolveResult) {
    // Header with close button
    ui.horizontal(|ui| {
        ui.heading(&result.pkg_name);
        if result.error.is_none() {
            ui.label(RichText::new(format!("→ {} packages", result.packages.len()))
                .color(Color32::GREEN));
        }
        
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("×").on_hover_text("Close").clicked() {
                result.show = false;
            }
        });
    });

    if let Some(err) = &result.error {
        // Error display
        ui.colored_label(Color32::RED, "Resolution failed:");
        egui::ScrollArea::vertical()
            .max_height(150.0)
            .show(ui, |ui| {
                ui.label(err);
            });
    } else {
        // Three resizable columns
        let total_width = ui.available_width();
        let available_height = ui.available_height();
        
        // Clamp ratios
        state.solve_col1 = state.solve_col1.clamp(0.05, 0.4);
        state.solve_col2 = state.solve_col2.clamp(0.05, 0.4);
        
        let pkg_width = total_width * state.solve_col1;
        let apps_width = total_width * state.solve_col2;
        let env_width = total_width * (1.0 - state.solve_col1 - state.solve_col2 - 0.02); // 0.02 for separators
        
        ui.horizontal_top(|ui| {
            ui.set_min_height(available_height);
            
            // Left: resolved packages
            ui.vertical(|ui| {
                ui.set_width(pkg_width);
                ui.set_min_height(available_height);
                ui.strong("Packages");
                egui::ScrollArea::vertical()
                    .id_salt("solve_pkgs_inline")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (i, pkg) in result.packages.iter().enumerate() {
                            let color = if i == 0 { Color32::YELLOW } else { Color32::LIGHT_GRAY };
                            ui.label(RichText::new(format!("{}. {}", i + 1, pkg)).color(color));
                        }
                    });
            });

            // Draggable separator 1
            let sep1 = ui.separator();
            let sep1_rect = sep1.rect.expand2(egui::vec2(4.0, 0.0));
            let sep1_id = ui.id().with("sep1");
            let sep1_response = ui.interact(sep1_rect, sep1_id, egui::Sense::drag());
            if sep1_response.dragged() {
                let delta = sep1_response.drag_delta().x / total_width;
                state.solve_col1 = (state.solve_col1 + delta).clamp(0.05, 0.4);
            }
            if sep1_response.hovered() || sep1_response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }

            // Middle: apps
            ui.vertical(|ui| {
                ui.set_width(apps_width);
                ui.set_min_height(available_height);
                ui.strong("Apps");
                egui::ScrollArea::vertical()
                    .id_salt("solve_apps_inline")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if result.apps.is_empty() {
                            ui.label(RichText::new("(no apps)").color(Color32::GRAY));
                        } else {
                            for app in &result.apps {
                                ui.horizontal(|ui| {
                                    let hover = app.path.as_deref().unwrap_or("(no path)");
                                    if ui.small_button("▶").on_hover_text(format!("Launch: {}", hover)).clicked() {
                                        info!("[GUI] Launch: {} @ {:?}", app.name, app.path);
                                        launch_with_env(&app.name, app.path.as_deref(), &result.env_lines);
                                    }
                                    ui.label(RichText::new(&app.name).color(Color32::GREEN))
                                        .on_hover_text(hover);
                                });
                            }
                        }
                    });
            });

            // Draggable separator 2
            let sep2 = ui.separator();
            let sep2_rect = sep2.rect.expand2(egui::vec2(4.0, 0.0));
            let sep2_id = ui.id().with("sep2");
            let sep2_response = ui.interact(sep2_rect, sep2_id, egui::Sense::drag());
            if sep2_response.dragged() {
                let delta = sep2_response.drag_delta().x / total_width;
                state.solve_col2 = (state.solve_col2 + delta).clamp(0.05, 0.4);
            }
            if sep2_response.hovered() || sep2_response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }

            // Right: environment - tree view style
            ui.vertical(|ui| {
                ui.set_width(env_width);
                ui.set_min_height(available_height);

                // Header with Expand/Collapse buttons
                let mut expand_all = false;
                let mut collapse_all = false;

                ui.horizontal(|ui| {
                    ui.strong("Environment");
                    if ui.small_button("Expand").clicked() {
                        expand_all = true;
                    }
                    if ui.small_button("Collapse").clicked() {
                        collapse_all = true;
                    }
                });

                egui::ScrollArea::vertical()
                    .id_salt("solve_env_inline")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if result.env_lines.is_empty() {
                            ui.label(RichText::new("(no env vars)").color(Color32::GRAY));
                        } else {
                            for (name, value) in &result.env_lines {
                                let id = egui::Id::new(("env_var", name));

                                // Apply expand/collapse before showing
                                if expand_all || collapse_all {
                                    let mut cstate = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
                                    cstate.set_open(expand_all);
                                    cstate.store(ui.ctx());
                                }

                                // Use CollapsingHeader - more clickable area
                                egui::CollapsingHeader::new(RichText::new(name).color(Color32::LIGHT_BLUE))
                                    .id_salt(id)
                                    .default_open(false)
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(value).color(Color32::GRAY));
                                    });
                            }
                        }
                    });
            });
        });
    }
}

fn run_solve(pkg_name: &str, storage: &Storage, result: &mut SolveResult) {
    result.pkg_name = pkg_name.to_string();
    result.packages.clear();
    result.apps.clear();
    result.env_lines.clear();
    result.error = None;

    debug!("[GUI] Running solve for: {}", pkg_name);

    // Create solver
    let solver = match Solver::from_packages(&storage.packages()) {
        Ok(s) => s,
        Err(e) => {
            warn!("[GUI] Failed to create solver: {:?}", e);
            result.error = Some(format!("Failed to create solver: {:?}", e));
            result.show = true;
            return;
        }
    };

    // Solve
    match solver.solve_impl(pkg_name) {
        Ok(pkgs) => {
            info!("[GUI] Solved {}: {} packages", pkg_name, pkgs.len());
            result.packages = pkgs.clone();
            
            // Collect apps and env from all resolved packages
            let mut merged_env: std::collections::HashMap<String, String> = std::collections::HashMap::new();
            
            for resolved_name in &pkgs {
                if let Some(pkg) = storage.get(resolved_name) {
                    // Collect apps
                    for app in &pkg.apps {
                        result.apps.push(ResolvedApp {
                            name: app.name.clone(),
                            path: app.path.clone(),
                            from_pkg: pkg.base.clone(),
                        });
                    }
                    
                    // Merge env
                    if let Some(env) = pkg.envs.first() {
                        for evar in &env.evars {
                            merged_env.insert(evar.name.clone(), evar.value.clone());
                        }
                    }
                }
            }
            
            // Sort env
            let mut env_vec: Vec<_> = merged_env.into_iter().collect();
            env_vec.sort_by(|a, b| a.0.cmp(&b.0));
            result.env_lines = env_vec;
            
            debug!("[GUI] Resolved: {} apps, {} env vars", result.apps.len(), result.env_lines.len());
            result.show = true;
        }
        Err(e) => {
            warn!("[GUI] Solve failed for {}: {:?}", pkg_name, e);
            result.error = Some(format!("{:?}", e));
            result.show = true;
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ExportFormat {
    Cmd,
    Ps1,
    Sh,
}

/// Launch app with pre-resolved environment.
fn launch_with_env(app_name: &str, path: Option<&str>, env_lines: &[(String, String)]) {
    use std::process::Command;
    
    let Some(exe_path) = path else {
        warn!("[GUI] Cannot launch {}: no path", app_name);
        return;
    };
    
    info!("[GUI] Launching {} with {} env vars", exe_path, env_lines.len());
    
    let mut cmd = Command::new(exe_path);
    
    // Apply environment
    for (key, value) in env_lines {
        cmd.env(key, value);
    }
    
    match cmd.spawn() {
        Ok(child) => {
            info!("[GUI] Launched {} (pid: {})", app_name, child.id());
        }
        Err(e) => {
            warn!("[GUI] Failed to launch {}: {}", app_name, e);
        }
    }
}

fn export_env(result: &SolveResult, format: ExportFormat) {
    if result.env_lines.is_empty() {
        warn!("[GUI] No environment to export");
        return;
    }

    // Generate script from resolved env
    let script = match format {
        ExportFormat::Cmd => {
            let mut s = String::from("@echo off\r\n");
            for (k, v) in &result.env_lines {
                s.push_str(&format!("SET {}={}\r\n", k, v));
            }
            s
        }
        ExportFormat::Ps1 => {
            let mut s = String::new();
            for (k, v) in &result.env_lines {
                s.push_str(&format!("$env:{} = \"{}\"\n", k, v.replace('"', "`\"")));
            }
            s
        }
        ExportFormat::Sh => {
            let mut s = String::from("#!/bin/bash\n");
            for (k, v) in &result.env_lines {
                s.push_str(&format!("export {}=\"{}\"\n", k, v.replace('"', "\\\"")));
            }
            s
        }
    };

    // File extension and filter
    let (ext, filter_name) = match format {
        ExportFormat::Cmd => ("cmd", "Windows Batch"),
        ExportFormat::Ps1 => ("ps1", "PowerShell"),
        ExportFormat::Sh => ("sh", "Shell Script"),
    };

    let default_name = format!("{}.{}", result.pkg_name, ext);

    // Sync file dialog
    let file = rfd::FileDialog::new()
        .set_title("Export Environment")
        .set_file_name(&default_name)
        .add_filter(filter_name, &[ext])
        .add_filter("All files", &["*"])
        .save_file();

    if let Some(path) = file {
        match std::fs::write(&path, &script) {
            Ok(_) => info!("[GUI] Exported to {:?}", path),
            Err(e) => warn!("[GUI] Failed to export: {}", e),
        }
    } else {
        debug!("[GUI] Export cancelled");
    }
}
