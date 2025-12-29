//! Node layout algorithms for dependency graphs.
//!
//! Provides multiple layout algorithms:
//! - `simple`: Basic hierarchical with barycenter ordering
//! - `brandes_kopf`: Brandes-Köpf algorithm for aligned node placement
//!
//! Reference: Brandes & Köpf, "Fast and Simple Horizontal Coordinate Assignment" (GD 2001)

use std::collections::HashMap;

//=============================================================================
// Data Structures
//=============================================================================

/// Node info for layout calculation.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LayoutNode {
    pub id: String,
    pub layer: usize,
    pub width: f32,
    pub height: f32,
}

/// Edge info for layout.
#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub from: String,
    pub to: String,
}

/// Result of layout calculation.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub positions: HashMap<String, (f32, f32)>,
}

/// Layout configuration.
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub h_spacing: f32,
    pub v_spacing: f32,
    pub node_sep: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            h_spacing: 330.0,
            v_spacing: 80.0,
            node_sep: 20.0,
        }
    }
}

/// Layout algorithm choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutAlgorithm {
    /// Simple barycenter-based layout.
    #[default]
    Simple,
    /// Brandes-Köpf aligned layout.
    BrandesKopf,
}

//=============================================================================
// Internal Graph Representation
//=============================================================================

/// Internal layered graph for layout algorithms.
struct LayeredGraph {
    /// Nodes in each layer, ordered.
    layers: Vec<Vec<String>>,
    /// Node data by ID.
    nodes: HashMap<String, LayoutNode>,
    /// Node position within layer.
    pos: HashMap<String, usize>,
    /// Upper neighbors (nodes in layer-1 that connect to this node).
    upper: HashMap<String, Vec<String>>,
    /// Lower neighbors (nodes in layer+1 that connect to this node).
    lower: HashMap<String, Vec<String>>,
}

impl LayeredGraph {
    fn new(nodes: Vec<LayoutNode>, edges: &[LayoutEdge]) -> Self {
        let mut layers: HashMap<usize, Vec<String>> = HashMap::new();
        let mut node_map: HashMap<String, LayoutNode> = HashMap::new();
        let mut max_layer = 0;

        for node in nodes {
            max_layer = max_layer.max(node.layer);
            layers.entry(node.layer).or_default().push(node.id.clone());
            node_map.insert(node.id.clone(), node);
        }

        // Convert to vec of layers
        let mut layer_vec: Vec<Vec<String>> = Vec::with_capacity(max_layer + 1);
        for i in 0..=max_layer {
            let mut layer = layers.remove(&i).unwrap_or_default();
            layer.sort(); // Initial alphabetical order
            layer_vec.push(layer);
        }

        // Build position map
        let mut pos = HashMap::new();
        for layer in &layer_vec {
            for (i, id) in layer.iter().enumerate() {
                pos.insert(id.clone(), i);
            }
        }

        // Build neighbor maps
        // Edge: from -> to means "from" is dependency of "to"
        // So "from" is in a higher layer (further from root)
        let mut upper: HashMap<String, Vec<String>> = HashMap::new();
        let mut lower: HashMap<String, Vec<String>> = HashMap::new();

        for edge in edges {
            let from_layer = node_map.get(&edge.from).map(|n| n.layer).unwrap_or(0);
            let to_layer = node_map.get(&edge.to).map(|n| n.layer).unwrap_or(0);

            if from_layer > to_layer {
                // from is lower (dependency), to is upper (dependent)
                upper.entry(edge.from.clone()).or_default().push(edge.to.clone());
                lower.entry(edge.to.clone()).or_default().push(edge.from.clone());
            } else if from_layer < to_layer {
                // from is upper, to is lower
                lower.entry(edge.from.clone()).or_default().push(edge.to.clone());
                upper.entry(edge.to.clone()).or_default().push(edge.from.clone());
            }
        }

        Self {
            layers: layer_vec,
            nodes: node_map,
            pos,
            upper,
            lower,
        }
    }

    fn layer_count(&self) -> usize {
        self.layers.len()
    }

    fn layer(&self, i: usize) -> &[String] {
        self.layers.get(i).map(|v| v.as_slice()).unwrap_or(&[])
    }

    fn layer_mut(&mut self, i: usize) -> Option<&mut Vec<String>> {
        self.layers.get_mut(i)
    }

    fn node_layer(&self, id: &str) -> usize {
        self.nodes.get(id).map(|n| n.layer).unwrap_or(0)
    }

    fn node_pos(&self, id: &str) -> usize {
        self.pos.get(id).copied().unwrap_or(0)
    }

    fn node_width(&self, id: &str) -> f32 {
        self.nodes.get(id).map(|n| n.width).unwrap_or(100.0)
    }

