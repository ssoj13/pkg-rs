//! Action buttons: Solve, Launch, Export.
//!
//! Bottom panel with actions for selected package.

use eframe::egui::{self, Color32, RichText, Ui, Window};
use log::{debug, info, trace, warn};
use crate::{Solver, Storage};
use super::state::AppState;

/// Resolved app info.
#[derive(Debug, Clone, Default)]
pub struct ResolvedApp {
    pub name: String,
    pub path: Option<String>,
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

        // Export dropdown
        ui.add_enabled_ui(has_selection, |ui| {
            egui::ComboBox::from_label("Export")
                .selected_text("...")
                .show_ui(ui, |ui| {
                    if ui.selectable_label(false, "Windows (.cmd)").clicked() {
                        debug!("[GUI] Export CMD clicked");
                        export_env(state, storage, ExportFormat::Cmd);
                    }
                    if ui.selectable_label(false, "PowerShell (.ps1)").clicked() {
                        debug!("[GUI] Export PS1 clicked");
                        export_env(state, storage, ExportFormat::Ps1);
                    }
                    if ui.selectable_label(false, "Bash (.sh)").clicked() {
                        debug!("[GUI] Export SH clicked");
                        export_env(state, storage, ExportFormat::Sh);
                    }
                    if ui.selectable_label(false, "Python (.py)").clicked() {
                        debug!("[GUI] Export PY clicked");
                        export_env(state, storage, ExportFormat::Py);
                    }
                });
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

/// Render solve result window.
pub fn render_solve_window(ctx: &egui::Context, result: &mut SolveResult) {
    if !result.show {
        return;
    }

    Window::new("Resolved Environment")
        .collapsible(true)
        .resizable(true)
        .default_width(700.0)
        .default_height(500.0)
        .show(ctx, |ui| {
            // Header
            ui.horizontal(|ui| {
                ui.heading(&result.pkg_name);
                if result.error.is_none() {
                    ui.label(RichText::new(format!("→ {} packages", result.packages.len()))
                        .color(Color32::GREEN));
                }
            });
            ui.separator();

            if let Some(err) = &result.error {
                // Error display
                ui.colored_label(Color32::RED, "Resolution failed:");
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.label(err);
                    });
            } else {
                // Three columns layout
                ui.columns(3, |cols| {
                    // Left: resolved packages
                    cols[0].heading("Packages");
                    cols[0].separator();
                    egui::ScrollArea::vertical()
                        .id_salt("solve_pkgs")
                        .max_height(350.0)
                        .show(&mut cols[0], |ui| {
                            for (i, pkg) in result.packages.iter().enumerate() {
                                let color = if i == 0 { Color32::YELLOW } else { Color32::LIGHT_GRAY };
                                ui.label(RichText::new(format!("{}. {}", i + 1, pkg)).color(color));
                            }
                        });

                    // Middle: apps from all packages
                    cols[1].heading("Apps");
                    cols[1].separator();
                    egui::ScrollArea::vertical()
                        .id_salt("solve_apps")
                        .max_height(350.0)
                        .show(&mut cols[1], |ui| {
                            if result.apps.is_empty() {
                                ui.label(RichText::new("(no apps)").color(Color32::GRAY));
                            } else {
                                for app in &result.apps {
                                    ui.horizontal(|ui| {
                                        let hover = app.path.as_deref().unwrap_or("(no path)");
                                        if ui.small_button("▶").on_hover_text(format!("Launch: {}", hover)).clicked() {
                                            info!("[GUI] Launch from solve: {} @ {:?}", app.name, app.path);
                                            launch_with_env(&app.name, app.path.as_deref(), &result.env_lines);
                                        }
                                        ui.label(RichText::new(&app.name).color(Color32::GREEN))
                                            .on_hover_text(hover);
                                    });
                                    ui.label(RichText::new(format!("  from {}", app.from_pkg))
                                        .color(Color32::DARK_GRAY));
                                }
                            }
                        });

                    // Right: merged environment
                    cols[2].heading("Environment");
                    cols[2].separator();
                    egui::ScrollArea::vertical()
                        .id_salt("solve_env")
                        .max_height(350.0)
                        .show(&mut cols[2], |ui| {
                            if result.env_lines.is_empty() {
                                ui.label(RichText::new("(no env vars)").color(Color32::GRAY));
                            } else {
                                for (name, value) in &result.env_lines {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(name).color(Color32::LIGHT_BLUE).strong());
                                    });
                                    // Value truncated
                                    let display_val = if value.len() > 40 {
                                        format!("{}...", &value[..37])
                                    } else {
                                        value.clone()
                                    };
                                    ui.label(RichText::new(format!("  {}", display_val)).color(Color32::GRAY));
                                }
                            }
                        });
                });
            }

            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    trace!("[GUI] Solve window closed");
                    result.show = false;
                }
                
                if result.error.is_none() && !result.env_lines.is_empty() {
                    if ui.button("Copy Env").clicked() {
                        let text: String = result.env_lines.iter()
                            .map(|(k, v)| format!("{}={}", k, v))
                            .collect::<Vec<_>>()
                            .join("\n");
                        ui.ctx().copy_text(text);
                        info!("[GUI] Environment copied to clipboard");
                    }
                    
                    if ui.button("Copy as CMD").clicked() {
                        let text: String = result.env_lines.iter()
                            .map(|(k, v)| format!("SET {}={}", k, v))
                            .collect::<Vec<_>>()
                            .join("\r\n");
                        ui.ctx().copy_text(text);
                        info!("[GUI] CMD script copied");
                    }
                }
            });
        });
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
    Py,
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

fn export_env(state: &AppState, storage: &Storage, format: ExportFormat) {
    let Some(pkg_name) = &state.selection.package else {
        return;
    };

    let Some(pkg) = storage.get(pkg_name) else {
        warn!("[GUI] Package not found for export: {}", pkg_name);
        return;
    };

    // Get first env
    let Some(env) = pkg.envs.first() else {
        warn!("[GUI] No environment defined for {}", pkg_name);
        return;
    };

    // Generate script
    let script = match format {
        ExportFormat::Cmd => env.to_cmd(),
        ExportFormat::Ps1 => env.to_ps1(),
        ExportFormat::Sh => env.to_sh(),
        ExportFormat::Py => env.to_py(),
    };

    // File extension and filter
    let (ext, filter_name) = match format {
        ExportFormat::Cmd => ("cmd", "Windows Batch"),
        ExportFormat::Ps1 => ("ps1", "PowerShell"),
        ExportFormat::Sh => ("sh", "Shell Script"),
        ExportFormat::Py => ("py", "Python"),
    };
    
    let default_name = format!("{}_{}.{}", pkg.base, env.name, ext);
    
    // Open save dialog in thread to avoid blocking UI
    let script_clone = script.clone();
    std::thread::spawn(move || {
        let file = rfd::FileDialog::new()
            .set_title("Export Environment")
            .set_file_name(&default_name)
            .add_filter(filter_name, &[ext])
            .add_filter("All files", &["*"])
            .save_file();
        
        if let Some(path) = file {
            match std::fs::write(&path, &script_clone) {
                Ok(_) => info!("[GUI] Exported to {:?}", path),
                Err(e) => warn!("[GUI] Failed to export: {}", e),
            }
        } else {
            debug!("[GUI] Export cancelled");
        }
    });
}
