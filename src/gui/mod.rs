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
mod node_layout;
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
    pub fn new(_cc: &eframe::CreationContext<'_>, storage: Storage) -> Self {
        // Load state from ~/.pkg/prefs.json
        let state = AppState::load();

        Self {
            state,
            storage,
            solve_result: SolveResult::default(),
            toolset_editor: ToolsetEditorState::default(),
        }
    }

    /// Run the GUI application.
    pub fn run(storage: Storage) -> eframe::Result<()> {
        // Load state to get window size
        let state = AppState::load();
        
        let mut viewport = egui::ViewportBuilder::default()
            .with_inner_size([state.window_width, state.window_height])
            .with_min_inner_size([800.0, 600.0]);
        
        // Restore window position if saved
        if let (Some(x), Some(y)) = (state.window_x, state.window_y) {
            viewport = viewport.with_position([x, y]);
        }
        
        let options = eframe::NativeOptions {
            viewport,
            ..Default::default()
        };

        eframe::run_native(
            "pkg-rs",
            options,
            Box::new(|cc| Ok(Box::new(PkgApp::new(cc, storage)))),
        )
    }
    
    /// Handle actions from package list.
    fn handle_list_action(&mut self, action: package_list::ListAction) {
        use package_list::ListAction;
        
        match action {
            ListAction::EditToolset(base_name) => {
                // Find package and create ToolsetDef from it
                if let Some(pkg) = self.storage.latest(&base_name) {
                    let def = toolset::ToolsetDef {
                        version: pkg.version.clone(),
                        description: None,
                        requires: pkg.reqs.clone(),
                        tags: pkg.tags.iter()
                            .filter(|t| *t != "toolset")
                            .cloned()
                            .collect(),
                    };
                    self.toolset_editor.edit_toolset(
                        &base_name,
                        &def,
                        pkg.package_source.as_deref(),
                    );
                }
            }
            ListAction::NewToolset(target_file) => {
                // Create new toolset, optionally targeting specific file
                self.toolset_editor.new_toolset(target_file.as_deref());
            }
            ListAction::DeleteToolset(pkg_name) => {
                // Find package and use its source path
                if let Some(pkg) = self.storage.get(&pkg_name) {
                    if let Some(ref source) = pkg.package_source {
                        let path = std::path::Path::new(source);
                        if let Ok(true) = toolset::delete_toolset(path, &pkg.base) {
                            self.refresh_storage();
                            self.state.selection.package = None;
                        }
                    }
                }
            }
            ListAction::NewFile => {
                // Open file dialog to create new .toml
                self.create_new_toolset_file();
            }
            ListAction::DeleteFile(file_path) => {
                // Delete entire .toml file
                let path = std::path::Path::new(&file_path);
                if path.exists() {
                    if let Ok(()) = std::fs::remove_file(path) {
                        self.refresh_storage();
                        self.state.selection.source_file = None;
                        self.state.selection.package = None;
                    }
                }
            }
        }
    }
    
    /// Refresh storage from disk.
    fn refresh_storage(&mut self) {
        if let Ok(new_storage) = Storage::scan_impl(Some(self.storage.location_paths())) {
            self.storage = new_storage;
        }
    }
    
    /// Create new .toml file for toolsets.
    fn create_new_toolset_file(&mut self) {
        // Use last directory or fallback to user toolsets dir
        let dir = self.state.last_toolset_dir
            .as_ref()
            .map(std::path::PathBuf::from)
            .filter(|p| p.exists())
            .or_else(toolset::user_toolsets_dir)
            .unwrap_or_else(|| {
                log::warn!("[GUI] Cannot determine toolsets directory");
                std::path::PathBuf::from(".")
            });
        
        // Ensure directory exists
        let _ = std::fs::create_dir_all(&dir);
        
        // Synchronous file dialog (blocks UI briefly but works reliably)
        let file = rfd::FileDialog::new()
            .set_title("Create Toolset File")
            .set_directory(&dir)
            .set_file_name("new-toolsets.toml")
            .add_filter("TOML", &["toml"])
            .save_file();
        
        if let Some(path) = file {
            // Extract name from filename (without .toml)
            let toolset_name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("new-toolset")
                .to_string();
            
            // Create dummy toolset so file appears in list
            let def = toolset::ToolsetDef {
                version: "1.0.0".to_string(),
                description: Some("New toolset".to_string()),
                requires: vec![],
                tags: vec![],
            };
            
            if let Err(e) = toolset::save_toolset(&path, &toolset_name, &def) {
                log::warn!("[GUI] Failed to create toolset file: {}", e);
                return;
            }
            
            log::info!("[GUI] Created toolset '{}' in {:?}", toolset_name, path);
            
            // Save directory for next time
            if let Some(parent) = path.parent() {
                self.state.last_toolset_dir = Some(parent.to_string_lossy().to_string());
            }
            
            // Refresh storage to pick up new file
            self.refresh_storage();
        }
    }
}

