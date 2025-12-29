//! Dependency graph visualization using egui-snarl.
//!
//! Based on Playa's node_editor implementation.
//! Each Package becomes a node, requirements become wire connections.

use std::collections::{HashMap, HashSet};

use eframe::egui::{Color32, Pos2, Ui};
use log::{debug, trace};
use egui_snarl::ui::{PinInfo, SnarlStyle, SnarlViewer};
use egui_snarl::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};
use serde::{Deserialize, Serialize};

use crate::Storage;
use super::state::AppState;

/// Node in the dependency graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageNode {
    /// Full package name (e.g., "maya-2026.1.0")
    pub name: String,
    /// Base name (e.g., "maya")
    pub base: String,
    /// Tags for coloring
    pub tags: Vec<String>,
    /// Depth in graph (0 = root)
    pub depth: usize,
    /// Is this the root/selected package?
    pub is_root: bool,
    /// Requirements (for input pins)
    pub reqs: Vec<String>,
}

/// Get node color based on package tags.
fn color_for_tags(tags: &[String], is_root: bool) -> Color32 {
    if is_root {
        Color32::from_rgb(255, 100, 100)  // Red for root
    } else if tags.iter().any(|t| t == "toolset") {
        Color32::from_rgb(100, 149, 237)  // Cornflower blue
    } else if tags.iter().any(|t| t == "render" || t == "renderer") {
        Color32::from_rgb(255, 140, 0)    // Dark orange
    } else if tags.iter().any(|t| t == "dcc") {
        Color32::from_rgb(50, 205, 50)    // Lime green
    } else if tags.iter().any(|t| t == "plugin" || t == "ext") {
        Color32::from_rgb(186, 85, 211)   // Medium orchid
    } else {
        Color32::from_rgb(169, 169, 169)  // Dark gray
    }
}

/// SnarlViewer implementation for PackageNode.
struct PackageNodeViewer;

#[allow(refining_impl_trait)]
impl SnarlViewer<PackageNode> for PackageNodeViewer {
    fn title(&mut self, node: &PackageNode) -> String {
        node.base.clone()
    }

    fn outputs(&mut self, _node: &PackageNode) -> usize {
        1  // Every package has one output (itself)
    }

    fn inputs(&mut self, node: &PackageNode) -> usize {
        node.reqs.len()  // Input per requirement
    }

    fn show_input(
        &mut self,
        pin: &InPin,
        ui: &mut Ui,
        snarl: &mut Snarl<PackageNode>,
    ) -> PinInfo {
        let node = &snarl[pin.id.node];
        let req_name = node.reqs.get(pin.id.input)
            .map(|r| r.split('@').next().unwrap_or(r))
            .unwrap_or("?");
        ui.label(req_name);
        PinInfo::circle().with_fill(Color32::from_rgb(100, 180, 255))
    }

    fn show_output(
        &mut self,
        _pin: &OutPin,
        ui: &mut Ui,
        _snarl: &mut Snarl<PackageNode>,
    ) -> PinInfo {
        ui.label("out");
        PinInfo::circle().with_fill(Color32::from_rgb(180, 180, 180))
    }

    fn has_body(&mut self, _node: &PackageNode) -> bool {
        false
    }

    fn show_body(
        &mut self,
        _node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        _ui: &mut Ui,
        _snarl: &mut Snarl<PackageNode>,
    ) {
    }

    fn show_header(
        &mut self,
        node_id: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<PackageNode>,
    ) {
        let node = &snarl[node_id];
        let color = color_for_tags(&node.tags, node.is_root);

        let icon = if node.is_root {
            "[ROOT]"
        } else if node.tags.iter().any(|t| t == "toolset") {
            "[T]"
        } else if node.tags.iter().any(|t| t == "dcc") {
            "[D]"
        } else if node.tags.iter().any(|t| t == "render" || t == "renderer") {
            "[R]"
        } else {
            "[P]"
        };

        ui.horizontal(|ui| {
            ui.colored_label(color, icon);
            ui.label(&node.name);
        });
    }
}

