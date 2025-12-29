//! GUI module for pkg-rs.
//!
//! Provides a graphical interface for browsing packages, editing toolsets,
//! and visualizing dependency graphs.
//!
//! Enable with `--features gui` and run with `pkg -g` / `pkg --gui`.

mod state;
mod package_list;
mod tree_editor;
mod node_graph;
mod actions;
mod toolset_editor;

pub use state::{AppState, Selection, ViewMode};
use actions::SolveResult;
use toolset_editor::ToolsetEditorState;

use eframe::egui;
use crate::{Storage, toolset};

/// Main GUI application.
pub struct PkgApp {
    state: AppState,
    storage: Storage,
    solve_result: SolveResult,
    toolset_editor: ToolsetEditorState,
}

impl PkgApp {
    /// Create new app with storage.
    pub fn new(cc: &eframe::CreationContext<'_>, storage: Storage) -> Self {
        // Load persisted state if available
        let state = cc.storage
            .and_then(|s| eframe::get_value(s, "pkg_app_state"))
            .unwrap_or_default();

        Self {
            state,
            storage,
            solve_result: SolveResult::default(),
            toolset_editor: ToolsetEditorState::default(),
        }
    }

    /// Run the GUI application.
    pub fn run(storage: Storage) -> eframe::Result<()> {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1200.0, 800.0])
                .with_min_inner_size([800.0, 600.0]),
            ..Default::default()
        };

        eframe::run_native(
            "pkg-rs",
            options,
            Box::new(|cc| Ok(Box::new(PkgApp::new(cc, storage)))),
        )
    }
}

impl eframe::App for PkgApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "pkg_app_state", &self.state);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel with mode selector
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.state.view_mode, ViewMode::Packages, "Packages");
                ui.selectable_value(&mut self.state.view_mode, ViewMode::Toolsets, "Toolsets");
                ui.separator();
                ui.selectable_value(&mut self.state.right_panel, state::RightPanel::Tree, "Tree");
                ui.selectable_value(&mut self.state.right_panel, state::RightPanel::Graph, "Graph");
                
                // New Toolset button (right side)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("+ New Toolset").clicked() {
                        self.toolset_editor.new_toolset();
                    }
                });
            });
        });

        // Left panel: package list
        egui::SidePanel::left("package_list")
            .default_width(250.0)
            .resizable(true)
            .show(ctx, |ui| {
                if let Some(action) = package_list::render(ui, &mut self.state, &self.storage) {
                    match action {
                        package_list::ListAction::EditToolset(name) => {
                            // Load toolset def and open editor
                            if let Some(dir) = toolset::user_toolsets_dir() {
                                let path = dir.join("user.toml");
                                if let Ok(defs) = toolset::parse_toolsets_file(&path) {
                                    if let Some(def) = defs.get(&name) {
                                        self.toolset_editor.edit_toolset(&name, def);
                                    }
                                }
                            }
                        }
                    }
                }
            });

        // Right panel: tree editor or graph
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state.right_panel {
                state::RightPanel::Tree => {
                    tree_editor::render(ui, &mut self.state, &self.storage);
                }
                state::RightPanel::Graph => {
                    node_graph::render(ui, &mut self.state, &self.storage);
                }
            }

            // Bottom actions
            ui.separator();
            actions::render(ui, &mut self.state, &self.storage, &mut self.solve_result);
        });

        // Solve result popup window
        actions::render_solve_window(ctx, &mut self.solve_result);
        
        // Toolset editor window
        if let Some(_name) = toolset_editor::render(ctx, &mut self.toolset_editor) {
            // Reload storage to pick up new toolset
            // TODO: Proper refresh mechanism
        }
    }
}