    /// Update position cache after reordering.
    fn update_positions(&mut self) {
        self.pos.clear();
        for layer in &self.layers {
            for (i, id) in layer.iter().enumerate() {
                self.pos.insert(id.clone(), i);
            }
        }
    }

    /// Get upper neighbors sorted by position.
    fn upper_neighbors(&self, id: &str) -> Vec<String> {
        let mut neighbors = self.upper.get(id).cloned().unwrap_or_default();
        neighbors.sort_by_key(|n| self.node_pos(n));
        neighbors
    }

    /// Get lower neighbors sorted by position.
    fn lower_neighbors(&self, id: &str) -> Vec<String> {
        let mut neighbors = self.lower.get(id).cloned().unwrap_or_default();
        neighbors.sort_by_key(|n| self.node_pos(n));
        neighbors
    }
}

//=============================================================================
// Public API
//=============================================================================

/// Main entry point for layout calculation.
pub fn layout_graph(
    nodes: Vec<LayoutNode>,
    edges: Vec<LayoutEdge>,
    config: LayoutConfig,
) -> LayoutResult {
    layout_graph_with_algorithm(nodes, edges, config, LayoutAlgorithm::BrandesKopf)
}

/// Layout with specific algorithm.
pub fn layout_graph_with_algorithm(
    nodes: Vec<LayoutNode>,
    edges: Vec<LayoutEdge>,
    config: LayoutConfig,
    algorithm: LayoutAlgorithm,
) -> LayoutResult {
    if nodes.is_empty() {
        return LayoutResult { positions: HashMap::new() };
    }

    let mut graph = LayeredGraph::new(nodes, &edges);
    
    // Crossing minimization (shared by all algorithms)
    barycenter_ordering(&mut graph, 8);

    match algorithm {
        LayoutAlgorithm::Simple => simple_positioning(&graph, &config),
        LayoutAlgorithm::BrandesKopf => brandes_kopf_positioning(&graph, &config),
    }
}

//=============================================================================
// Crossing Minimization (Barycenter)
//=============================================================================

fn barycenter_ordering(graph: &mut LayeredGraph, passes: usize) {
    for _ in 0..passes {
        // Forward sweep
        for i in 1..graph.layer_count() {
            order_layer_by_barycenter(graph, i, true);
        }
        // Backward sweep
        for i in (0..graph.layer_count().saturating_sub(1)).rev() {
            order_layer_by_barycenter(graph, i, false);
        }
    }
    graph.update_positions();
}

fn order_layer_by_barycenter(graph: &mut LayeredGraph, layer_idx: usize, use_upper: bool) {
    let layer = graph.layer(layer_idx).to_vec();
    
    let mut barycenters: Vec<(String, f32)> = layer.iter().map(|id| {
        let neighbors = if use_upper {
            graph.upper_neighbors(id)
        } else {
            graph.lower_neighbors(id)
        };

        let bc = if neighbors.is_empty() {
            graph.node_pos(id) as f32
        } else {
            let sum: f32 = neighbors.iter().map(|n| graph.node_pos(n) as f32).sum();
            sum / neighbors.len() as f32
        };

        (id.clone(), bc)
    }).collect();

    barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    if let Some(layer_mut) = graph.layer_mut(layer_idx) {
        *layer_mut = barycenters.into_iter().map(|(id, _)| id).collect();
    }
    
    // Update positions for this layer
    if let Some(layer) = graph.layers.get(layer_idx) {
        for (i, id) in layer.iter().enumerate() {
            graph.pos.insert(id.clone(), i);
        }
    }
}

//=============================================================================
// Simple Positioning (Grid-based)
//=============================================================================

fn simple_positioning(graph: &LayeredGraph, config: &LayoutConfig) -> LayoutResult {
    let mut positions = HashMap::new();
    let max_layer = graph.layer_count().saturating_sub(1);

    for layer_idx in 0..graph.layer_count() {
        let layer = graph.layer(layer_idx);
        let count = layer.len();
        let total_height = (count.saturating_sub(1)) as f32 * config.v_spacing;
        let start_y = -total_height / 2.0 + 300.0;

        for (slot, id) in layer.iter().enumerate() {
            let x = (max_layer - layer_idx) as f32 * config.h_spacing + 100.0;
            let y = start_y + slot as f32 * config.v_spacing;
            positions.insert(id.clone(), (x, y));
        }
    }

    LayoutResult { positions }
}

//=============================================================================
// Brandes-Köpf Algorithm
//=============================================================================

