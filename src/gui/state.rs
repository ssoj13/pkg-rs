//! GUI state management.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use log::{debug, warn};
use super::node_graph::NodeGraphState;
use super::tree_editor::TreeEditState;

fn default_graph_depth() -> usize { 4 }
fn default_h_spacing() -> f32 { 330.0 }
fn default_v_spacing() -> f32 { 80.0 }

// Solve panel column ratios (packages, apps) - env takes the rest
fn default_solve_col1() -> f32 { 0.15 }  // packages
fn default_solve_col2() -> f32 { 0.15 }  // apps

// Window defaults
fn default_window_width() -> f32 { 1200.0 }
fn default_window_height() -> f32 { 800.0 }
fn default_left_panel_width() -> f32 { 250.0 }

/// Current view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ViewMode {
    #[default]
    Packages,
    Toolsets,
}

/// Right panel mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RightPanel {
    #[default]
    Tree,
    Graph,
}

/// Current selection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Selection {
    /// Selected package name (full name like "maya-2026.1.0").
    pub package: Option<String>,
    /// Selected source file (for toolsets view).
    pub source_file: Option<String>,
    /// Expanded tree nodes (for tree view persistence).
    pub expanded: Vec<String>,
}

/// Persistent application state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    /// Current view mode (Packages or Toolsets).
    pub view_mode: ViewMode,
    /// Right panel mode (Tree or Graph).
    pub right_panel: RightPanel,
    /// Current selection.
    pub selection: Selection,
    /// Graph depth slider value (default: 4).
    #[serde(default = "default_graph_depth")]
    pub graph_depth: usize,
    /// Horizontal spacing between depth levels.
    #[serde(default = "default_h_spacing")]
    pub graph_h_spacing: f32,
    /// Vertical spacing between nodes.
    #[serde(default = "default_v_spacing")]
    pub graph_v_spacing: f32,
    /// Filter text for package list.
    pub filter: String,
    /// Show only toolsets in list.
    pub toolsets_only: bool,
    /// Node graph state (lazy init).
    #[serde(skip)]
    pub graph_state: Option<NodeGraphState>,
    /// Solve panel: packages column ratio.
    #[serde(default = "default_solve_col1")]
    pub solve_col1: f32,
    /// Solve panel: apps column ratio.
    #[serde(default = "default_solve_col2")]
    pub solve_col2: f32,
    /// Window width.
    #[serde(default = "default_window_width")]
    pub window_width: f32,
    /// Window height.
    #[serde(default = "default_window_height")]
    pub window_height: f32,
    /// Window X position.
    #[serde(default)]
    pub window_x: Option<f32>,
    /// Window Y position.
    #[serde(default)]
    pub window_y: Option<f32>,
    /// Left panel width.
    #[serde(default = "default_left_panel_width")]
    pub left_panel_width: f32,
    /// Last directory used for +File dialog.
    #[serde(default)]
    pub last_toolset_dir: Option<String>,
    /// Tree edit state (for editing toolset requirements).
    #[serde(skip)]
    pub tree_edit: TreeEditState,
}

/// Get prefs file path: ~/.pkg/prefs.json
pub fn prefs_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".pkg").join("prefs.json"))
}

impl AppState {
    /// Load state from ~/.pkg/prefs.json
    pub fn load() -> Self {
        let Some(path) = prefs_path() else {
            debug!("[GUI] Cannot determine prefs path, using defaults");
            return Self::default();
        };
        
        if !path.exists() {
            debug!("[GUI] Prefs file not found, using defaults");
            return Self::default();
        }
        
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str(&content) {
                    Ok(state) => {
                        debug!("[GUI] Loaded prefs from {:?}", path);
                        state
                    }
                    Err(e) => {
                        warn!("[GUI] Failed to parse prefs: {}", e);
                        Self::default()
                    }
                }
            }
            Err(e) => {
                warn!("[GUI] Failed to read prefs: {}", e);
                Self::default()
            }
        }
    }
    
    /// Save state to ~/.pkg/prefs.json
    pub fn save(&self) {
        let Some(path) = prefs_path() else {
            warn!("[GUI] Cannot determine prefs path");
            return;
        };
        
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    warn!("[GUI] Failed to write prefs: {}", e);
                } else {
                    debug!("[GUI] Saved prefs to {:?}", path);
                }
            }
            Err(e) => {
                warn!("[GUI] Failed to serialize prefs: {}", e);
            }
        }
    }
}