impl eframe::App for PkgApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // Save to ~/.pkg/prefs.json instead of eframe storage
        self.state.save();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Exit on Escape
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        
        // Track window size/position changes
        ctx.input(|i| {
            if let Some(rect) = i.viewport().inner_rect {
                self.state.window_width = rect.width();
                self.state.window_height = rect.height();
            }
            if let Some(pos) = i.viewport().outer_rect {
                self.state.window_x = Some(pos.min.x);
                self.state.window_y = Some(pos.min.y);
            }
        });
        // Top panel with mode selector
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.state.view_mode, ViewMode::Packages, "Packages");
                ui.selectable_value(&mut self.state.view_mode, ViewMode::Toolsets, "Toolsets");
                ui.separator();
                ui.selectable_value(&mut self.state.right_panel, state::RightPanel::Tree, "Tree");
                ui.selectable_value(&mut self.state.right_panel, state::RightPanel::Graph, "Graph");
            });
        });

        // Left panel: package list
        egui::SidePanel::left("package_list")
            .default_width(self.state.left_panel_width)
            .resizable(true)
            .show(ctx, |ui| {
                // Track panel width
                self.state.left_panel_width = ui.available_width();
                if let Some(action) = package_list::render(ui, &mut self.state, &self.storage) {
                    self.handle_list_action(action);
                }
            });

        // Central panel with vertical split: Tree/Graph on top, solve result below
        egui::CentralPanel::default().show(ctx, |ui| {
            // Calculate available height for split
            let available = ui.available_height();
            let has_solve = self.solve_result.show;
            
            // Top part: Tree/Graph view
            let top_height = if has_solve {
                (available * 0.55).max(200.0)  // 55% for view when solve shown
            } else {
                available - 40.0  // Leave space for action bar
            };
            
            let tree_action = egui::Frame::NONE.show(ui, |ui| {
                ui.set_max_height(top_height);
                match self.state.right_panel {
                    state::RightPanel::Tree => {
                        tree_editor::render(ui, &mut self.state, &self.storage)
                    }
                    state::RightPanel::Graph => {
                        node_graph::render(ui, &mut self.state, &self.storage);
                        None
                    }
                }
            }).inner;

            // Handle tree actions
            if let Some(tree_editor::TreeAction::Refresh) = tree_action {
                self.refresh_storage();
            }

            ui.separator();
            
            // Action bar
            actions::render(ui, &mut self.state, &self.storage, &mut self.solve_result);
            
            // Solve result (inline, below actions)
            if has_solve {
                ui.separator();
                actions::render_solve_inline(ui, &mut self.state, &mut self.solve_result);
            }
        });
        
        // Toolset editor window
        if toolset_editor::render(ctx, &mut self.toolset_editor) {
            // Reload storage to pick up new/edited toolset
            if let Ok(new_storage) = Storage::scan_impl(Some(self.storage.location_paths())) {
                self.storage = new_storage;
            }
        }
    }
}