/// Persistent state for node graph.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeGraphState {
    /// The egui-snarl graph.
    #[serde(skip)]
    pub snarl: Snarl<PackageNode>,

    /// Currently displayed package.
    #[serde(skip)]
    pub current_pkg: Option<String>,

    /// Needs rebuild flag.
    #[serde(skip)]
    needs_rebuild: bool,

    /// Fit all requested.
    #[serde(skip)]
    fit_requested: bool,

    /// Layout requested.
    #[serde(skip)]
    layout_requested: bool,

    /// Counter to force viewport reset.
    #[serde(skip)]
    viewport_counter: u64,
}

impl NodeGraphState {
    pub fn new() -> Self {
        Self {
            snarl: Snarl::new(),
            current_pkg: None,
            needs_rebuild: true,
            fit_requested: false,
            layout_requested: false,
            viewport_counter: 0,
        }
    }

    /// Set package to display.
    pub fn set_package(&mut self, pkg_name: &str) {
        if self.current_pkg.as_deref() != Some(pkg_name) {
            self.current_pkg = Some(pkg_name.to_string());
            self.needs_rebuild = true;
        }
    }

    /// Rebuild graph from storage.
    pub fn rebuild(&mut self, storage: &Storage, max_depth: usize, h_spacing: f32, v_spacing: f32) {
        if !self.needs_rebuild {
            return;
        }
        self.needs_rebuild = false;
        self.snarl = Snarl::new();
        
        debug!("[GUI] Rebuilding graph for {:?}, depth={}", self.current_pkg, max_depth);

        let Some(root_name) = &self.current_pkg else { return };
        let Some(root_pkg) = storage.get(root_name) else { return };

        // Collect nodes via BFS
        let mut node_info: HashMap<String, (PackageNode, usize)> = HashMap::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue = vec![(root_pkg.name.clone(), 0usize)];
        let mut actual_max_depth = 0;

        while let Some((pkg_name, depth)) = queue.pop() {
            if visited.contains(&pkg_name) {
                continue;
            }
            visited.insert(pkg_name.clone());
            actual_max_depth = actual_max_depth.max(depth);

            let Some(pkg) = storage.get(&pkg_name).or_else(|| storage.latest(&pkg_name)) else {
                continue;
            };

            let node = PackageNode {
                name: pkg.name.clone(),
                base: pkg.base.clone(),
                tags: pkg.tags.clone(),
                depth,
                is_root: depth == 0,
                reqs: pkg.reqs.clone(),
            };
            node_info.insert(pkg.name.clone(), (node, depth));

            // Queue children (only if we haven't reached max depth)
            if depth < max_depth {
                for req in &pkg.reqs {
                    let req_base = req.split('@').next().unwrap_or(req);
                    if let Some(resolved) = storage.latest(req_base) {
                        if !visited.contains(&resolved.name) {
                            queue.push((resolved.name.clone(), depth + 1));
                        }
                    }
                }
            }
        }

        // Use hierarchical layout algorithm
        use super::node_layout::{LayoutNode, LayoutEdge, LayoutConfig, layout_graph};
        
        // Prepare layout nodes
        let layout_nodes: Vec<LayoutNode> = node_info.iter().map(|(name, (node, _))| {
            LayoutNode {
                id: name.clone(),
                layer: node.depth,
                width: 150.0,
                height: 30.0 + node.reqs.len() as f32 * 20.0,
            }
        }).collect();
        
        // Prepare layout edges (dependency -> dependent)
        let mut layout_edges: Vec<LayoutEdge> = Vec::new();
        for (parent_name, (node, _)) in &node_info {
            for req in &node.reqs {
                let req_base = req.split('@').next().unwrap_or(req);
                if let Some(resolved) = storage.latest(req_base) {
                    if node_info.contains_key(&resolved.name) {
                        layout_edges.push(LayoutEdge {
                            from: resolved.name.clone(),
                            to: parent_name.clone(),
                        });
                    }
                }
            }
        }
        
        // Run layout algorithm
        let config = LayoutConfig {
            h_spacing,
            v_spacing,
            node_sep: 20.0,
        };
        let layout_result = layout_graph(layout_nodes, layout_edges, config);
        
        // Create nodes with calculated positions
        let mut name_to_node: HashMap<String, NodeId> = HashMap::new();
        
        for (name, (node, _)) in &node_info {
            let (x, y) = layout_result.positions.get(name).copied().unwrap_or((50.0, 50.0));
            let pos = Pos2::new(x, y);
            
            let node_id = self.snarl.insert_node(pos, node.clone());
            name_to_node.insert(name.clone(), node_id);
        }

        // Create wires (child output -> parent input)
        for (parent_name, (node, _)) in &node_info {
            if let Some(&parent_id) = name_to_node.get(parent_name) {
                for (input_idx, req) in node.reqs.iter().enumerate() {
                    let req_base = req.split('@').next().unwrap_or(req);
                    // Find resolved package
                    if let Some(resolved) = storage.latest(req_base) {
                        if let Some(&child_id) = name_to_node.get(&resolved.name) {
                            let out_pin = OutPinId { node: child_id, output: 0 };
                            let in_pin = InPinId { node: parent_id, input: input_idx };
                            let _ = self.snarl.connect(out_pin, in_pin);
                        }
                    }
                }
            }
        }

        self.fit_requested = true;
    }
}