/// Brandes-Köpf horizontal coordinate assignment.
/// Runs 4 passes with different alignment directions and averages results.
fn brandes_kopf_positioning(graph: &LayeredGraph, config: &LayoutConfig) -> LayoutResult {
    // Run 4 variants
    let x1 = bk_pass(graph, config, true, true);   // left, upper
    let x2 = bk_pass(graph, config, true, false);  // left, lower
    let x3 = bk_pass(graph, config, false, true);  // right, upper
    let x4 = bk_pass(graph, config, false, false); // right, lower

    // Balance: take average of 4 results
    let mut positions = HashMap::new();
    let max_layer = graph.layer_count().saturating_sub(1);

    for (id, node) in &graph.nodes {
        let coords: Vec<f32> = [&x1, &x2, &x3, &x4]
            .iter()
            .filter_map(|x| x.get(id).copied())
            .collect();

        let y = if coords.is_empty() {
            graph.node_pos(id) as f32 * config.v_spacing + 300.0
        } else {
            // Use median for more stable result
            let mut sorted = coords.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mid = sorted.len() / 2;
            if sorted.len() % 2 == 0 {
                (sorted[mid - 1] + sorted[mid]) / 2.0
            } else {
                sorted[mid]
            }
        };

        let x = (max_layer - node.layer) as f32 * config.h_spacing + 100.0;
        positions.insert(id.clone(), (x, y));
    }

    LayoutResult { positions }
}

/// Single pass of Brandes-Köpf algorithm.
/// Returns Y-coordinates for each node.
fn bk_pass(
    graph: &LayeredGraph,
    config: &LayoutConfig,
    left_to_right: bool,
    align_upper: bool,
) -> HashMap<String, f32> {
    let mut root: HashMap<String, String> = HashMap::new();
    let mut align: HashMap<String, String> = HashMap::new();
    let mut x: HashMap<String, f32> = HashMap::new();

    // Initialize: each node is its own root
    for id in graph.nodes.keys() {
        root.insert(id.clone(), id.clone());
        align.insert(id.clone(), id.clone());
    }

    // Phase 1: Vertical alignment
    bk_vertical_alignment(graph, &mut root, &mut align, left_to_right, align_upper);

    // Phase 2: Horizontal compaction (vertical in our rotated view)
    bk_horizontal_compaction(graph, config, &root, &mut x, left_to_right);

    x
}

/// Vertical alignment phase: group nodes into blocks.
fn bk_vertical_alignment(
    graph: &LayeredGraph,
    root: &mut HashMap<String, String>,
    align: &mut HashMap<String, String>,
    left_to_right: bool,
    align_upper: bool,
) {
    let layer_order: Vec<usize> = if align_upper {
        (1..graph.layer_count()).collect()
    } else {
        (0..graph.layer_count().saturating_sub(1)).rev().collect()
    };

    for layer_idx in layer_order {
        let layer = graph.layer(layer_idx).to_vec();
        let mut r: i32 = if left_to_right { -1 } else { i32::MAX };

        let node_order: Vec<usize> = if left_to_right {
            (0..layer.len()).collect()
        } else {
            (0..layer.len()).rev().collect()
        };

        for k in node_order {
            let v = &layer[k];
            let neighbors = if align_upper {
                graph.upper_neighbors(v)
            } else {
                graph.lower_neighbors(v)
            };

            if neighbors.is_empty() {
                continue;
            }

            // Get median neighbor(s)
            let medians = get_medians(&neighbors);

            for m in medians {
                if align.get(v) == Some(v) {
                    let m_pos = graph.node_pos(&m) as i32;
                    
                    let no_conflict = if left_to_right {
                        m_pos > r
                    } else {
                        m_pos < r
                    };

                    if no_conflict {
                        align.insert(m.clone(), v.clone());
                        root.insert(v.clone(), root.get(&m).cloned().unwrap_or(m.clone()));
                        align.insert(v.clone(), root.get(v).cloned().unwrap_or(v.clone()));
                        r = m_pos;
                    }
                }
            }
        }
    }
}

/// Get median element(s) from sorted list.
fn get_medians(neighbors: &[String]) -> Vec<String> {
    if neighbors.is_empty() {
        return vec![];
    }
    let len = neighbors.len();
    let mid = len / 2;
    if len % 2 == 1 {
        vec![neighbors[mid].clone()]
    } else {
        vec![neighbors[mid - 1].clone(), neighbors[mid].clone()]
    }
}

