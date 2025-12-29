//! Tree editor for package details.
//!
//! Displays package structure as collapsible tree:
//! - envs -> Env -> Evars
//! - apps -> App (with Launch button)
//! - reqs
//! - tags

use eframe::egui::{self, Color32, RichText, Ui};
use log::{debug, info, warn};
use std::collections::HashMap;
use crate::{Solver, Storage};
use super::state::AppState;

/// Render tree editor panel.
pub fn render(ui: &mut Ui, state: &mut AppState, storage: &Storage) {
    let Some(pkg_name) = &state.selection.package else {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("Select a package from the list").color(Color32::GRAY));
        });
        return;
    };

    let Some(pkg) = storage.get(pkg_name) else {
        ui.label(RichText::new(format!("Package not found: {}", pkg_name)).color(Color32::RED));
        return;
    };

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

        // Reqs section
        let reqs_header = format!("reqs ({})", pkg.reqs.len());
        egui::CollapsingHeader::new(RichText::new(reqs_header).strong())
            .default_open(true)
            .show(ui, |ui| {
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
            });

        // Tags section
        let tags_header = format!("tags ({})", pkg.tags.len());
        egui::CollapsingHeader::new(RichText::new(tags_header).strong())
            .default_open(false)
            .show(ui, |ui| {
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
            });
    });
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
