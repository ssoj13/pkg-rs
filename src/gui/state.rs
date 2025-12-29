//! GUI state management.

use serde::{Deserialize, Serialize};
use super::node_graph::NodeGraphState;

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
    /// Graph depth slider value.
    pub graph_depth: usize,
    /// Filter text for package list.
    pub filter: String,
    /// Show only toolsets in list.
    pub toolsets_only: bool,
    /// Node graph state (lazy init).
    #[serde(skip)]
    pub graph_state: Option<NodeGraphState>,
}