/// Horizontal compaction phase: assign coordinates to blocks.
fn bk_horizontal_compaction(
    graph: &LayeredGraph,
    config: &LayoutConfig,
    root: &HashMap<String, String>,
    x: &mut HashMap<String, f32>,
    left_to_right: bool,
) {
    // Initialize all positions
    for id in graph.nodes.keys() {
        x.insert(id.clone(), f32::NEG_INFINITY);
    }

    // Process layers
    for layer_idx in 0..graph.layer_count() {
        let layer = graph.layer(layer_idx);
        
        let order: Vec<usize> = if left_to_right {
            (0..layer.len()).collect()
        } else {
            (0..layer.len()).rev().collect()
        };

        for k in order {
            let v = &layer[k];
            let v_root = root.get(v).cloned().unwrap_or(v.clone());

            if x.get(&v_root).copied().unwrap_or(f32::NEG_INFINITY) == f32::NEG_INFINITY {
                place_block(graph, config, root, x, &v_root, left_to_right);
            }
        }
    }

    // Assign coordinates to all nodes from their roots
    for layer in &graph.layers {
        for v in layer {
            let v_root = root.get(v).cloned().unwrap_or(v.clone());
            let root_x = x.get(&v_root).copied().unwrap_or(0.0);
            x.insert(v.clone(), root_x);
        }
    }
}

/// Place a block (root and all aligned nodes).
fn place_block(
    graph: &LayeredGraph,
    config: &LayoutConfig,
    root: &HashMap<String, String>,
    x: &mut HashMap<String, f32>,
    v: &str,
    left_to_right: bool,
) {
    if x.get(v).copied().unwrap_or(f32::NEG_INFINITY) != f32::NEG_INFINITY {
        return;
    }

    x.insert(v.to_string(), 0.0);

    // Find minimum position based on predecessors in same layer
    let current = v.to_string();
    let mut min_sep = 0.0_f32;

    loop {
        let layer_idx = graph.node_layer(&current);
        let pos = graph.node_pos(&current);
        let layer = graph.layer(layer_idx);

        // Get predecessor in layer
        let pred_pos = if left_to_right && pos > 0 {
            Some(pos - 1)
        } else if !left_to_right && pos + 1 < layer.len() {
            Some(pos + 1)
        } else {
            None
        };

        if let Some(p_pos) = pred_pos {
            if let Some(pred) = layer.get(p_pos) {
                let pred_root = root.get(pred).cloned().unwrap_or(pred.clone());
                place_block(graph, config, root, x, &pred_root, left_to_right);

                let pred_x = x.get(&pred_root).copied().unwrap_or(0.0);
                let sep = graph.node_width(pred) / 2.0 + config.node_sep + graph.node_width(&current) / 2.0;
                
                let required = if left_to_right {
                    pred_x + sep
                } else {
                    pred_x - sep
                };

                min_sep = if left_to_right {
                    min_sep.max(required)
                } else {
                    if min_sep == 0.0 { required } else { min_sep.min(required) }
                };
            }
        }

        // Move to next in alignment chain
        // For simplicity, we just process the root node
        break;
    }

    x.insert(v.to_string(), min_sep);
}

//=============================================================================
// Tests
//=============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_layout() {
        let nodes = vec![
            LayoutNode { id: "A".into(), layer: 0, width: 100.0, height: 50.0 },
            LayoutNode { id: "B".into(), layer: 1, width: 100.0, height: 50.0 },
            LayoutNode { id: "C".into(), layer: 1, width: 100.0, height: 50.0 },
        ];
        let edges = vec![
            LayoutEdge { from: "B".into(), to: "A".into() },
            LayoutEdge { from: "C".into(), to: "A".into() },
        ];

        let result = layout_graph_with_algorithm(
            nodes, edges, LayoutConfig::default(), LayoutAlgorithm::Simple
        );
        assert_eq!(result.positions.len(), 3);
    }

    #[test]
    fn test_brandes_kopf_layout() {
        let nodes = vec![
            LayoutNode { id: "A".into(), layer: 0, width: 100.0, height: 50.0 },
            LayoutNode { id: "B".into(), layer: 1, width: 100.0, height: 50.0 },
            LayoutNode { id: "C".into(), layer: 1, width: 100.0, height: 50.0 },
            LayoutNode { id: "D".into(), layer: 2, width: 100.0, height: 50.0 },
        ];
        let edges = vec![
            LayoutEdge { from: "B".into(), to: "A".into() },
            LayoutEdge { from: "C".into(), to: "A".into() },
            LayoutEdge { from: "D".into(), to: "B".into() },
        ];

        let result = layout_graph_with_algorithm(
            nodes, edges, LayoutConfig::default(), LayoutAlgorithm::BrandesKopf
        );
        assert_eq!(result.positions.len(), 4);
        
        // D should be closer to B than to C (vertically aligned)
        let d_pos = result.positions.get("D").unwrap();
        let b_pos = result.positions.get("B").unwrap();
        let c_pos = result.positions.get("C").unwrap();
        
        let d_to_b = (d_pos.1 - b_pos.1).abs();
        let d_to_c = (d_pos.1 - c_pos.1).abs();
        assert!(d_to_b <= d_to_c, "D should be closer to B: d_to_b={}, d_to_c={}", d_to_b, d_to_c);
    }
}