/// Render dependency graph.
pub fn render(ui: &mut Ui, state: &mut AppState, storage: &Storage) {
    let Some(pkg_name) = &state.selection.package else {
        ui.centered_and_justified(|ui| {
            ui.label("Select a package to view its dependency graph");
        });
        return;
    };

    if storage.get(pkg_name).is_none() {
        ui.label(format!("Package not found: {}", pkg_name));
        return;
    };

    // Lazy init graph state
    let graph_state = state.graph_state.get_or_insert_with(NodeGraphState::new);

    // Sync package
    graph_state.set_package(pkg_name);

    // Rebuild if needed
    if graph_state.needs_rebuild {
        graph_state.rebuild(storage, state.graph_depth, state.graph_h_spacing, state.graph_v_spacing);
    }

    // Toolbar
    ui.horizontal(|ui| {
        ui.label("Depth:");
        if ui.add(eframe::egui::Slider::new(&mut state.graph_depth, 0..=10)).changed() {
            graph_state.needs_rebuild = true;
        }

        ui.separator();
        
        ui.label("H:");
        if ui.add(eframe::egui::Slider::new(&mut state.graph_h_spacing, 150.0..=500.0)).changed() {
            graph_state.needs_rebuild = true;
        }
        
        ui.label("V:");
        if ui.add(eframe::egui::Slider::new(&mut state.graph_v_spacing, 10.0..=100.0)).changed() {
            graph_state.needs_rebuild = true;
        }

        ui.separator();

        // A - fit All
        if ui.button("A").on_hover_text("Fit All - zoom to see all nodes").clicked() {
            trace!("[GUI] Graph: Fit All clicked");
            graph_state.fit_requested = true;
        }

        // L - Layout
        if ui.button("L").on_hover_text("Layout - re-arrange nodes").clicked() {
            debug!("[GUI] Graph: Layout clicked");
            graph_state.layout_requested = true;
        }

        ui.separator();

        let node_count = graph_state.snarl.node_ids().count();
        ui.label(format!("{} nodes", node_count));

        ui.separator();

        // Legend
        legend_item(ui, "Root", Color32::from_rgb(255, 100, 100));
        legend_item(ui, "Toolset", Color32::from_rgb(100, 149, 237));
        legend_item(ui, "DCC", Color32::from_rgb(50, 205, 50));
        legend_item(ui, "Render", Color32::from_rgb(255, 140, 0));
    });

    ui.separator();

    // Handle fit request
    if graph_state.fit_requested {
        graph_state.fit_requested = false;
        graph_state.viewport_counter += 1;
    }

    // Handle layout request
    if graph_state.layout_requested {
        graph_state.layout_requested = false;
        graph_state.needs_rebuild = true;
    }

    // Render snarl
    let mut viewer = PackageNodeViewer;
    let style = SnarlStyle {
        centering: Some(true),
        ..Default::default()
    };

    let snarl_id = format!("pkg_node_graph_{}", graph_state.viewport_counter);
    graph_state.snarl.show(&mut viewer, &style, &snarl_id, ui);
}

/// Draw a legend item.
fn legend_item(ui: &mut Ui, label: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(eframe::egui::Vec2::new(12.0, 12.0), eframe::egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
    ui.label(label);
}
